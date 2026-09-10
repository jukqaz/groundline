import { useState } from "react";
import {
  Activity,
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
} from "lucide-react";
import {
  collectionLabel,
  formatTime,
  nextStep,
  reasonHelp,
  type CoreStatus,
  type Status,
  type Theme,
} from "./model";

export function CollectionFacts({ status }: { status: Status | null }) {
  return (
    <dl className="facts">
      <div>
        <dt>수집 상태</dt>
        <dd>{collectionLabel(status?.collection_state)}</dd>
      </div>
      <div>
        <dt>수집 동의</dt>
        <dd>
          {!status
            ? "확인 전"
            : status.consent_status === "active"
              ? "동의함"
              : "동의 필요"}
        </dd>
      </div>
      <div>
        <dt>마지막 수집 완료</dt>
        <dd>{status ? formatTime(status.last_success_utc) : "확인 전"}</dd>
      </div>
      <div>
        <dt>전송 대기</dt>
        <dd>
          {status?.pending_event_count == null
            ? "확인 전"
            : `${status.pending_event_count}건`}
        </dd>
      </div>
    </dl>
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
  status,
  core,
  busy,
  native,
  navigate,
  diagnose,
  refresh,
}: {
  status: Status | null;
  core: CoreStatus | null;
  busy: boolean;
  native: boolean;
  navigate: (target: "connection" | "settings" | "compose") => void;
  diagnose: () => void;
  refresh: () => void;
}) {
  const next = nextStep(status);
  return (
    <>
      <section className="next-action">
        <div className="large-mark">
          <Activity size={29} />
        </div>
        <div>
          <span className="eyebrow">지금 할 일</span>
          <h2>{next.title}</h2>
          <p>{next.detail}</p>
        </div>
        <button
          className="primary"
          disabled={busy || (next.action === "refresh" && !native)}
          onClick={() =>
            next.action === "refresh"
              ? refresh()
              : navigate(next.action === "connect" ? "connection" : "settings")
          }
        >
          {next.action === "refresh"
            ? "상태 확인"
            : next.action === "settings"
              ? "수집 설정"
              : "서버 연결 보기"}
          <ArrowRight size={16} />
        </button>
      </section>
      <div className="summary-grid">
        <section className="summary-card">
          <div className="section-title">
            <h2>GroundLine Core</h2>
            <ShieldCheck size={20} />
          </div>
          <strong className="metric">
            {core
              ? core.status === "PASS"
                ? "실행 진단 통과"
                : "확인 필요"
              : "진단 전"}
          </strong>
          <p>로컬 작업 규칙과 실행 상태를 점검합니다.</p>
          <button
            className="secondary"
            disabled={!native || busy}
            onClick={diagnose}
          >
            <RefreshCw size={16} />
            Core 진단
          </button>
          {core && (
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
          )}
        </section>
        <section className="summary-card">
          <div className="section-title">
            <h2>Insights</h2>
            <Activity size={20} />
          </div>
          <strong className="metric">
            {collectionLabel(status?.collection_state)}
          </strong>
          <p>
            {status?.endpoint
              ? "연결 설정이 저장되어 있습니다."
              : "활동 통계를 내 서버에서 확인하세요."}
          </p>
          <button
            className="secondary"
            disabled={busy}
            onClick={() => navigate("connection")}
          >
            {status?.endpoint ? "연결 관리" : "서버 연결"}
            <ArrowRight size={16} />
          </button>
          <dl className="facts compact">
            <div>
              <dt>마지막 수집 완료</dt>
              <dd>
                {status ? formatTime(status.last_success_utc) : "확인 전"}
              </dd>
            </div>
            <div>
              <dt>전송 대기</dt>
              <dd>
                {status?.pending_event_count == null
                  ? "확인 전"
                  : `${status.pending_event_count}건`}
              </dd>
            </div>
          </dl>
        </section>
      </div>
      <section className="overview-section">
        <div>
          <h2>직접 서버를 운영하시나요?</h2>
          <p>
            Docker Compose로 API·ClickHouse·Grafana 설정을 함께 준비할 수
            있습니다. 서버 메뉴에서 기존 서버에 연결하거나 새 구성을 만드세요.
          </p>
        </div>
        <button
          className="secondary"
          disabled={busy}
          onClick={() => navigate("compose")}
        >
          서버 구성
          <ArrowRight size={16} />
        </button>
      </section>
    </>
  );
}

