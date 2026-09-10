import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowDownToLine,
  ArrowUpRight,
  Check,
  ChevronDown,
  CircleHelp,
  Clock3,
  FileInput,
  RefreshCw,
  ShieldCheck,
  TriangleAlert,
} from "lucide-react";
import { Button, Choice, Notice } from "./ui";
import {
  errorMessage,
  formatTime,
  validOrigin,
  validDashboardOrigin,
  type Runtime,
  type Status,
} from "./model";

type Usage = {
  start_utc: string;
  end_utc: string;
  complete: boolean;
  root_count: number | null;
  total_tokens: number | null;
  cache_ratio: number | null;
};
const number = (value: number | null | undefined) =>
  value == null ? "—" : value.toLocaleString("ko-KR");

export function UsageCard({
  runtime,
  native,
  disabled,
  dashboard,
}: {
  runtime: Runtime;
  native: boolean;
  disabled: boolean;
  dashboard: () => void;
}) {
  const [period, setPeriod] = useState("today");
  const [usage, setUsage] = useState<Usage | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  async function load(next = period) {
    const id = ++request.current;
    setPeriod(next);
    setLoading(true);
    setError("");
    setUsage(null);
    try {
      const value = await invoke<Usage>("usage_summary", {
        runtime,
        period: next,
      });
      if (id === request.current) setUsage(value);
    } catch (e) {
      if (id === request.current) setError(errorMessage(e));
    } finally {
      if (id === request.current) setLoading(false);
    }
  }
  return (
    <section
      className="usage-card"
      aria-label="이 기기 사용량"
      aria-busy={loading}
    >
      <div className="section-title">
        <div>
          <h2>이 기기 사용량</h2>
          <p className="eyebrow">
            {runtime === "codex_app" ? "Codex App" : "Codex CLI"} · 로컬 집계
          </p>
        </div>
        <div className="inline-controls">
          <Choice
            label="사용량 기간"
            value={period}
            disabled={loading || disabled}
            options={[
              ["today", "오늘"],
              ["week", "최근 7일"],
            ]}
            onValueChange={(next) => {
              setPeriod(next);
              if (usage || error) void load(next);
            }}
          />
          <Button
            className="text-button icon-button"
            aria-label="사용량 새로고침"
            title="사용량 새로고침"
            disabled={!native || disabled || loading}
            onClick={() => void load()}
          >
            <RefreshCw size={16} className={loading ? "spin" : ""} />
          </Button>
        </div>
      </div>
      {usage ? (
        <>
          <dl className="usage-metrics">
            <div>
              <dt>토큰</dt>
              <dd>{number(usage.total_tokens)}</dd>
            </div>
            <div>
              <dt>작업</dt>
              <dd>{number(usage.root_count)}</dd>
            </div>
            <div>
              <dt>입력 캐시</dt>
              <dd>
                {usage.cache_ratio == null
                  ? "—"
                  : `${Math.round(usage.cache_ratio * 100)}%`}
              </dd>
            </div>
          </dl>
          <div className="usage-footer">
            <p className="helper">
              {usage.complete ? "집계 완료" : "일부 기록만 집계"} ·{" "}
              {formatTime(usage.end_utc)} 기준
            </p>
            <Button
              className="text-button"
              onClick={dashboard}
              disabled={disabled}
            >
              상세 분석 <ArrowUpRight size={15} />
            </Button>
          </div>
          <p className="metric-note">
            토큰은 하위 에이전트 포함 · 작업은 루트 작업 기준 · 기기 시간대
          </p>
        </>
      ) : (
        <div className="usage-empty">
          <Activity size={24} />
          <div>
            <strong>
              {loading
                ? "사용량을 계산하고 있습니다"
                : "선택한 기간의 작업 요약"}
            </strong>
            <p>이 기기의 활동 통계만 읽으며 서버로 보내지 않습니다.</p>
          </div>
          <Button
            disabled={!native || disabled || loading}
            onClick={() => void load()}
          >
            {loading ? "계산 중…" : "사용량 불러오기"}
          </Button>
        </div>
      )}
      {error && <Notice kind="error">{error}</Notice>}
    </section>
  );
}

