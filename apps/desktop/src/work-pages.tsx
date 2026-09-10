import { Button, Input, Choice } from "./ui";
import { useState } from "react";
import {
  DeliveryHistory,
  DiagnosticExport,
  Diagnostics,
  UsageCard,
} from "./operations";
import {
  Activity,
  CircleCheck,
  Link2,
  ChevronDown,
  ArrowRight,
  Check,
  Database,
  LockKeyhole,
  Monitor,
  Moon,
  RefreshCw,
  ShieldCheck,
  Sun,
  X,
  Send,
} from "lucide-react";
import {
  formatTime,
  nextStep,
  reasonHelp,
  type CoreStatus,
  type Status,
  type Theme,
  type AppPreferences,
  type Runtime,
} from "./model";

export function DeliverySummary({ status }: { status: Status | null }) {
  const receipt = status?.delivery_confirmation;
  return (
    <section className="delivery-card" aria-label="서버 수신 확인">
      <div className="section-title">
        <h2>서버 수신 확인</h2>
        <Send size={20} />
      </div>
      <dl className="delivery-facts">
        <div>
          <dt>최근 수신 건수</dt>
          <dd>
            {receipt ? `${receipt.event_count.toLocaleString("ko-KR")}건` : "—"}
          </dd>
        </div>
        <div className="receipt-time">
          <dt>서버 수신 확인 시각</dt>
          <dd>{formatTime(receipt?.confirmed_at_utc)}</dd>
        </div>
        <div>
          <dt>전송 대기</dt>
          <dd>
            {status?.pending_event_count == null
              ? "—"
              : `${status.pending_event_count.toLocaleString("ko-KR")}건`}
          </dd>
        </div>
        <div>
          <dt>마지막 수집 완료</dt>
          <dd className="collection-time">
            {formatTime(status?.last_success_utc)}
          </dd>
        </div>
      </dl>
      <p className="helper">
        {receipt
          ? "최근 전송 묶음 기준 · 중복 수신 포함"
          : "서버의 수신 확인 기록이 생기면 표시됩니다."}
      </p>
    </section>
  );
}

export function Attention({ status }: { status: Status | null }) {
  if (!status?.blocking_reason_codes?.length) return null;
  return (
    <section className="attention-panel" aria-label="확인이 필요한 항목">
      <h3>다음 확인 사항</h3>
      <ul>
        {status.blocking_reason_codes.map((code) => (
          <li key={code}>{reasonHelp(code)}</li>
        ))}
      </ul>
    </section>
  );
}

export function Overview({
  runtime,
  dashboard,
  status,
  busy,
  native,
  navigate,
  refresh,
}: {
  runtime: Runtime;
  dashboard: () => void;
  status: Status | null;
  busy: boolean;
  native: boolean;
  navigate: (target: "connection" | "collection") => void;
  refresh: () => void;
}) {
  const next = nextStep(status);
  return (
    <>
      <section className="next-action">
        <span
          className="connection-indicator"
          data-active={status?.collection_state === "active"}
          aria-hidden="true"
        >
          {status?.collection_state === "active" &&
          status.delivery_confirmation &&
          status.pending_event_count === 0 ? (
            <CircleCheck size={34} />
          ) : (
            <Activity size={30} />
          )}
        </span>
        <div>
          <h2>{next.title}</h2>
          <p>{next.detail}</p>
          {status?.endpoint && (
            <p className="endpoint connected-endpoint">
              <Link2 size={14} />
              {status.endpoint}
            </p>
          )}
        </div>
        <Button
          className="primary"
          disabled={busy || (next.action === "refresh" && !native)}
          onClick={() =>
            next.action === "refresh"
              ? refresh()
              : navigate(
                  next.action === "collection" ? "collection" : "connection",
                )
          }
        >
          {next.action === "refresh"
            ? "상태 확인"
            : next.action === "collection"
              ? "수집 설정"
              : status?.endpoint
                ? "연결 관리"
                : "서버 연결 보기"}
          <ArrowRight size={16} />
        </Button>
      </section>
      <DeliverySummary status={status} />
      <UsageCard
        key={runtime}
        runtime={runtime}
        native={native}
        disabled={busy}
        dashboard={dashboard}
      />
      <DeliveryHistory status={status} />
    </>
  );
}

