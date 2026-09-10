import { describe, it, expect } from "vitest";
import {
  storedTheme,
  resolveTheme,
  validOrigin,
  validDashboardOrigin,
  setupError,
  initialSetup,
  errorMessage,
  nextStep,
  setupStepError,
  reasonHelp,
  formatTime,
} from "./model";

describe("사용자 설정과 경계", () => {
  it("대시보드는 외부 HTTPS와 loopback HTTP만 허용한다", () => {
    for (const url of [
      "https://grafana.example.com",
      "http://localhost:23100",
      "http://127.0.0.1:23100",
      "http://[::1]:23100",
    ])
      expect(validDashboardOrigin(url)).toBe(true);
    for (const url of [
      "http://192.168.1.1",
      "http://100.64.0.1",
      "http://localhost.example.com",
      "http://localhost@evil.example.com",
      "http://localhost/path",
      "http://localhost?token=value",
      "http://localhost#fragment",
    ])
      expect(validDashboardOrigin(url)).toBe(false);
  });
  it("수집 완료를 서버 수신 성공으로 오인하지 않는다", () => {
    const status = {
      endpoint: "https://example.com",
      collection_enabled: true,
      consent_status: "active",
      collection_state: "active",
      pending_event_count: 0,
    };
    expect(nextStep(status).title).not.toContain("수신");
    const acknowledged = {
      ...status,
      delivery_confirmation: {
        event_count: 1,
        confirmed_at_utc: "2026-09-10T00:00:00Z",
      },
    };
    expect(nextStep(acknowledged).title).toContain("수신");
    expect(
      nextStep({ ...acknowledged, pending_event_count: 1 }).title,
    ).not.toContain("수신");
    expect(
      nextStep({ ...acknowledged, collection_enabled: false }).action,
    ).toBe("settings");
  });
  it("첫 실행과 잘못된 저장값은 시스템 테마를 사용한다", () => {
    expect(storedTheme(null)).toBe("system");
    expect(storedTheme("unexpected")).toBe("system");
    expect(resolveTheme(storedTheme(null), true)).toBe("dark");
    expect(resolveTheme(storedTheme(null), false)).toBe("light");
    expect(resolveTheme(storedTheme("light"), true)).toBe("light");
  });
  it("일반 HTTPS와 선택형 Tailscale을 허용하고 URL 자격증명과 리다이렉트 입력을 거부한다", () => {
    for (const url of [
      "https://insights.example.com",
      "http://100.64.0.1:18080",
      "http://localhost:18080",
    ])
      expect(validOrigin(url)).toBe(true);
    for (const url of [
      "http://example.com",
      "https://user:password@example.com",
      "https://example.com/path",
      "https://example.com?redirect=1",
      "javascript:alert(1)",
    ])
      expect(validOrigin(url)).toBe(false);
  });
  it("포트 충돌과 Compose 주입 문자를 거부한다", () => {
    const input = {
      ...initialSetup,
      apiUrl: "https://insights.example.com",
      grafanaUrl: "https://grafana.example.com",
      apiImage:
        "ghcr.io/jukqaz/groundline-insights-api@sha256:" + "a".repeat(64),
    };
    expect(setupError(input)).toBeNull();
    expect(setupError({ ...input, grafanaPort: input.apiPort })).toContain(
      "서로 다른",
    );
    expect(
      setupError({ ...input, enrollmentKey: "x".repeat(32) + "$VALUE" }),
    ).toContain("비밀번호");
  });
  it("미등록 오류 본문을 사용자 화면으로 전달하지 않는다", () => {
    expect(errorMessage("PRIVATE_SENTINEL")).not.toContain("PRIVATE_SENTINEL");
  });
  it("현재 연결과 동의 상태에 따라 다음 작업을 안내한다", () => {
    expect(nextStep(null).action).toBe("refresh");
    expect(nextStep({ collection_enabled: false }).action).toBe("connect");
    expect(
      nextStep({ endpoint: "https://example.com", collection_enabled: false })
        .action,
    ).toBe("settings");
    expect(
      nextStep({
        endpoint: "https://example.com",
        collection_enabled: true,
        consent_status: "active",
        collection_state: "active",
      }).title,
    ).toContain("완료");
    expect(reasonHelp("PRIVATE_REASON")).not.toContain("PRIVATE_REASON");
    expect(formatTime("not a timestamp")).toBe("시간 확인 필요");
  });
  it("서버 단계별 검증은 필수 이미지와 Tailscale 모드 불일치를 숨기지 않는다", () => {
    const input = {
      ...initialSetup,
      apiUrl: "https://insights.example.com",
      grafanaUrl: "https://grafana.example.com",
    };
    expect(setupStepError(input, 0)).toBeNull();
    expect(setupStepError(input, 1)).toContain("digest");
    expect(
      setupStepError(
        { ...input, mode: "tailscale", tailnetIp: "100.64.0.1" },
        0,
      ),
    ).toContain("Tailnet");
    expect(
      setupStepError(
        {
          ...input,
          mode: "tailscale",
          apiUrl: "http://100.64.0.1",
          tailnetIp: "100.64.0.999",
        },
        0,
      ),
    ).toContain("IPv4");
  });
});