const outcomes = {
  accepted: "새 수신",
  duplicate: "중복 수신",
  checked: "수집 확인",
  failed: "확인 필요",
};
const historyReasons: Record<string, string> = {
  accepted: "서버가 통계를 수신했습니다",
  duplicate: "이미 수신한 데이터입니다",
  no_new_delivery: "이번 실행에서 새 전송 없음",
  authentication: "등록키·서버 인증 확인",
  api_upgrade: "Insights API 업데이트 필요",
  collection: "Codex 활동 기록 확인",
  connection: "서버 접근 경로 확인",
  delivery: "서버 응답·저장소 확인",
  check_required: "연결 진단에서 확인",
};
export function DeliveryHistory({ status }: { status: Status | null }) {
  const entries = status?.activity_history?.entries ?? [];
  const [filter, setFilter] = useState("all");
  const visible =
    filter === "failed"
      ? entries.filter((e) => e.outcome === "failed")
      : entries;
  return (
    <details className="history-panel">
      <summary>
        <Clock3 size={17} />
        <span>최근 전송 이력</span>
        <span className="history-count">{entries.length}개</span>
        <ChevronDown size={16} />
      </summary>
      <div className="history-body">
        <div className="section-title">
          <p className="helper">최근 20개 실행·수신 기록</p>
          <Choice
            label="전송 이력 필터"
            value={filter}
            options={[
              ["all", "전체"],
              ["failed", "확인 필요"],
            ]}
            onValueChange={setFilter}
          />
        </div>
        {status?.history_unavailable ? (
          <Notice kind="error">
            이력 파일을 읽을 수 없습니다. 기존 파일은 보존됩니다.
          </Notice>
        ) : visible.length ? (
          <ul className="history-list">
            {visible.map((entry, i) => (
              <li key={`${entry.at_utc}-${i}`}>
                <span
                  className="history-icon"
                  data-tone={
                    entry.outcome === "failed"
                      ? "warning"
                      : entry.outcome === "accepted"
                        ? "good"
                        : "neutral"
                  }
                >
                  {entry.outcome === "failed" ? (
                    <TriangleAlert size={15} />
                  ) : entry.outcome === "accepted" ? (
                    <Check size={15} />
                  ) : (
                    <Clock3 size={15} />
                  )}
                </span>
                <div>
                  <strong>
                    {outcomes[entry.outcome]}
                    {entry.event_count > 0
                      ? ` · ${number(entry.event_count)}건`
                      : ""}
                  </strong>
                  <p>{historyReasons[entry.reason] ?? "연결 진단에서 확인"}</p>
                </div>
                <time dateTime={entry.at_utc}>{formatTime(entry.at_utc)}</time>
              </li>
            ))}
          </ul>
        ) : (
          <p className="empty-line">
            {filter === "failed" && entries.length
              ? "확인이 필요한 기록이 없습니다."
              : "아직 기록이 없습니다. 이 버전의 수집기가 실행되면 표시됩니다."}
          </p>
        )}
      </div>
    </details>
  );
}