export function ConnectionManager({
  status,
  busy,
  native,
  repair,
  settings,
  run,
  openDashboard,
  saveDashboard,
}: {
  status: Status;
  busy: boolean;
  native: boolean;
  repair: () => void;
  settings: () => void;
  run: () => void;
  openDashboard: () => void;
  saveDashboard: (value: string) => Promise<boolean>;
}) {
  const [editingDashboard, setEditingDashboard] = useState(false);
  const [dashboard, setDashboard] = useState(status.grafana_url || "");
  return (
    <>
      <section className="connection-summary">
        <div className="section-title">
          <div>
            <span className="eyebrow">저장된 연결</span>
            <h2>{collectionLabel(status.collection_state)}</h2>
          </div>
          <span className="tag">
            {status.tailnet_required ? "Tailscale" : "일반 연결"}
          </span>
        </div>
        <p className="endpoint">{status.endpoint}</p>
        <CollectionFacts status={status} />
        <div className="actions">
          <button
            className="primary"
            disabled={!native || busy || !status.collection_enabled}
            onClick={run}
          >
            지금 수집·전송 확인
            <ArrowRight size={16} />
          </button>
          <button className="secondary" disabled={busy} onClick={settings}>
            {status.collection_enabled ? "수집 설정" : "수집 다시 시작"}
          </button>
        </div>
        <p className="helper">
          수집 완료 기록과 서버 수신은 다를 수 있습니다. 전송 실행 결과에서
          서버가 받은 건수를 확인하세요.
        </p>
      </section>
      <Attention status={status} />
      <section className="overview-section">
        <div>
          <h2>Grafana 대시보드</h2>
          <p className="endpoint">
            {status.grafana_url ||
              "대시보드 주소를 추가하면 여기서 바로 열 수 있습니다."}
          </p>
        </div>
        <div className="actions">
          <button
            className="secondary"
            disabled={!native || busy || !status.grafana_url}
            onClick={openDashboard}
          >
            대시보드 열기
            <ArrowRight size={16} />
          </button>
          <button
            className="text-button"
            disabled={busy}
            onClick={() => {
              setDashboard(status.grafana_url || "");
              setEditingDashboard(!editingDashboard);
            }}
          >
            주소 {status.grafana_url ? "수정" : "추가"}
          </button>
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
            <span>대시보드 HTTPS 주소</span>
            <input
              type="url"
              value={dashboard}
              onChange={(e) => setDashboard(e.target.value)}
              placeholder="https://grafana.example.com"
            />
            <small>
              비우고 저장하면 바로가기만 제거됩니다. 모든 Codex 환경에서
              공통으로 사용합니다.
            </small>
          </label>
          <div className="actions">
            <button className="primary" disabled={!native || busy}>
              주소 저장
            </button>
            <button
              type="button"
              className="secondary"
              disabled={busy}
              onClick={() => setEditingDashboard(false)}
            >
              취소
            </button>
          </div>
        </form>
      )}
      <details className="diagnostic-details">
        <summary>연결 진단과 등록키 관리</summary>
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
        <button className="secondary" disabled={busy} onClick={repair}>
          등록키 다시 확인
        </button>
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
        처음 연결하면 기존 Codex 활동 기록을 동기화합니다. 이후에는 활동
        체크포인트를 수집합니다.
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

export function SettingsPage({
  status,
  theme,
  setTheme,
  busy,
  native,
  stop,
  resume,
  connect,
}: {
  status: Status | null;
  theme: Theme;
  setTheme: (v: Theme) => void;
  busy: boolean;
  native: boolean;
  stop: () => void;
  resume: () => Promise<boolean>;
  connect: () => void;
}) {
  const [resuming, setResuming] = useState(false);
  const [consent, setConsent] = useState(false);
  return (
    <div className="settings-page">
      <section className="settings-block">
        <h2>화면 테마</h2>
        <p className="helper">
          기본값은 시스템입니다. 운영체제의 화면 모드가 바뀌면 함께 전환됩니다.
        </p>
        <div
          className="appearance-options"
          role="group"
          aria-label="설정 화면 테마"
        >
          {(
            [
              ["system", Monitor, "시스템", "운영체제 설정 따르기"],
              ["light", Sun, "라이트", "밝은 화면"],
              ["dark", Moon, "다크", "어두운 화면"],
            ] as const
          ).map(([id, Icon, label, desc]) => (
            <button
              key={id}
              aria-pressed={theme === id}
              onClick={() => setTheme(id)}
            >
              <Icon size={23} />
              <strong>{label}</strong>
              <small>{desc}</small>
              {theme === id && <Check className="choice-check" size={16} />}
            </button>
          ))}
        </div>
      </section>
      <section className="settings-block">
        <div className="section-title">
          <h2>개인정보와 수집</h2>
          <span className="tag">
            {status
              ? status.collection_enabled
                ? "수집 켜짐"
                : "수집 꺼짐"
              : "확인 전"}
          </span>
        </div>
        <CollectionFacts status={status} />
        <p className="helper">
          수집을 중지하면 이후 자동 수집을 멈춥니다. 기존 로컬 데이터와 서버에
          저장된 통계는 보존됩니다.
        </p>
        {status?.collection_enabled ? (
          <button
            className="secondary stop-button"
            disabled={!native || busy}
            onClick={stop}
          >
            수집 중지
          </button>
        ) : status?.endpoint ? (
          <button
            className="primary"
            disabled={!native || busy || resuming}
            onClick={() => {
              setResuming(true);
              setConsent(false);
            }}
          >
            동의 후 다시 시작
          </button>
        ) : (
          <button className="secondary" disabled={busy} onClick={connect}>
            서버 연결 설정
            <ArrowRight size={16} />
          </button>
        )}
        {resuming && !status?.collection_enabled && (
          <div className="resume-panel">
            <p className="endpoint">전송 대상: {status?.endpoint}</p>
            <Consent checked={consent} setChecked={setConsent} />
            <div className="actions">
              <button
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
              </button>
              <button
                className="secondary"
                disabled={busy}
                onClick={() => {
                  setResuming(false);
                  setConsent(false);
                }}
              >
                취소
              </button>
            </div>
          </div>
        )}
      </section>
      <section className="settings-block">
        <h2>어떤 정보가 어디에 남나요?</h2>
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
      </section>
    </div>
  );
}
