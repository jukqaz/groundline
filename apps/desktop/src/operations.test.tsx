// @vitest-environment jsdom
import {
  act,
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Theme } from "@radix-ui/themes";
import {
  diagnosticStages,
  parseConnectionFile,
  UsageCard,
  DeliveryHistory,
  ConnectionImport,
} from "./operations";

const rpc = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke: rpc }));
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  rpc.mockReset();
  vi.unstubAllGlobals();
});

describe("diagnostic evidence", () => {
  it("keeps hook and authentication unknown when only server health succeeds", () => {
    const steps = diagnosticStages(
      {
        collection_state: "active",
        collection_enabled: true,
        consent_status: "active",
      },
      {
        reachable: true,
        contract_compatible: true,
        storage_ready: true,
        checked_at_utc: "2026-09-10T00:00:00Z",
      },
    );
    expect(steps[1].state).toBe("neutral");
    expect(steps[3].state).toBe("good");
    expect(steps[4].state).toBe("good");
    expect(steps[5].state).toBe("neutral");
  });
  it("latest authentication failure overrides an older successful receipt", () => {
    const steps = diagnosticStages(
      {
        last_delivery_error_code: "remote_authentication_rejected",
        delivery_confirmation: {
          confirmed_at_utc: "2026-09-09T00:00:00Z",
          event_count: 1,
        },
      },
      null,
    );
    expect(steps[5].state).toBe("warning");
    expect(steps[5].detail).toContain("인증 거절");
  });
});

describe("connection import", () => {
  const input = {
    schema: 1,
    kind: "groundline-connection",
    api_url: "https://insights.example.com",
    grafana_url: "",
  };
  it("accepts only the current address-only contract", () => {
    expect(parseConnectionFile(JSON.stringify(input)).api_url).toBe(
      input.api_url,
    );
    for (const bad of [
      { ...input, token: "secret" },
      { ...input, schema: 99 },
      { ...input, api_url: "https://user:pass@example.com" },
      { ...input, api_url: "https://example.com?token=secret" },
      { ...input, grafana_url: "javascript:alert(1)" },
    ]) {
      expect(() => parseConnectionFile(JSON.stringify(bad))).toThrow();
    }
    expect(() => parseConnectionFile(" ".repeat(8193))).toThrow();
  });
  it("previews imported addresses without invoking enrollment or saving credentials", async () => {
    const apply = vi.fn(async () => {});
    render(
      <Theme>
        <ConnectionImport disabled={false} apply={apply} />
      </Theme>,
    );
    fireEvent.change(screen.getByLabelText("연결 정보 파일"), {
      target: {
        files: [{ size: 100, text: async () => JSON.stringify(input) }],
      },
    });
    await waitFor(() => expect(apply).toHaveBeenCalledWith(input));
    expect(screen.getByText(/주소를 불러왔습니다/)).toBeTruthy();
    expect(rpc).not.toHaveBeenCalled();
  });
  it("discards a file read when its screen or runtime is replaced", async () => {
    const apply = vi.fn(async () => {});
    let complete!: (value: string) => void;
    const content = new Promise<string>((resolve) => {
      complete = resolve;
    });
    const view = render(
      <Theme>
        <ConnectionImport disabled={false} apply={apply} />
      </Theme>,
    );
    fireEvent.change(screen.getByLabelText("연결 정보 파일"), {
      target: { files: [{ size: 100, text: () => content }] },
    });
    view.unmount();
    await act(async () => {
      complete(JSON.stringify(input));
      await content;
    });
    expect(apply).not.toHaveBeenCalled();
  });
});

describe("local usage and history", () => {
  it("reads usage only when asked and preserves unknown cache ratio", async () => {
    rpc.mockResolvedValue({
      start_utc: "2026-09-10T00:00:00Z",
      end_utc: "2026-09-10T09:00:00Z",
      complete: false,
      root_count: 12,
      total_tokens: 1440,
      cache_ratio: null,
    });
    render(
      <Theme>
        <UsageCard
          runtime="codex_cli"
          native
          disabled={false}
          dashboard={() => {}}
        />
      </Theme>,
    );
    expect(rpc).not.toHaveBeenCalled();
    fireEvent.click(screen.getByText("사용량 불러오기"));
    await waitFor(() => expect(screen.getByText("1,440")).toBeTruthy());
    expect(rpc).toHaveBeenCalledWith("usage_summary", {
      runtime: "codex_cli",
      period: "today",
    });
    expect(screen.getByText(/일부 기록만 집계/)).toBeTruthy();
    expect(screen.queryByText("0%")).toBeNull();
  });
  it("does not call a duplicate response a new receipt", () => {
    render(
      <Theme>
        <DeliveryHistory
          status={{
            activity_history: {
              last_hook_at_utc: null,
              entries: [
                {
                  at_utc: "2026-09-10T09:00:00Z",
                  outcome: "duplicate",
                  event_count: 1,
                  reason: "duplicate",
                },
                {
                  at_utc: "2026-09-10T08:00:00Z",
                  outcome: "checked",
                  event_count: 0,
                  reason: "no_new_delivery",
                },
              ],
            },
          }}
        />
      </Theme>,
    );
    expect(screen.getByText("중복 수신 · 1건")).toBeTruthy();
    expect(screen.getByText("이번 실행에서 새 전송 없음")).toBeTruthy();
    expect(screen.queryByText("새 수신 · 1건")).toBeNull();
  });
});