type Health = {
  checked_at_utc: string;
  reachable: boolean;
  storage_ready: boolean | null;
  contract_compatible: boolean;
};
export type Stage = {
  title: string;
  state: "good" | "warning" | "neutral";
  detail: string;
};
export function diagnosticStages(
  status: Status | null,
  health: Health | null,
): Stage[] {
  const hook = status?.activity_history?.last_hook_at_utc;
  const receipt = status?.delivery_confirmation;
  const authRejected = [
    "remote_authentication_rejected",
    "enrollment_credential_rejected",
    "proxy_authentication_rejected",
  ].includes(status?.last_delivery_error_code ?? "");
  return [
    {
      title: "Codex 활동 기록",
      state:
        status?.codex_state_store_present === true
          ? "good"
          : status?.codex_state_store_present === false
            ? "warning"
            : "neutral",
      detail: status?.codex_state_store_present
        ? "선택한 환경의 로컬 저장소를 찾았습니다"
        : "선택한 App 또는 CLI에서 작업한 뒤 확인하세요",
    },
    {
      title: "훅 수집 실행",
      state: hook ? "good" : "neutral",
      detail: hook
        ? `최근 처리 ${formatTime(hook)} · 훅 신뢰 설정은 별도 확인`
        : "호출 기록 없음 · 수동 전송 성공과 구분합니다",
    },
    {
      title: "자동 수집",
      state:
        status?.collection_enabled && status?.consent_status === "active"
          ? "good"
          : "neutral",
      detail: status?.collection_enabled
        ? "Codex 훅이 실행될 때 통계를 수집합니다"
        : "수집 꺼짐 · 위의 자동 수집에서 다시 시작할 수 있습니다",
    },
    {
      title: "Insights API",
      state: health
        ? health.contract_compatible
          ? "good"
          : "warning"
        : "neutral",
      detail: health
        ? health.contract_compatible
          ? "현재 수신 규격과 호환됩니다"
          : "현재 수신 규격을 지원하는 API로 업데이트하세요"
        : "서버 점검을 실행하면 응답과 호환성을 확인합니다",
    },
    {
      title: "ClickHouse",
      state:
        health?.storage_ready === true
          ? "good"
          : health?.storage_ready === false
            ? "warning"
            : "neutral",
      detail:
        health?.storage_ready === true
          ? "서버가 저장소 준비 완료를 보고했습니다"
          : health?.storage_ready === false
            ? "서버의 ClickHouse 실행·연결 설정을 확인하세요"
            : "저장소 준비 상태 확인 전",
    },
    {
      title: "인증·서버 수신",
      state: authRejected ? "warning" : receipt ? "good" : "neutral",
      detail: authRejected
        ? "최근 인증 거절 · 등록키 다시 확인을 실행하세요"
        : receipt
          ? `수신 기록 ${formatTime(receipt.confirmed_at_utc)} · 현재 연결 보장은 아님`
          : "아직 수신 확인 없음 · 서버 점검만으로 전송 성공이 되지 않습니다",
    },
  ];
}
export function Diagnostics({
  status,
  runtime,
  native,
  disabled,
}: {
  status: Status | null;
  runtime: Runtime;
  native: boolean;
  disabled: boolean;
}) {
  const [health, setHealth] = useState<Health | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const request = useRef(0);
  useEffect(
    () => () => {
      request.current++;
    },
    [],
  );
  async function check() {
    const id = ++request.current;
    setLoading(true);
    setError("");
    setHealth(null);
    try {
      const value = await invoke<Health>("server_health", { runtime });
      if (id === request.current) setHealth(value);
    } catch (e) {
      if (id === request.current) setError(errorMessage(e));
    } finally {
      if (id === request.current) setLoading(false);
    }
  }
  return (
    <section className="diagnosis-panel" aria-label="단계별 연결 진단">
      <div className="section-title">
        <h2>연결 진단</h2>
        <Button
          disabled={!native || disabled || loading || !status?.endpoint}
          onClick={() => void check()}
        >
          <RefreshCw size={15} className={loading ? "spin" : ""} />
          {loading ? "점검 중…" : "서버 점검"}
        </Button>
      </div>
      <p className="helper">
        {health
          ? `${formatTime(health.checked_at_utc)} 점검 · 등록·전송 없이 서버 상태만 확인`
          : "로컬 기록과 서버 상태를 단계별로 확인합니다."}
      </p>
      {error && <Notice kind="error">{error}</Notice>}
      <ol className="diagnosis-list">
        {diagnosticStages(status, health).map((stage) => (
          <li key={stage.title}>
            <span className="stage-icon" data-tone={stage.state}>
              {stage.state === "good" ? (
                <Check size={16} />
              ) : stage.state === "warning" ? (
                <TriangleAlert size={16} />
              ) : (
                <CircleHelp size={16} />
              )}
            </span>
            <div>
              <strong>{stage.title}</strong>
              <p>{stage.detail}</p>
            </div>
            <span className="stage-state" data-tone={stage.state}>
              {stage.state === "good"
                ? "확인"
                : stage.state === "warning"
                  ? "확인 필요"
                  : "대기"}
            </span>
          </li>
        ))}
      </ol>
    </section>
  );
}