export function ConnectionManager({
  runtime,
  status,
  busy,
  native,
  repair,
  stop,
  resume,
  run,
  openDashboard,
  saveDashboard,
}: {
  runtime: Runtime;
  status: Status;
  busy: boolean;
  native: boolean;
  repair: () => void;
  stop: () => void;
  resume: () => Promise<boolean>;
  run: () => void;
  openDashboard: () => void;
  saveDashboard: (value: string) => Promise<boolean>;
}) {
  const [editingDashboard, setEditingDashboard] = useState(false);
  const [dashboard, setDashboard] = useState(status.grafana_url || "");
  return (
    <>
      <section className="connection-summary" aria-label="연결 관리">
        <div className="section-title">
          <div>
            <h2>Insights 서버</h2>
          </div>
          <span className="tag" data-active={status.collection_enabled}>
            {status.tailnet_required ? "Tailscale" : "일반 연결"}
          </span>
        </div>
        <p className="endpoint">{status.endpoint}</p>
        <p className="helper">
          연결은 공통으로 사용하고, 수집 설정은 선택한 Codex 환경에 적용합니다.
        </p>
        <div className="actions">
          <Button
            className="primary"
            disabled={!native || busy || !status.collection_enabled}
            onClick={run}
          >
            지금 수집·전송 확인
            <ArrowRight size={16} />
          </Button>
        </div>
      </section>
      <CollectionSettings
        status={status}
        busy={busy}
        native={native}
        stop={stop}
        resume={resume}
      />
      <Attention status={status} />
      <section id="dashboard" tabIndex={-1} className="overview-section">
        <div>
          <h2>Grafana 대시보드</h2>
          <p className="endpoint">
            {status.grafana_url ||
              "대시보드 주소를 추가하면 여기서 바로 열 수 있습니다."}
          </p>
        </div>
        <div className="actions">
          <Button
            className="secondary"
            disabled={!native || busy || !status.grafana_url}
            onClick={openDashboard}
          >
            대시보드 열기
            <ArrowRight size={16} />
          </Button>
          <Button
            className="text-button"
            disabled={busy}
            onClick={() => {
              setDashboard(status.grafana_url || "");
              setEditingDashboard(!editingDashboard);
            }}
          >
            주소 {status.grafana_url ? "수정" : "추가"}
          </Button>
        </div>
      </section>
      {editingDashboard && (
        <form
          className="inline-editor"
          onSubmit={async (e) => {
            e.preventDefault();
            if (await saveDashboard(dashboard)) setEditingDashboard(false);
          }}
        >
          <label className="field">
            <span>대시보드 주소</span>
            <Input
              type="url"
              value={dashboard}
              onChange={(e) => setDashboard(e.target.value)}
              placeholder="https://grafana.example.com"
            />
            <small>
              외부 서버는 HTTPS, 이 기기의 localhost는 HTTP도 지원합니다. 비우고
              저장하면 바로가기만 제거됩니다. 모든 Codex 환경에서 공통으로
              사용합니다.
            </small>
          </label>
          <div className="actions">
            <Button className="primary" disabled={!native || busy}>
              주소 저장
            </Button>
            <Button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={() => setEditingDashboard(false)}
            >
              취소
            </Button>
          </div>
        </form>
      )}
      <details className="diagnostic-details">
        <summary>연결 점검과 전송 상세</summary>
        <Diagnostics
          key={runtime}
          status={status}
          runtime={runtime}
          native={native}
          disabled={busy}
        />

        <dl className="facts">
          <div>
            <dt>마지막 상태 확인</dt>
            <dd>{formatTime(status.last_check_utc)}</dd>
          </div>
          <div>
            <dt>전송 시도</dt>
            <dd>{status.delivery_attempt_count ?? 0}회</dd>
          </div>
          <div>
            <dt>다음 자동 재시도</dt>
            <dd>{formatTime(status.delivery_next_attempt_utc)}</dd>
          </div>
          <div>
            <dt>격리된 이벤트</dt>
            <dd>{status.quarantined_event_count ?? 0}건</dd>
          </div>
        </dl>
        <p className="helper">
          인증 실패 시 같은 서버의 등록키를 다시 확인할 수 있습니다. 서버
          변경에는 기존 등록과 대기 데이터의 별도 검토가 필요합니다.
        </p>
        <Button className="secondary" disabled={busy} onClick={repair}>
          등록키 다시 확인
        </Button>
      </details>
    </>
  );
}

