// @vitest-environment jsdom
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
  within,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { App } from "./app";
import { defaultPreferences, type AppPreferences, type Status } from "./model";

const rpc = vi.hoisted(() => vi.fn());
const events = vi.hoisted(
  () => new Map<string, (event: { payload: string }) => void>(),
);
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => true, invoke: rpc }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: async (
    name: string,
    listener: (event: { payload: string }) => void,
  ) => {
    events.set(name, listener);
    return () => events.delete(name);
  },
}));
const enrolled: Status = {
  endpoint: "https://insights.example.com",
  grafana_url: "https://grafana.example.com",
  collection_state: "active",
  collection_enabled: true,
  consent_status: "active",
  pending_event_count: 0,
};
let current: Status;
let preferences: AppPreferences;
beforeEach(() => {
  current = { ...enrolled };
  preferences = { ...defaultPreferences };
  localStorage.clear();
  vi.stubGlobal("scrollTo", vi.fn());
  vi.stubGlobal("matchMedia", () => ({
    matches: false,
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
  }));
  vi.stubGlobal("requestAnimationFrame", (cb: () => void) => {
    cb();
    return 1;
  });
  HTMLElement.prototype.scrollIntoView = vi.fn();
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  rpc.mockReset();
  rpc.mockImplementation(async (command: string, args: any) => {
    if (command === "get_app_preferences") return { ...preferences };
    if (command === "save_app_preferences") {
      preferences = { ...args.preferences };
      return preferences;
    }
    if (command === "snapshot") return { ...current };
    if (command === "check_connection") return { ticket: "fixture-ticket" };
    if (command === "set_collection") {
      if (args.action === "disable")
        current = {
          ...current,
          collection_enabled: false,
          collection_state: "disabled",
          consent_status: "revoked",
        };
      return { uploaded_count: 0 };
    }
    if (command === "resume_collection")
      current = {
        ...current,
        collection_enabled: true,
        consent_status: "active",
        collection_state: "awaiting_first_collection",
      };
    if (command === "export_compose")
      return { directory: "/tmp/GroundLine-fixture" };
    if (command === "save_dashboard")
      current = { ...current, grafana_url: args.url };
    return {};
  });
});
afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
async function mount() {
  render(<App />);
  await waitFor(() =>
    expect(
      (screen.getByRole("button", { name: "개요" }) as HTMLButtonElement)
        .disabled,
    ).toBe(false),
  );
}
function nav(label: string) {
  fireEvent.click(
    within(screen.getByRole("navigation", { name: "주 메뉴" })).getByRole(
      "button",
      { name: label },
    ),
  );
}

async function choose(name: string, option: string) {
  fireEvent.keyDown(screen.getByRole("combobox", { name }), {
    key: "ArrowDown",
  });
  fireEvent.click(await screen.findByRole("option", { name: option }));
}