export type ConnectionFile = {
  schema: 1;
  kind: "groundline-connection";
  api_url: string;
  grafana_url: string;
};
export function parseConnectionFile(text: string): ConnectionFile {
  if (text.length > 8192) throw new Error("invalid_connection_file");
  const value: unknown = JSON.parse(text);
  if (!value || typeof value !== "object" || Array.isArray(value))
    throw new Error("invalid_connection_file");
  const v = value as Record<string, unknown>;
  if (
    Object.keys(v).some(
      (k) => !["schema", "kind", "api_url", "grafana_url"].includes(k),
    ) ||
    v.schema !== 1 ||
    v.kind !== "groundline-connection" ||
    typeof v.api_url !== "string" ||
    v.api_url.length > 2048 ||
    !validOrigin(v.api_url) ||
    typeof v.grafana_url !== "string" ||
    (v.grafana_url !== "" && !validDashboardOrigin(v.grafana_url))
  )
    throw new Error("invalid_connection_file");
  return v as ConnectionFile;
}
export function ConnectionImport({
  disabled,
  apply,
}: {
  disabled: boolean;
  apply: (value: ConnectionFile) => Promise<void>;
}) {
  const input = useRef<HTMLInputElement>(null);
  const request = useRef(0);
  useEffect(
    () => () => {
      request.current += 1;
    },
    [],
  );
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [loaded, setLoaded] = useState(false);
  return (
    <div className="connection-import">
      <input
        ref={input}
        type="file"
        accept=".json,application/json"
        hidden
        aria-label="연결 정보 파일"
        onChange={async (e) => {
          const file = e.currentTarget.files?.[0];
          e.currentTarget.value = "";
          if (!file) return;
          const current = ++request.current;
          setError("");
          setLoading(true);
          setLoaded(false);
          try {
            if (file.size > 8192) throw new Error("invalid_connection_file");
            const value = parseConnectionFile(await file.text());
            if (request.current !== current) return;
            await apply(value);
            if (request.current === current) setLoaded(true);
          } catch {
            if (request.current === current)
              setError(
                "연결 정보 파일을 확인하세요. 서버 주소만 포함한 GroundLine JSON 파일을 사용할 수 있습니다.",
              );
          } finally {
            if (request.current === current) setLoading(false);
          }
        }}
      />
      <Button
        type="button"
        disabled={disabled || loading}
        onClick={() => input.current?.click()}
      >
        <FileInput size={16} />
        {loading ? "읽는 중…" : "연결 정보 가져오기"}
      </Button>
      <p className="helper">
        {loaded
          ? "주소를 불러왔습니다. 등록키를 입력해 연결을 확인하세요."
          : "관리자에게 받은 connection.json · 등록키는 별도 입력"}
      </p>
      {error && <Notice kind="error">{error}</Notice>}
    </div>
  );
}

export function DiagnosticExport({
  runtime,
  native,
  disabled,
}: {
  runtime: Runtime;
  native: boolean;
  disabled: boolean;
}) {
  const [busy, setBusy] = useState(false);
  const [exported, setExported] = useState(false);
  const [error, setError] = useState("");
  async function perform(reveal: boolean) {
    setBusy(true);
    setError("");
    try {
      if (reveal) await invoke("reveal_export");
      else {
        await invoke("export_diagnostics", { runtime });
        setExported(true);
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="settings-block preference-row">
      <div>
        <h2>
          <ShieldCheck size={16} /> 진단 파일
        </h2>
        <p className="helper">서버 주소·등록키·원문을 제외한 진단 파일</p>
      </div>
      <div className="inline-controls">
        <Button
          disabled={!native || disabled || busy}
          onClick={() => void perform(false)}
        >
          <ArrowDownToLine size={16} />
          {busy ? "처리 중…" : "진단 내보내기"}
        </Button>
        {exported && (
          <Button
            className="text-button"
            disabled={busy}
            onClick={() => void perform(true)}
          >
            폴더 열기
          </Button>
        )}
      </div>
      {exported && (
        <p className="helper preference-feedback">
          다운로드 폴더에 진단 파일을 저장했습니다.
        </p>
      )}
      {error && (
        <p className="preference-error" role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
