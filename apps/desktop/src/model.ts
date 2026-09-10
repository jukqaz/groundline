export type Theme = "light" | "system" | "dark";
export type Runtime = "codex_app" | "codex_cli";
export type AppPreferences = {
  schema: 1;
  close_action: "tray" | "quit";
  runtime: Runtime;
  alerts_enabled?: boolean;
};
export const defaultPreferences: AppPreferences = {
  schema: 1,
  close_action: "tray",
  runtime: "codex_app",
  alerts_enabled: false,
};
export type Status = {
  codex_state_store_present?: boolean;
  owner_profile_configured?: boolean;
  enrollment_credential_valid?: boolean;
  delivery_operator_required?: boolean;
  last_delivery_error_code?: string;
  history_unavailable?: boolean;
  activity_history?: {
    last_hook_at_utc: string | null;
    entries: {
      at_utc: string;
      outcome: "accepted" | "duplicate" | "checked" | "failed";
      event_count: number;
      reason: string;
    }[];
  };
  collection_state?: string;
  collection_enabled?: boolean;
  ready_to_collect?: boolean;
  last_success_utc?: string;
  last_check_utc?: string;
  pending_event_count?: number;
  quarantined_event_count?: number;
  delivery_attempt_count?: number;
  delivery_next_attempt_utc?: string;
  consent_status?: string;
  endpoint?: string;
  grafana_url?: string;
  tailnet_required?: boolean;
  blocking_reason_codes?: string[];
  delivery_confirmation?: {
    confirmed_at_utc: string;
    event_count: number;
  } | null;
};
export type CoreStatus = {
  version: string;
  status: string;
  checksum_verified: boolean;
  live_hooks_verified: boolean;
};
export function formatTime(value?: string): string {
  if (!value) return "기록 없음";
  const date = new Date(value);
  return Number.isNaN(date.getTime())
    ? "시간 확인 필요"
    : date.toLocaleString("ko-KR");
}
export function nextStep(status: Status | null) {
  if (!status)
    return {
      title: "이 기기의 상태를 확인하세요",
      detail: "데스크톱 앱에서 상태를 새로고침하면 필요한 작업을 안내합니다.",
      action: "refresh",
    } as const;
  if (!status.endpoint)
    return {
      title: "Insights 서버를 연결하세요",
      detail: "관리자에게 받은 서버 주소와 등록키로 시작하세요.",
      action: "connect",
    } as const;
  if (!status.collection_enabled || status.consent_status !== "active")
    return {
      title: "수집 동의를 확인하세요",
      detail:
        "서버 설정은 보존되어 있습니다. 수집 범위를 확인하고 다시 시작할 수 있습니다.",
      action: "collection",
    } as const;
  const code = status.collection_state;
  if (
    code === "active" &&
    status.delivery_confirmation &&
    status.pending_event_count === 0
  )
    return {
      title: "서버 수신이 확인됐습니다",
      detail: "현재 수집 동의에 따라 Codex 훅이 실행될 때 전송합니다.",
      action: "connect",
    } as const;
  if (code === "active")
    return {
      title: "최근 수집이 정상적으로 완료되었습니다",
      detail: "서버 메뉴에서 전송 대기와 최근 확인 시간을 살펴보세요.",
      action: "connect",
    } as const;
  return {
    title: collectionLabel(code),
    detail: reasonHelp(status.blocking_reason_codes?.[0]),
    action: "connect",
  } as const;
}
const guidance: Record<string, string> = {
  owner_profile_required: "서버 주소와 등록키를 입력해 연결을 준비하세요.",
  invalid_owner_profile:
    "저장된 연결 설정을 확인하세요. 기존 데이터는 보존됩니다.",
  enrollment_credential_required:
    "등록키가 없습니다. 연결 정보를 다시 확인하세요.",
  invalid_enrollment_credential:
    "등록키를 다시 입력하고 서버 인증을 확인하세요.",
  delivery_operator_action_required:
    "서버의 등록키, API와 ClickHouse 실행 상태를 확인한 뒤 전송을 다시 확인하세요.",
  api_upgrade_required: "관리자에게 Insights API 업데이트를 요청하세요.",
  reconsent_required: "서버의 자동 수집에서 범위를 확인하고 다시 동의하세요.",
  tailnet_not_connected: "선택한 Tailscale 연결을 켠 뒤 상태를 새로고침하세요.",
  tailnet_connection_unverified:
    "Tailscale 실행 상태와 서버 접근 경로를 확인하세요.",
  outbox_capacity_exceeded:
    "대기 데이터가 저장 한도에 도달했습니다. 서버 상태를 확인하고 전송을 재시도하세요.",
  codex_state_store_unavailable:
    "선택한 Codex App 또는 CLI에서 작업한 뒤 다시 확인하세요.",
  collection_clock_skew: "이 기기의 날짜와 시간을 확인하세요.",
  collection_stale:
    "최근 수집 기록이 오래되었습니다. 지금 수집·전송 확인을 실행하세요.",
  first_collection_pending:
    "아직 첫 수집이 완료되지 않았습니다. 지금 수집·전송 확인을 실행하세요.",
  delivery_pending:
    "대기 중인 데이터가 있습니다. 서버 연결을 확인한 뒤 전송할 수 있습니다.",
  retry_required:
    "최근 작업이 완료되지 않았습니다. 서버 상태를 확인하고 재시도하세요.",
  collection_incomplete:
    "일부 활동 기록을 집계하지 못해 전송을 보류했습니다. 로컬 기록 진단이 필요합니다.",
  collection_operator_action_required:
    "수집이 중단되었습니다. 로컬 상태를 보존한 채 진단이 필요합니다.",
};
export function reasonHelp(code?: string) {
  return (
    (code && guidance[code]) ||
    "현재 상태를 새로고침하고 연결 설정을 확인하세요."
  );
}
export type Setup = {
  apiUrl: string;
  grafanaUrl: string;
  datasetRoot: string;
  apiPort: number;
  grafanaPort: number;
  apiImage: string;
  mode: "https" | "tailscale";
  tailnetIp: string;
  clickhousePassword: string;
  grafanaPassword: string;
  enrollmentKey: string;
};
export const initialSetup: Setup = {
  apiUrl: "",
  grafanaUrl: "",
  datasetRoot: "/srv/groundline",
  apiPort: 18080,
  grafanaPort: 13000,
  apiImage: "",
  mode: "https",
  tailnetIp: "",
  clickhousePassword: "",
  grafanaPassword: "",
  enrollmentKey: "",
};
export function storedTheme(value: string | null): Theme {
  return value === "light" || value === "dark" ? value : "system";
}
export function resolveTheme(
  theme: Theme,
  systemDark: boolean,
): "light" | "dark" {
  return theme === "system" ? (systemDark ? "dark" : "light") : theme;
}
export function validOrigin(value: string, httpsOnly = false): boolean {
  try {
    const url = new URL(value);
    const tailnet =
      url.hostname.endsWith(".ts.net") ||
      /^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\.\d{1,3}\.\d{1,3}$/.test(
        url.hostname,
      );
    const local = ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname);
    return (
      (url.protocol === "https:" ||
        (!httpsOnly && url.protocol === "http:" && (local || tailnet))) &&
      url.pathname === "/" &&
      !url.username &&
      !url.password &&
      !url.search &&
      !url.hash
    );
  } catch {
    return false;
  }
}
export function validDashboardOrigin(value: string): boolean {
  if (value.length > 2048 || !validOrigin(value)) return false;
  const url = new URL(value);
  return (
    url.protocol === "https:" ||
    ["localhost", "127.0.0.1", "[::1]"].includes(url.hostname)
  );
}
export function setupError(input: Setup): string | null {
  const addressIssue = setupStepError(input, 0);
  if (addressIssue) return addressIssue;
  if (!validOrigin(input.apiUrl, input.mode === "https"))
    return "API 주소를 확인하세요. 일반 연결에는 HTTPS 주소가 필요합니다.";
  if (!validOrigin(input.grafanaUrl, true))
    return "Grafana의 HTTPS 주소를 입력하세요.";
  if (!input.datasetRoot.match(/^(\/[^/]|[A-Za-z]:[\\/])/))
    return "서버의 절대 데이터 경로를 입력하세요.";
  if (
    ![input.apiPort, input.grafanaPort].every(
      (p) => Number.isInteger(p) && p >= 1024 && p <= 65535,
    ) ||
    input.apiPort === input.grafanaPort
  )
    return "포트는 1024~65535 사이의 서로 다른 값이어야 합니다.";
  if (!/^.+@sha256:[a-f0-9]{64}$/.test(input.apiImage))
    return "배포할 Insights API 이미지의 SHA-256 digest를 입력하세요.";
  if (
    input.mode === "tailscale" &&
    !/^100\.(6[4-9]|[7-9]\d|1[01]\d|12[0-7])\.\d{1,3}\.\d{1,3}$/.test(
      input.tailnetIp,
    )
  )
    return "Tailscale 모드에는 서버의 Tailnet IPv4 주소가 필요합니다.";
  if (
    [input.clickhousePassword, input.grafanaPassword, input.enrollmentKey].some(
      (v) => v !== "" && !/^[A-Za-z0-9_-]{32,128}$/.test(v),
    )
  )
    return "직접 지정할 비밀번호·등록키는 영문, 숫자, -와 _로 32~128자를 입력하세요. 비우면 안전하게 생성합니다.";
  return null;
}
export function setupStepError(input: Setup, step: number): string | null {
  if (step > 0) return setupError(input);
  if (!validOrigin(input.apiUrl, input.mode === "https"))
    return "API 주소를 확인하세요. 일반 연결에는 HTTPS 주소가 필요합니다.";
  if (!validOrigin(input.grafanaUrl, true))
    return "Grafana의 HTTPS 주소를 입력하세요.";
  const host = new URL(input.apiUrl).hostname;
  const isTailnet = host.endsWith(".ts.net") || tailnetIp(host);
  if (input.mode === "tailscale" && (!isTailnet || !tailnetIp(input.tailnetIp)))
    return "Tailscale 모드에는 Tailnet 서버 주소와 유효한 Tailnet IPv4가 필요합니다.";
  if (input.mode === "https" && isTailnet)
    return "Tailscale 주소를 사용하려면 연결 방식을 Tailscale로 선택하세요.";
  return null;
}
function tailnetIp(value: string): boolean {
  const parts = value.split(".");
  return (
    parts.length === 4 &&
    parts.every((p) => /^\d{1,3}$/.test(p) && Number(p) <= 255) &&
    Number(parts[0]) === 100 &&
    Number(parts[1]) >= 64 &&
    Number(parts[1]) <= 127
  );
}
const reasons: Record<string, string> = {
  disabled: "수집 꺼짐",
  awaiting_first_collection: "첫 수집 대기",
  active: "수집 중",
  configuration_required: "설정 필요",
  delivery_operator_action_required: "전송 재확인 필요",
  tailnet_disconnected: "Tailscale 연결 필요",
  native_activity_unavailable: "Codex 활동 기록 대기",
  reconsent_required: "수집 동의 필요",
  delivery_pending: "전송 대기",
  api_upgrade_required: "서버 업데이트 필요",
  unsupported_local_state: "현재 버전이 지원하지 않는 로컬 상태",
  tailnet_unverified: "Tailscale 확인 필요",
  outbox_capacity_exceeded: "대기 데이터 한도 도달",
  collection_operator_action_required: "수집 점검 필요",
  collection_incomplete: "기록 집계 확인 필요",
  clock_skew: "기기 시간 확인 필요",
  stale: "최근 수집 확인 필요",
  retry_required: "다시 확인 필요",
};
export function collectionLabel(code: string | undefined) {
  return code ? (reasons[code] ?? "상태 확인 필요") : "확인 전";
}
const errors: Record<string, string> = {
  usage_unavailable:
    "이 환경의 Codex 활동 기록을 읽지 못했습니다. Codex App 또는 CLI 선택을 확인하세요.",
  notification_permission_required:
    "시스템 설정에서 GroundLine 알림을 허용한 뒤 다시 켜세요.",
  notification_unavailable:
    "시스템 알림을 사용할 수 없습니다. 트레이와 앱에서 상태를 확인할 수 있습니다.",
  invalid_owner_profile: "서버 주소와 등록키 형식을 확인하세요.",
  enrollment_credential_rejected:
    "Insights 등록키가 일치하지 않습니다. 서버 API의 GROUNDLINE_ENROLLMENT_TOKEN을 확인하세요. TrueNAS 관리용 API 키와는 별개입니다.",
  remote_authentication_rejected:
    "서버가 인증을 거절했습니다. Insights 기기 등록키와 프록시 설정을 확인하세요. TrueNAS 관리용 API 키와는 별개입니다.",
  proxy_authentication_rejected: "서버의 프록시 인증 설정을 확인하세요.",
  tailnet_peer_rejected:
    "서버가 Tailscale 전용으로 설정되어 있습니다. 서버의 연결 방식을 확인하세요.",
  enrollment_disabled: "서버 관리자가 기기 등록을 켜야 합니다.",
  api_upgrade_required:
    "연결 확인을 지원하는 Insights API 버전으로 서버를 업데이트하세요.",
  collector_enrollment_failed:
    "서버에 연결하지 못했습니다. 주소와 TLS 인증서를 확인하세요.",
  event_upload_failed: "서버 또는 ClickHouse 준비 상태를 확인하지 못했습니다.",
  invalid_compose:
    "데이터 경로, Tailnet IP, 이미지 digest와 포트 설정을 확인하세요.",
  invalid_service_password:
    "직접 입력하는 비밀번호·등록키는 영문·숫자·-·_로 32~128자를 사용하세요.",
  endpoint_change_requires_review:
    "기존 연결이 있습니다. 남아 있는 데이터와 기기 등록을 검토한 후 서버를 변경해야 합니다.",
  connection_check_required:
    "확인이 만료되었습니다. 등록키를 입력해 연결을 다시 확인하세요.",
  unsupported_local_state:
    "지원하지 않는 기존 상태가 있습니다. 원본을 보존한 채 버전을 확인해야 합니다.",
  core_not_installed: "Core 설치를 찾지 못했습니다.",
  invalid_core_package: "Core 패키지 무결성 확인에 실패했습니다.",
  worker_timeout: "작업 시간이 초과되었습니다. 현재 상태를 새로 확인하세요.",
  consent_required: "수집 범위를 읽고 동의해 주세요.",
  invalid_grafana_url:
    "경로와 자격증명을 제외한 Grafana HTTPS 주소를 입력하세요.",
  dashboard_open_failed:
    "대시보드를 열지 못했습니다. 기본 브라우저와 저장된 주소를 확인하세요.",
  export_not_available:
    "이번 실행에서 생성한 폴더가 없습니다. 설정 파일을 먼저 생성하세요.",
  collection_configuration_required:
    "저장된 서버 주소와 등록키를 먼저 확인하세요.",
};
export function errorMessage(error: unknown): string {
  return typeof error === "string" && errors[error]
    ? errors[error]
    : "작업을 완료하지 못했습니다. 입력값과 현재 상태를 확인하고 다시 시도하세요.";
}