describe("메뉴에서 끝내는 사용자 작업", () => {
  it("창 닫기는 트레이가 기본이며 저장 성공 뒤에만 종료 설정을 반영한다", async () => {
    await mount();
    nav("설정");
    const closeAction = screen.getByRole("combobox", { name: "창 닫기" });
    expect(closeAction.textContent).toContain("트레이에 숨기기");
    await choose("창 닫기", "앱 종료");
    await screen.findByText("앱 실행 설정을 저장했습니다.");
    expect(closeAction.textContent).toContain("앱 종료");
    expect(preferences.close_action).toBe("quit");
    rpc.mockImplementationOnce(async () => {
      throw "local_state_failed";
    });
    await choose("창 닫기", "트레이에 숨기기");
    await screen.findByRole("alert");
    expect(closeAction.textContent).toContain("앱 종료");
    expect(
      rpc.mock.calls.some(([name]) =>
        ["set_collection", "resume_collection", "connect"].includes(name),
      ),
    ).toBe(false);
  });
  it("수집 완료를 수신으로 오인하지 않고 다시 열면 실제 확인 기록을 갱신한다", async () => {
    current = { ...enrolled, last_success_utc: "2026-09-10T00:00:00Z" };
    await mount();
    const delivery = screen.getByRole("region", { name: "서버 수신 확인" });
    expect(
      within(delivery).getByText(/서버의 수신 확인 기록이 생기면 표시됩니다/),
    ).toBeTruthy();
    current = {
      ...current,
      delivery_confirmation: {
        confirmed_at_utc: "2026-09-10T00:05:00Z",
        event_count: 3,
      },
      pending_event_count: 2,
    };
    fireEvent.focus(window);
    await within(delivery).findByText("3건");
    expect(within(delivery).getByText("2건")).toBeTruthy();
    expect(
      rpc.mock.calls.every(([name]) =>
        ["snapshot", "get_app_preferences"].includes(name),
      ),
    ).toBe(true);
  });
  it("개요를 기본으로 열고 설정된 연결은 등록 폼 대신 관리 화면을 보여준다", async () => {
    await mount();
    expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("개요");
    expect(localStorage.getItem("groundline-theme")).toBe("system");
    expect(
      within(screen.getByRole("navigation", { name: "주 메뉴" }))
        .getAllByRole("button")
        .map((button) => button.textContent),
    ).toEqual(["개요", "서버", "설정"]);
    nav("서버");
    expect(screen.getByRole("region", { name: "연결 관리" })).toBeTruthy();
    expect(screen.queryByPlaceholderText("등록키 입력")).toBeNull();
    expect(
      rpc.mock.calls.every(([name]) =>
        ["snapshot", "get_app_preferences"].includes(name),
      ),
    ).toBe(true);
    fireEvent.click(
      screen.getByRole("button", { name: "지금 수집·전송 확인" }),
    );
    await screen.findByText(/이번 실행에서 서버가 수신한 이벤트는 없습니다/);
    expect(screen.queryByText(/서버가 0건을 수신했습니다/)).toBeNull();
  });
  it("중지 후 동의 체크가 있어야 기존 연결의 수집을 재개한다", async () => {
    await mount();
    nav("설정");
    fireEvent.click(screen.getByRole("button", { name: "수집 중지" }));
    await screen.findByText(/자동 수집을 중지했습니다/);
    fireEvent.click(screen.getByRole("button", { name: "동의 후 다시 시작" }));
    const resume = screen.getByRole("button", {
      name: "동의하고 수집 재개",
    }) as HTMLButtonElement;
    expect(resume.disabled).toBe(true);
    expect(rpc.mock.calls.some(([name]) => name === "resume_collection")).toBe(
      false,
    );
    fireEvent.click(screen.getByRole("checkbox"));
    fireEvent.click(resume);
    await screen.findByText(/수집 동의를 저장하고 자동 수집을 다시 켰습니다/);
    expect(rpc).toHaveBeenCalledWith("resume_collection", {
      runtime: "codex_app",
      consent: true,
    });
  });
  it.each(["화면", "트레이"])(
    "연결 동의 중 %s 메뉴를 바꾸면 네이티브 임시 등록키도 해제한다",
    async (source) => {
      current = { collection_enabled: false, collection_state: "disabled" };
      await mount();
      nav("서버");
      fireEvent.change(
        screen.getByRole("textbox", { name: /^Insights 서버 주소/ }),
        { target: { value: "https://insights.example.com" } },
      );
      fireEvent.change(screen.getByPlaceholderText("등록키 입력"), {
        target: { value: "x".repeat(32) },
      });
      fireEvent.click(screen.getByRole("button", { name: "연결 확인" }));
      await screen.findByRole("button", { name: "동의하고 연결" });
      expect(
        (
          screen.getByRole("button", {
            name: "동의하고 연결",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(true);
      if (source === "트레이")
        events.get("groundline:navigate")?.({ payload: "settings" });
      else nav("설정");
      await screen.findByRole("heading", { level: 1, name: "설정" });
      expect(rpc).toHaveBeenCalledWith("cancel_connection");
      nav("서버");
      expect(
        (screen.getByPlaceholderText("등록키 입력") as HTMLInputElement).value,
      ).toBe("");
      expect(rpc.mock.calls.some(([name]) => name === "connect")).toBe(false);
    },
  );
  it("소모된 연결 티켓의 실패는 재확인 폼으로 복구한다", async () => {
    await mount();
    nav("서버");
    fireEvent.click(screen.getByText("연결 진단과 등록키 관리"));
    fireEvent.click(screen.getByRole("button", { name: "등록키 다시 확인" }));
    expect(
      (
        screen.getByRole("textbox", {
          name: /^Insights 서버 주소/,
        }) as HTMLInputElement
      ).readOnly,
    ).toBe(true);
    fireEvent.change(screen.getByPlaceholderText("등록키 입력"), {
      target: { value: "x".repeat(32) },
    });
    fireEvent.click(screen.getByRole("button", { name: "연결 확인" }));
    await screen.findByRole("button", { name: "동의하고 연결" });
    fireEvent.click(screen.getByRole("checkbox"));
    rpc.mockImplementationOnce(async () => {
      throw "connection_check_required";
    });
    fireEvent.click(screen.getByRole("button", { name: "동의하고 연결" }));
    await screen.findByRole("alert");
    expect(screen.getByPlaceholderText("등록키 입력")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "동의하고 연결" })).toBeNull();
  });
  it("환경 전환 후 해당 환경만 수집 중지한다", async () => {
    await mount();
    nav("서버");
    current = { ...enrolled, endpoint: "https://cli.example.com" };
    await choose("대상 환경", "Codex CLI");
    await screen.findByText("https://cli.example.com");
    expect(screen.queryByText("https://insights.example.com")).toBeNull();
    nav("설정");
    fireEvent.click(screen.getByRole("button", { name: "수집 중지" }));
    await screen.findByText(/자동 수집을 중지했습니다/);
    expect(rpc).toHaveBeenCalledWith("set_collection", {
      runtime: "codex_cli",
      action: "disable",
    });
  });
  it("Grafana 주소를 고쳐도 수집을 재등록하거나 전송하지 않는다", async () => {
    await mount();
    nav("서버");
    fireEvent.click(screen.getByRole("button", { name: "주소 수정" }));
    fireEvent.change(screen.getByLabelText(/대시보드 주소/), {
      target: { value: "https://dashboard.example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "주소 저장" }));
    await screen.findByText("대시보드 주소를 저장했습니다.");
    expect(screen.getByText("https://dashboard.example.com")).toBeTruthy();
    expect(
      rpc.mock.calls.every(([name]) =>
        ["snapshot", "get_app_preferences", "save_dashboard"].includes(name),
      ),
    ).toBe(true);
  });
  it("서버 필수값을 단계별 검사하고 검토 이후에만 생성하며 비밀값을 요약에 표시하지 않는다", async () => {
    current = { collection_enabled: false, collection_state: "disabled" };
    await mount();
    nav("서버");
    fireEvent.click(screen.getByRole("button", { name: /새 서버 구성/ }));
    fireEvent.click(screen.getByRole("button", { name: "다음 단계" }));
    await screen.findByRole("alert");
    expect(rpc.mock.calls.some(([name]) => name === "export_compose")).toBe(
      false,
    );
    fireEvent.change(
      screen.getByLabelText("Insights API 주소", { exact: true }),
      { target: { value: "https://insights.example.com" } },
    );
    fireEvent.change(screen.getByLabelText("Grafana 주소", { exact: true }), {
      target: { value: "https://grafana.example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: "다음 단계" }));
    expect(screen.getByRole("heading", { name: "배포 이미지" })).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "다음 단계" }));
    await screen.findByText(/SHA-256 digest/);
    fireEvent.change(
      screen.getByPlaceholderText(
        "ghcr.io/jukqaz/groundline-insights-api@sha256:…",
      ),
      {
        target: {
          value:
            "ghcr.io/jukqaz/groundline-insights-api@sha256:" + "a".repeat(64),
        },
      },
    );
    fireEvent.change(screen.getByLabelText("비밀번호", { exact: true }), {
      target: { value: "PRIVATE_TEST_SECRET".repeat(2) },
    });
    fireEvent.click(screen.getByRole("button", { name: "다음 단계" }));
    expect(
      screen.getByRole("heading", { name: "생성 전 최종 확인" }),
    ).toBeTruthy();
    expect(screen.queryByText("PRIVATE_TEST_SECRET".repeat(2))).toBeNull();
    fireEvent.click(
      screen.getByRole("button", { name: "Compose 파일 만들기" }),
    );
    await screen.findByRole("button", { name: "생성 폴더 열기" });
    fireEvent.click(screen.getByRole("button", { name: "생성 폴더 열기" }));
    await waitFor(() => expect(rpc).toHaveBeenCalledWith("reveal_export"));
    await waitFor(() =>
      expect(
        (
          screen.getByRole("button", {
            name: "이 주소로 기기 연결",
          }) as HTMLButtonElement
        ).disabled,
      ).toBe(false),
    );
    fireEvent.click(
      screen.getByRole("button", { name: "이 주소로 기기 연결" }),
    );
    expect(
      (
        screen.getByRole("textbox", {
          name: /^Insights 서버 주소/,
        }) as HTMLInputElement
      ).value,
    ).toBe("https://insights.example.com");
    expect(
      (screen.getByPlaceholderText("등록키 입력") as HTMLInputElement).value,
    ).toBe("");
    expect(
      rpc.mock.calls.some(([name]) =>
        ["connect", "check_connection", "set_collection"].includes(name),
      ),
    ).toBe(false);
  });
  it("기존 서버 연결과 새 서버 구성에서 입력한 주소를 양방향으로 공유한다", async () => {
    current = { collection_enabled: false, collection_state: "disabled" };
    await mount();
    nav("서버");
    fireEvent.change(
      screen.getByRole("textbox", { name: /^Insights 서버 주소/ }),
      { target: { value: "https://shared.example.com" } },
    );
    fireEvent.change(screen.getByRole("textbox", { name: /^Grafana 주소/ }), {
      target: { value: "https://dashboard.example.com" },
    });
    fireEvent.click(screen.getByRole("button", { name: /새 서버 구성/ }));
    expect(
      (
        screen.getByRole("textbox", {
          name: "Insights API 주소",
        }) as HTMLInputElement
      ).value,
    ).toBe("https://shared.example.com");
    expect(
      (
        screen.getByRole("textbox", {
          name: "Grafana 주소",
        }) as HTMLInputElement
      ).value,
    ).toBe("https://dashboard.example.com");
    fireEvent.change(
      screen.getByRole("textbox", { name: "Insights API 주소" }),
      { target: { value: "https://revised.example.com" } },
    );
    fireEvent.click(screen.getByRole("button", { name: /기존 서버 연결/ }));
    expect(
      (
        screen.getByRole("textbox", {
          name: /^Insights 서버 주소/,
        }) as HTMLInputElement
      ).value,
    ).toBe("https://revised.example.com");
    expect(
      rpc.mock.calls.every(([name]) =>
        ["snapshot", "get_app_preferences"].includes(name),
      ),
    ).toBe(true);
  });
  it("서버 메뉴 안에서 구성 작업으로 전환해도 확인 티켓과 임시 등록키를 취소한다", async () => {
    current = { collection_enabled: false, collection_state: "disabled" };
    await mount();
    nav("서버");
    fireEvent.change(
      screen.getByRole("textbox", { name: /^Insights 서버 주소/ }),
      { target: { value: "https://insights.example.com" } },
    );
    fireEvent.change(screen.getByPlaceholderText("등록키 입력"), {
      target: { value: "x".repeat(32) },
    });
    fireEvent.click(screen.getByRole("button", { name: "연결 확인" }));
    await screen.findByRole("button", { name: "동의하고 연결" });
    fireEvent.click(screen.getByRole("button", { name: /새 서버 구성/ }));
    await screen.findByRole("heading", { name: "접속 주소" });
    expect(rpc).toHaveBeenCalledWith("cancel_connection");
    fireEvent.click(screen.getByRole("button", { name: /기존 서버 연결/ }));
    expect(
      (screen.getByPlaceholderText("등록키 입력") as HTMLInputElement).value,
    ).toBe("");
    expect(screen.queryByRole("button", { name: "동의하고 연결" })).toBeNull();
  });
  it("새 서버 초안의 주소를 고쳐도 현재 연결에는 적용하지 않는다", async () => {
    await mount();
    nav("서버");
    fireEvent.click(screen.getByText("연결 진단과 등록키 관리"));
    fireEvent.click(screen.getByRole("button", { name: "등록키 다시 확인" }));
    fireEvent.click(screen.getByRole("button", { name: /새 서버 구성/ }));
    fireEvent.change(
      screen.getByRole("textbox", { name: "Insights API 주소" }),
      { target: { value: "https://new.example.com" } },
    );
    fireEvent.click(screen.getByRole("button", { name: /기존 서버 연결/ }));
    expect(screen.getByText("https://insights.example.com")).toBeTruthy();
    expect(screen.queryByPlaceholderText("등록키 입력")).toBeNull();
    expect(
      rpc.mock.calls.every(([name]) =>
        ["snapshot", "get_app_preferences"].includes(name),
      ),
    ).toBe(true);
  });
});
