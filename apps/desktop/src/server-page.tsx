import {
  useEffect,
  useState,
  type FormEvent,
  type Dispatch,
  type SetStateAction,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowDown,
  ArrowRight,
  Check,
  ChevronDown,
  Database,
  FileDown,
  FolderOpen,
  Globe2,
  LockKeyhole,
  Monitor,
  RefreshCw,
} from "lucide-react";
import { Button, Input, Field, Notice } from "./ui";
import { initialSetup, setupError, setupStepError, type Setup } from "./model";
import compatibility from "../../../infrastructure/compatibility.json";
export function ServerPage({
  setup,
  setSetup,
  busy,
  action,
  setError,
  setSuccess,
  onConnect,
}: {
  setup: Setup;
  setSetup: Dispatch<SetStateAction<Setup>>;
  busy: string;
  action: (name: string, fn: () => Promise<void>) => Promise<boolean>;
  setError: (value: string) => void;
  setSuccess: (value: string) => void;
  onConnect: (api: string, grafana: string) => void;
}) {
  const native = isTauri();
  const [step, setStep] = useState(0);
  const [advanced, setAdvanced] = useState(false);
  const [exported, setExported] = useState("");
  useEffect(() => {
    setExported("");
    setStep(0);
  }, [setup.apiUrl, setup.grafanaUrl]);
  const [exportedCustomCredentials, setExportedCustomCredentials] =
    useState(false);
  useEffect(() => {
    window.scrollTo({ top: 0 });
  }, [step]);
  function update<K extends keyof Setup>(key: K, value: Setup[K]) {
    setSetup((s) => ({ ...s, [key]: value }));
    setExported("");
    setSuccess("");
    setError("");
  }
  function submitStep(e: FormEvent) {
    e.preventDefault();
    if (step < 2) {
      const issue = setupStepError(setup, step);
      if (issue) {
        setError(issue);
        return;
      }
      setError("");
      setStep(step + 1);
      return;
    }
    exportFiles();
  }
  function exportFiles() {
    const issue = setupError(setup);
    if (issue) {
      setError(issue);
      return;
    }
    if (!native) {
      setError(
        "파일 생성은 데스크톱 앱에서 실행하세요. 입력한 구성은 검토할 수 있습니다.",
      );
      return;
    }
    const input = { ...setup };
    void action("export", async () => {
      const result = await invoke<{ directory: string }>("export_compose", {
        input,
      });
      setExported(result.directory);
      setExportedCustomCredentials(
        [
          input.clickhousePassword,
          input.grafanaPassword,
          input.enrollmentKey,
        ].some(Boolean),
      );
      setSetup((s) => ({
        ...s,
        clickhousePassword: "",
        grafanaPassword: "",
        enrollmentKey: "",
      }));
      setSuccess(
        "비공개 설정 파일을 만들었습니다. 아래의 서버 실행 안내를 확인하세요.",
      );
    });
  }
  return (
    <>
      <div className="server-intro">
        <span className="tag">서버 관리자용</span>
        <p>
          주소는 기존 서버 연결 화면과 공유합니다. 이 구성은 파일로 생성하며,
          저장된 기기 연결에 자동으로 적용하지 않습니다.
        </p>
      </div>
      <ol className="workflow-steps" aria-label="서버 구성 단계">
        {["주소와 연결", "저장소와 이미지", "검토와 생성"].map((label, i) => (
          <li
            key={label}
            aria-current={step === i ? "step" : undefined}
            className={step >= i ? "current" : ""}
          >
            <span>{step > i ? <Check size={16} /> : i + 1}</span>
            {label}
          </li>
        ))}
      </ol>
      <div className="content-columns">
        <form
          id="compose-form"
          className="settings-form"
          noValidate
          onSubmit={submitStep}
        >
          <div hidden={step !== 0}>
            <section>
              <h2>접속 주소</h2>
              <div className="field-grid">
                <Field label="Insights API 주소">
                  <Input
                    type="url"
                    aria-label="Insights API 주소"
                    required
                    placeholder="https://insights.example.com"
                    value={setup.apiUrl}
                    onChange={(e) => update("apiUrl", e.target.value)}
                  />
                </Field>
                <Field label="Grafana 주소">
                  <Input
                    type="url"
                    required
                    placeholder="https://grafana.example.com"
                    value={setup.grafanaUrl}
                    onChange={(e) => update("grafanaUrl", e.target.value)}
                  />
                </Field>
              </div>
            </section>
            <section>
              <h2>연결 방식</h2>
              <div
                className="segmented"
                role="group"
                aria-label="서버 연결 방식"
              >
                <Button
                  type="button"
                  aria-pressed={setup.mode === "https"}
                  onClick={() => update("mode", "https")}
                >
                  <Globe2 size={16} />
                  일반 HTTPS
                </Button>
                <Button
                  type="button"
                  aria-pressed={setup.mode === "tailscale"}
                  onClick={() => update("mode", "tailscale")}
                >
                  Tailscale <span className="optional">선택 사항</span>
                </Button>
              </div>
              {setup.mode === "tailscale" ? (
                <div className="spaced">
                  <Field
                    label="서버 Tailnet IPv4"
                    hint="Tailscale을 사용하는 서버에만 필요합니다."
                  >
                    <Input
                      placeholder="100.64.0.1"
                      value={setup.tailnetIp}
                      onChange={(e) => update("tailnetIp", e.target.value)}
                      required
                    />
                  </Field>
                </div>
              ) : (
                <p className="helper">
                  로컬 포트를 HTTPS 프록시에 연결합니다. Tailscale 설치는
                  필요하지 않습니다.
                </p>
              )}
            </section>
          </div>
          <div hidden={step !== 1}>
            <section>
              <h2>저장소와 포트</h2>
              <Field label="데이터 저장 경로">
                <Input
                  required
                  value={setup.datasetRoot}
                  onChange={(e) => update("datasetRoot", e.target.value)}
                  spellCheck={false}
                />
              </Field>
              <div className="field-grid spaced">
                <Field label="API 포트 (Insights)">
                  <Input
                    type="number"
                    min="1024"
                    max="65535"
                    value={setup.apiPort}
                    onChange={(e) => update("apiPort", +e.target.value)}
                    required
                  />
                </Field>
                <Field label="Grafana 포트">
                  <Input
                    type="number"
                    min="1024"
                    max="65535"
                    value={setup.grafanaPort}
                    onChange={(e) => update("grafanaPort", +e.target.value)}
                    required
                  />
                </Field>
              </div>
            </section>
            <section>
              <div className="section-title">
                <h2>ClickHouse</h2>
                <span className="quiet">내부 네트워크</span>
              </div>
              <div className="field-grid triple">
                <Field label="데이터베이스">
                  <Input value="groundline" readOnly />
                </Field>
                <Field label="사용자">
                  <Input value="groundline_ingest" readOnly />
                </Field>
                <Field label="비밀번호">
                  <Input
                    type="password"
                    autoComplete="new-password"
                    placeholder="자동 생성"
                    value={setup.clickhousePassword}
                    onChange={(e) =>
                      update("clickhousePassword", e.target.value)
                    }
                  />
                </Field>
              </div>
              <p className="helper">
                컨테이너 내부에서만 연결됩니다. Grafana는 별도의 읽기 전용
                계정을 사용합니다.
              </p>
            </section>
            <section>
              <h2>배포 이미지</h2>{" "}
              <Field
                label="Insights API 이미지"
                hint="배포할 버전의 ghcr.io 이미지와 @sha256: digest를 입력하세요."
              >
                <Input
                  value={setup.apiImage}
                  placeholder="ghcr.io/jukqaz/groundline-insights-api@sha256:…"
                  onChange={(e) => update("apiImage", e.target.value)}
                  spellCheck={false}
                />
              </Field>
            </section>
            <details
              className="advanced"
              open={advanced}
              onToggle={(e) => setAdvanced(e.currentTarget.open)}
            >
              <summary>
                서비스 자격증명 (선택)
                <ChevronDown size={16} />
              </summary>
              <div className="field-grid spaced">
                <Field label="Grafana 관리자 비밀번호">
                  <Input
                    type="password"
                    autoComplete="new-password"
                    placeholder="자동 생성"
                    value={setup.grafanaPassword}
                    onChange={(e) => update("grafanaPassword", e.target.value)}
                  />
                </Field>
                <Field label="기기 등록키">
                  <Input
                    type="password"
                    autoComplete="new-password"
                    placeholder="자동 생성"
                    value={setup.enrollmentKey}
                    onChange={(e) => update("enrollmentKey", e.target.value)}
                  />
                </Field>
              </div>
              <p className="helper">
                비우면 64자리 값을 생성합니다. 직접 입력할 때는 영문·숫자·-·_로
                32~128자를 사용하세요.
              </p>
            </details>
          </div>
          {step === 2 && (
            <section className="review-panel">
              <h2>생성 전 최종 확인</h2>
              <p className="helper">
                아래 값으로 Docker Compose 파일을 만듭니다. 생성 후 서버에서
                실행할 수 있습니다.
              </p>
              <dl className="facts">
                <div>
                  <dt>연결 방식</dt>
                  <dd>{setup.mode === "https" ? "일반 HTTPS" : "Tailscale"}</dd>
                </div>
                <div>
                  <dt>Insights API</dt>
                  <dd>{setup.apiUrl}</dd>
                </div>
                <div>
                  <dt>Grafana</dt>
                  <dd>{setup.grafanaUrl}</dd>
                </div>
                <div>
                  <dt>서버 저장 경로</dt>
                  <dd>{setup.datasetRoot}</dd>
                </div>
                <div>
                  <dt>공개할 포트</dt>
                  <dd>
                    API {setup.apiPort} · Grafana {setup.grafanaPort}
                  </dd>
                </div>
                <div>
                  <dt>API 이미지</dt>
                  <dd className="image-digest">{setup.apiImage}</dd>
                </div>
                <div>
                  <dt>ClickHouse</dt>
                  <dd>groundline · 내부 전용 · :8123</dd>
                </div>
                <div>
                  <dt>등록키·비밀번호</dt>
                  <dd>
                    {(
                      exported
                        ? exportedCustomCredentials
                        : [
                            setup.clickhousePassword,
                            setup.grafanaPassword,
                            setup.enrollmentKey,
                          ].some(Boolean)
                    )
                      ? "직접 지정한 값 + 나머지 자동 생성"
                      : "모두 안전하게 자동 생성"}
                  </dd>
                </div>
              </dl>
            </section>
          )}
          <Notice>
            <LockKeyhole size={14} />
            등록키와 비밀번호는 생성 파일에만 저장됩니다.
          </Notice>
          <div className="actions">
            {step > 0 && (
              <Button
                className="secondary"
                type="button"
                disabled={!!busy}
                onClick={() => {
                  setStep(step - 1);
                  setError("");
                }}
              >
                이전 단계
              </Button>
            )}
            <Button className="primary" disabled={!!busy} type="submit">
              {busy === "export" ? (
                <RefreshCw className="spin" size={17} />
              ) : (
                <FileDown size={17} />
              )}
              {step === 2 ? "Compose 파일 만들기" : "다음 단계"}
            </Button>
            <Button
              className="secondary"
              type="button"
              disabled={!!busy}
              onClick={() => {
                setSetup(initialSetup);
                setStep(0);
                setError("");
                setExported("");
                setSuccess("");
              }}
            >
              기본값 복원
            </Button>
          </div>
          <p className="helper">
            다운로드 폴더에 새 비공개 폴더를 만듭니다. 기존 배포 파일을 덮어쓰지
            않습니다.
          </p>
          {exported && (
            <section className="export-next" aria-label="생성 결과">
              <div className="section-title">
                <h2>설정 파일을 만들었습니다</h2>
                <Check size={22} />
              </div>
              <p className="endpoint">{exported}</p>
              <ul>
                <li>
                  <strong>compose.yaml</strong> — 서비스 구성과 자격증명
                </li>
                <li>
                  <strong>secrets.json</strong> — 관리자 비밀번호와 기기 등록키
                </li>
                <li>
                  <strong>connection.json</strong> — 공유할 접속 주소
                </li>
                <li>
                  <strong>README.ko.md</strong> — 서버 실행·HTTPS 설정 안내
                </li>
              </ul>
              <div className="actions">
                <Button
                  type="button"
                  className="primary"
                  disabled={!!busy}
                  onClick={() =>
                    void action("reveal", async () => {
                      await invoke("reveal_export");
                    })
                  }
                >
                  <FolderOpen size={17} />
                  생성 폴더 열기
                </Button>
                <Button
                  type="button"
                  className="secondary"
                  disabled={!!busy}
                  onClick={() => onConnect(setup.apiUrl, setup.grafanaUrl)}
                >
                  이 주소로 기기 연결
                  <ArrowRight size={16} />
                </Button>
              </div>
              <p className="helper">
                README 순서대로 서버를 실행하고 HTTPS를 준비한 뒤 연결하세요.
                파일 생성만으로 서버가 실행되거나 수집이 시작되지는 않습니다.
              </p>
            </section>
          )}
        </form>
        <aside className="preview-panel">
          <h2>구성 미리보기</h2>
          <h3>서비스 구성도</h3>
          <div className="architecture">
            <div className="proxy-node">
              {setup.mode === "https" ? "HTTPS" : "Tailscale"}
              <small>
                {setup.mode === "https" ? "리버스 프록시" : "선택한 사설 연결"}
              </small>
            </div>
            <div className="arrows">
              <ArrowDown />
              <ArrowDown />
            </div>
            <div className="nodes">
              <div>
                Insights API<small>:{setup.apiPort || "—"} → :8080</small>
              </div>
              <div>
                Grafana<small>:{setup.grafanaPort || "—"} → :3000</small>
              </div>
            </div>
            <div className="arrows">
              <ArrowDown />
              <ArrowDown />
            </div>
            <div className="nodes">
              <div>
                ClickHouse<small>:8123</small>
              </div>
              <div>
                읽기 전용<small>ClickHouse</small>
              </div>
            </div>
          </div>
          <div className="included">
            <h3>포함되는 서비스</h3>
            {[
              [
                Activity,
                "Insights API",
                "데이터 수집",
                "동의한 활동 통계를 받는 API 서버입니다.",
              ],
              [
                Database,
                "ClickHouse",
                "활동 저장",
                "수집된 통계를 저장하는 데이터베이스입니다.",
              ],
              [
                Monitor,
                "Grafana",
                "대시보드",
                "읽기 전용 데이터 소스와 대시보드를 구성합니다.",
              ],
            ].map(([Icon, name, tag, desc]) => {
              const I = Icon as typeof Activity;
              return (
                <div className="service" key={String(name)}>
                  <div>
                    <I size={16} />
                    <strong>{String(name)}</strong>
                    <span>{String(tag)}</span>
                  </div>
                  <p>{String(desc)}</p>
                </div>
              );
            })}
          </div>
          <details className="versions">
            <summary>
              고정된 의존성 보기
              <ChevronDown size={14} />
            </summary>
            <p>
              ClickHouse{" "}
              {compatibility.clickhouse_image.split(":")[1].split("@")[0]}
            </p>
            <p>
              Grafana {compatibility.grafana_image.split(":")[1].split("@")[0]}
            </p>
            <p>
              데이터 소스{" "}
              {compatibility.grafana_clickhouse_plugin.split("@")[1]}
            </p>
          </details>
        </aside>
      </div>
    </>
  );
}