export function Consent({
  checked,
  setChecked,
}: {
  checked: boolean;
  setChecked: (value: boolean) => void;
}) {
  return (
    <div className="consent-panel">
      <ShieldCheck size={26} />
      <h2>수집할 정보를 확인하세요</h2>
      <p>
        처음 연결하면 최근 7일의 활동 통계를 전송합니다. 이후에는 새로 발생한
        활동을 수집합니다.
      </p>
      <ul>
        <li>토큰 사용량과 모델·도구 사용 통계</li>
        <li>작업 시간, 완료 여부, 오류 분류</li>
        <li>내용과 경로를 제외한 제한된 진단 메타데이터</li>
      </ul>
      <p className="helper">
        대화 원문, 코드, 파일 경로와 자격증명은 전송 대상에서 제외합니다.
      </p>
      <label className="checkbox">
        <input
          type="checkbox"
          checked={checked}
          onChange={(e) => setChecked(e.target.checked)}
        />
        <span>
          수집 범위와 초기 기록 동기화를 확인했으며, 이 서버로 전송하는 데
          동의합니다.
        </span>
      </label>
    </div>
  );
}

function CollectionSettings({
  status,
  busy,
  native,
  stop,
  resume,
}: {
  status: Status;
  busy: boolean;
  native: boolean;
  stop: () => void;
  resume: () => Promise<boolean>;
}) {
  const [resuming, setResuming] = useState(false);
  const [consent, setConsent] = useState(false);
  return (
    <>
      <section
        id="collection"
        tabIndex={-1}
        className="collection-controls"
        aria-label="활동 통계 수집"
      >
        <div className="section-title">
          <h3>자동 수집</h3>
          <span className="tag">
            {status
              ? status.collection_enabled
                ? "수집 켜짐"
                : "수집 꺼짐"
              : "확인 전"}
          </span>
        </div>
        <p className="helper">
          Codex 사용 시 통계를 전송합니다. 수집을 꺼도 기존 기록은 보존됩니다.
        </p>
        {status?.collection_enabled ? (
          <Button
            className="secondary stop-button"
            disabled={!native || busy}
            onClick={stop}
          >
            수집 중지
          </Button>
        ) : (
          <Button
            className="primary"
            disabled={!native || busy || resuming}
            onClick={() => {
              setResuming(true);
              setConsent(false);
            }}
          >
            동의 후 다시 시작
          </Button>
        )}
        {resuming && !status?.collection_enabled && (
          <div className="resume-panel">
            <p className="endpoint">전송 대상: {status?.endpoint}</p>
            <Consent checked={consent} setChecked={setConsent} />
            <div className="actions">
              <Button
                className="primary"
                disabled={!consent || busy || !native}
                onClick={async () => {
                  if (await resume()) {
                    setResuming(false);
                    setConsent(false);
                  }
                }}
              >
                동의하고 수집 재개
              </Button>
              <Button
                className="secondary"
                disabled={busy}
                onClick={() => {
                  setResuming(false);
                  setConsent(false);
                }}
              >
                취소
              </Button>
            </div>
          </div>
        )}
      </section>
      <details className="settings-block privacy-details">
        <summary>
          <ShieldCheck size={18} />
          개인정보 처리 안내 <ChevronDown size={16} />
        </summary>
        {[
          [
            Check,
            "활동 통계",
            "사용량, 시간, 모델·도구 분류와 완료 상태를 연결한 Insights 서버의 ClickHouse에 저장합니다.",
          ],
          [
            X,
            "원문과 자격증명 제외",
            "대화 원문, 소스 코드, 파일 경로와 자격증명은 전송 대상에서 제외합니다.",
          ],
          [
            LockKeyhole,
            "이 기기의 등록키",
            "등록키는 제한된 로컬 파일에 보관합니다. 연결 확인 취소·만료 시 임시 등록키를 해제하며, 브라우저 저장소에 남기지 않습니다.",
          ],
          [
            Database,
            "서버의 데이터",
            "수집 중지는 서버 데이터 삭제와 별개입니다. 보관 기간과 삭제는 서버 관리자가 관리합니다. Grafana는 읽기 전용 계정을 사용합니다.",
          ],
        ].map(([Icon, title, description]) => {
          const I = Icon as typeof Check;
          return (
            <div className="privacy-row" key={String(title)}>
              <I size={20} />
              <div>
                <h3>{String(title)}</h3>
                <p>{String(description)}</p>
              </div>
            </div>
          );
        })}
      </details>
    </>
  );
}

export function SettingsPage({
  runtime,
  core,
  diagnose,
  setAlerts,
  theme,
  setTheme,
  preferences,
  preferencesError,
  setCloseAction,
  busy,
  native,
}: {
  runtime: Runtime;
  core: CoreStatus | null;
  diagnose: () => void;
  setAlerts: (enabled: boolean) => void;
  theme: Theme;
  setTheme: (v: Theme) => void;
  preferences: AppPreferences;
  preferencesError: string;
  setCloseAction: (value: AppPreferences["close_action"]) => void;
  busy: boolean;
  native: boolean;
}) {
  return (
    <div className="settings-page">
      <section className="settings-block preference-row">
        <h2>
          <label htmlFor="close-action">창 닫기</label>
        </h2>
        <Choice
          id="close-action"
          label="창 닫기"
          value={preferences.close_action}
          disabled={!native || busy || !!preferencesError}
          options={[
            ["tray", "트레이에 숨기기"],
            ["quit", "앱 종료"],
          ]}
          onValueChange={(value) =>
            setCloseAction(value as AppPreferences["close_action"])
          }
        />
        {preferencesError && (
          <p className="preference-error" role="alert">
            {preferencesError}
          </p>
        )}
      </section>
      <section className="settings-block preference-row">
        <div>
          <h2>테마</h2>
          <p className="helper">시스템을 선택하면 기기 설정을 따릅니다.</p>
        </div>
        <div className="theme-segment" role="group" aria-label="화면 테마">
          {(
            [
              ["system", Monitor, "시스템"],
              ["light", Sun, "라이트"],
              ["dark", Moon, "다크"],
            ] as const
          ).map(([id, Icon, label]) => (
            <Button
              key={id}
              aria-pressed={theme === id}
              onClick={() => setTheme(id)}
            >
              <Icon size={16} />
              <span>{label}</span>
            </Button>
          ))}
        </div>
      </section>
      <section className="settings-block preference-row">
        <div>
          <h2>문제 알림</h2>
          <p className="helper">트레이 실행 중 반복 실패·조치 필요 시 알림</p>
        </div>
        <Choice
          label="문제 알림"
          value={preferences.alerts_enabled ? "on" : "off"}
          disabled={!native || busy || !!preferencesError}
          options={[
            ["off", "꺼짐"],
            ["on", "켜짐"],
          ]}
          onValueChange={(v) => setAlerts(v === "on")}
        />
      </section>
      <details className="diagnostic-details settings-diagnostics">
        <summary>문제 해결</summary>
        <section className="settings-block core-settings">
          <div className="section-title">
            <h2>GroundLine Core</h2>
            <ShieldCheck size={20} />
            <strong
              className="metric"
              data-state={
                core
                  ? core.status === "PASS"
                    ? "success"
                    : "attention"
                  : "unknown"
              }
            >
              {core
                ? core.status === "PASS"
                  ? "실행 진단 통과"
                  : "확인 필요"
                : "진단 전"}
            </strong>
            <Button
              className="secondary"
              disabled={!native || busy}
              onClick={diagnose}
            >
              <RefreshCw size={16} />
              Core 진단
            </Button>
          </div>
          {core && (
            <details className="core-details">
              <summary>
                진단 상세 <ChevronDown size={14} />
              </summary>
              <dl className="facts compact">
                <div>
                  <dt>패키지 버전</dt>
                  <dd>{core.version}</dd>
                </div>
                <div>
                  <dt>패키지 무결성</dt>
                  <dd>{core.checksum_verified ? "확인 완료" : "확인 필요"}</dd>
                </div>
                <div>
                  <dt>실제 훅 실행</dt>
                  <dd>
                    {core.live_hooks_verified ? "확인 완료" : "별도 확인 필요"}
                  </dd>
                </div>
              </dl>
            </details>
          )}
        </section>
        <DiagnosticExport
          key={runtime}
          runtime={runtime}
          native={native}
          disabled={busy}
        />
      </details>
    </div>
  );
}
