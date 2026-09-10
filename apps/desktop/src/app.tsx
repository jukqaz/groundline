import React, { useEffect, useRef, useState, type FormEvent } from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowRight,
  Check,
  CircleHelp,
  Globe2,
  Home,
  KeyRound,
  Laptop,
  Monitor,
  Moon,
  RefreshCw,
  Server,
  ShieldCheck,
  Sun,
} from "lucide-react";
import {
  errorMessage,
  initialSetup,
  type Setup,
  resolveTheme,
  storedTheme,
  validOrigin,
  type Runtime,
  type Theme,
  type Status,
  type CoreStatus,
} from "./model";
import { Field, Notice } from "./ui";
import {
  Overview,
  ConnectionManager,
  SettingsPage,
  Consent,
} from "./work-pages";
import { ServerPage } from "./server-page";
import "./style.css";

const native = isTauri();
type Page = "overview" | "server" | "settings";
type ServerView = "connection" | "compose";
export function App() {
  const [page, setPage] = useState<Page>("overview");
  const [serverView, setServerView] = useState<ServerView>("connection");
  const [setup, setSetup] = useState<Setup>(initialSetup);
  const [theme, setTheme] = useState<Theme>(() => {
    try {
      return storedTheme(localStorage.getItem("groundline-theme"));
    } catch {
      return "system";
    }
  });
  const [runtime, setRuntime] = useState<Runtime>("codex_app");
  const [status, setStatus] = useState<Status | null>(null);
  const [core, setCore] = useState<CoreStatus | null>(null);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [loading, setLoading] = useState(native);
  const [editingConnection, setEditingConnection] = useState(false);
  const running = useRef(false);
  const [checkedAt, setCheckedAt] = useState("");
  const heading = useRef<HTMLHeadingElement>(null);
  const endpoint = setup.apiUrl;
  const grafana = setup.grafanaUrl;
  function setEndpoint(value: string) {
    setSetup((s) => ({ ...s, apiUrl: value }));
  }
  function setGrafana(value: string) {
    setSetup((s) => ({ ...s, grafanaUrl: value }));
  }
  const [key, setKey] = useState("");
  const [ticket, setTicket] = useState("");
  const [consent, setConsent] = useState(false);
  const [connected, setConnected] = useState(false);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    window.scrollTo({ top: 0 });
  }, [page, serverView]);
  useEffect(() => {
    if (error) window.scrollTo({ top: 0 });
  }, [error]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(
        theme,
        media.matches,
      );
    };
    apply();
    media.addEventListener("change", apply);
    try {
      localStorage.setItem("groundline-theme", theme);
    } catch {
      /* Theme still works without storage. */
    }
    return () => media.removeEventListener("change", apply);
  }, [theme]);
  useEffect(() => {
    let current = true;
    setStatus(null);
    setServerView("connection");
    setSetup((s) => ({
      ...s,
      clickhousePassword: "",
      grafanaPassword: "",
      enrollmentKey: "",
    }));
    setTicket("");
    setConsent(false);
    setConnected(false);
    setKey("");
    setEndpoint("");
    setGrafana("");
    setEditingConnection(false);
    setError("");
    setSuccess("");
    setLoading(native);
    setCheckedAt("");
    if (native)
      invoke<Status>("snapshot", { runtime })
        .then((value) => {
          if (current) {
            setStatus(value);
            setEndpoint(value.endpoint || "");
            setGrafana(value.grafana_url || "");
            setCheckedAt(new Date().toLocaleTimeString("ko-KR"));
          }
        })
        .catch((e) => {
          if (current) setError(errorMessage(e));
        })
        .finally(() => {
          if (current) setLoading(false);
        });
    return () => {
      current = false;
    };
  }, [runtime]);
  useEffect(() => {
    if (!ticket) return;
    const timeout = setTimeout(() => {
      void action("expire", async () => {
        await cancelConnection();
        setError("연결 확인이 만료되었습니다. 등록키로 다시 확인하세요.");
      });
    }, 600_000);
    return () => clearTimeout(timeout);
  }, [ticket]);
  async function cancelConnection() {
    setTicket("");
    setConsent(false);
    setKey("");
    if (native) await invoke("cancel_connection");
  }
  function navigate(next: Page, view: ServerView = serverView) {
    if (running.current || loading) return;
    const apply = () => {
      if (
        serverView === "compose" &&
        view === "connection" &&
        status?.endpoint
      ) {
        setEditingConnection(false);
        setConnected(false);
      }
      setPage(next);
      setServerView(view);
      setError("");
      setSuccess("");
      setKey("");
    };
    if (ticket && (next !== "server" || view !== "connection")) {
      void action("cancel", async () => {
        await cancelConnection();
        apply();
      });
    } else apply();
  }
  async function action(
    name: string,
    fn: () => Promise<void>,
  ): Promise<boolean> {
    if (running.current) return false;
    running.current = true;
    setBusy(name);
    setError("");
    setSuccess("");
    try {
      await fn();
      return true;
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      running.current = false;
      setBusy("");
    }
  }
  async function refresh() {
    const result = await invoke<Status>("snapshot", { runtime });
    setStatus(result);
    setCheckedAt(new Date().toLocaleTimeString("ko-KR"));
  }
  async function runCollection() {
    await action("run", async () => {
      const r = await invoke<{ uploaded_count: number }>("set_collection", {
        runtime,
        action: "run",
      });
      await refresh();
      setSuccess(
        r.uploaded_count > 0
          ? `서버가 ${r.uploaded_count}건을 수신했습니다.`
          : "이번 실행에서 서버가 수신한 이벤트는 없습니다. 현재 상태와 대기 건수를 확인하세요.",
      );
    });
  }
  async function stopCollection() {
    await action("disable", async () => {
      await invoke("set_collection", { runtime, action: "disable" });
      await refresh();
      setSuccess(
        "자동 수집을 중지했습니다. 기존 데이터와 서버 설정은 보존됩니다.",
      );
    });
  }
  function checkConnection(e: FormEvent) {
    e.preventDefault();
    setTicket("");
    setConsent(false);
    setConnected(false);
    if (!validOrigin(endpoint)) {
      setError(
        "기본 주소는 https://insights.example.com 형식입니다. 경로와 쿼리를 제외하세요.",
      );
      return;
    }
    if (key.length < 32 || key.length > 4096) {
      setError("서버의 등록키를 입력하세요. 32~4096자가 필요합니다.");
      return;
    }
    if (grafana && !validOrigin(grafana, true)) {
      setError("Grafana의 HTTPS 주소를 확인하세요.");
      return;
    }
    if (!native) {
      setKey("");
      setError("실제 서버 연결은 데스크톱 앱에서 확인할 수 있습니다.");
      return;
    }
    const input = { endpoint, key, runtime, grafanaUrl: grafana };
    setKey("");
    void action("check", async () => {
      const result = await invoke<{ ticket: string }>("check_connection", {
        input,
      });
      setTicket(result.ticket);
      setSuccess(
        "서버, ClickHouse 준비 상태와 등록키를 확인했습니다. 수집 범위를 확인해 주세요.",
      );
    });
  }
  const titles = {
    overview: [
      "이 기기의 상태",
      "Core와 Insights의 상태를 한곳에서 확인하세요.",
    ],
    server: [
      "서버",
      "Insights·ClickHouse·Grafana의 구성과 연결을 한곳에서 관리하세요.",
    ],
    settings: ["설정", "화면 테마와 개인정보 수집을 직접 관리하세요."],
  };
  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark">
            <Activity size={23} />
          </span>
          <div>
            GroundLine<small>Desktop</small>
          </div>
        </div>
        <nav aria-label="주 메뉴">
          {(
            [
              ["overview", Home, "개요"],
              ["server", Server, "서버"],
              ["settings", ShieldCheck, "설정"],
            ] as const
          ).map(([id, Icon, label]) => (
            <button
              key={id}
              className={page === id ? "nav-item selected" : "nav-item"}
              onClick={() => navigate(id)}
              disabled={!!busy || loading}
              aria-current={page === id ? "page" : undefined}
            >
              <Icon size={21} />
              {label}
            </button>
          ))}
        </nav>
        <div className="sidebar-bottom">
          <div className="device">
            <Laptop size={16} />
            {native ? "이 Mac · 데스크톱 앱" : "브라우저 미리보기"}
          </div>
          <div className="theme-picker" role="group" aria-label="화면 테마">
            {(
              [
                ["light", Sun, "라이트"],
                ["system", Monitor, "시스템"],
                ["dark", Moon, "다크"],
              ] as const
            ).map(([id, Icon, label]) => (
              <button
                key={id}
                onClick={() => setTheme(id)}
                aria-pressed={theme === id}
                title={label}
              >
                <Icon size={16} />
                <span>{label}</span>
              </button>
            ))}
          </div>
          <p className="version">GroundLine 0.24.3 · 미리보기</p>
        </div>
      </aside>
      <main aria-busy={!!busy || loading}>
        <header className="page-header">
          <div>
            <h1 ref={heading} tabIndex={-1}>
              {titles[page][0]}
            </h1>
            <p>{titles[page][1]}</p>
          </div>
          {(page !== "server" || serverView === "connection") && (
            <button
              className="secondary header-action"
              disabled={!native || !!busy || loading}
              onClick={() => void action("refresh", refresh)}
            >
              <RefreshCw
                size={16}
                className={busy === "refresh" || loading ? "spin" : ""}
              />
              새로고침
            </button>
          )}
        </header>
        {page === "server" && (
          <div
            className="server-switcher"
            role="group"
            aria-label="서버 설정 작업"
          >
            <button
              aria-pressed={serverView === "connection"}
              disabled={!!busy || loading}
              onClick={() => navigate("server", "connection")}
            >
              <Globe2 size={20} />
              <span>
                <strong>기존 서버 연결</strong>
                <small>서버 연결 · 전송 관리 · 대시보드</small>
              </span>
            </button>
            <button
              aria-pressed={serverView === "compose"}
              disabled={!!busy || loading}
              onClick={() => navigate("server", "compose")}
            >
              <Server size={20} />
              <span>
                <strong>새 서버 구성</strong>
                <small>Docker Compose · ClickHouse · Grafana</small>
              </span>
            </button>
          </div>
        )}
        {(page !== "server" || serverView === "connection") && (
          <div className="environment-bar">
            <label>
              대상 환경{" "}
              <select
                aria-label="대상 환경"
                value={runtime}
                disabled={!!busy || loading}
                onChange={(e) => {
                  const next = e.target.value as Runtime;
                  void action("runtime", async () => {
                    await cancelConnection();
                    setRuntime(next);
                  });
                }}
              >
                <option value="codex_app">Codex App</option>
                <option value="codex_cli">Codex CLI</option>
              </select>
            </label>
            <small>
              {loading
                ? "상태 확인 중…"
                : checkedAt
                  ? `${checkedAt} 확인 · 환경별로 수집 동의를 관리합니다.`
                  : "환경별로 수집 동의를 관리합니다."}
            </small>
          </div>
        )}
        {error && <Notice kind="error">{error}</Notice>}
        {success && <Notice kind="success">{success}</Notice>}
        {busy &&
          ["check", "connect", "run", "core", "export"].includes(busy) && (
            <Notice>
              {
                (
                  {
                    check: "서버와 등록키를 확인하고 있습니다…",
                    connect: "연결 설정과 수집 동의를 저장하고 있습니다…",
                    run: "수집하고 서버 수신 결과를 확인하고 있습니다…",
                    core: "설치된 Core 패키지를 진단하고 있습니다…",
                    export: "설정 파일을 생성하고 있습니다…",
                  } as Record<string, string>
                )[busy]
              }
            </Notice>
          )}
        <div hidden={page !== "server" || serverView !== "compose"}>
          <ServerPage
            setup={setup}
            setSetup={setSetup}
            busy={busy}
            action={action}
            setError={setError}
            setSuccess={setSuccess}
            onConnect={(api, grafana) => {
              navigate("server", "connection");
              if (
                status?.endpoint &&
                new URL(status.endpoint).origin !== new URL(api).origin
              ) {
                setError(
                  "이 환경에는 다른 서버가 저장되어 있습니다. 기존 등록과 대기 데이터를 검토한 뒤 서버를 변경하세요.",
                );
                return;
              }
              setEndpoint(api);
              setGrafana(grafana);
              setEditingConnection(true);
              setConnected(false);
            }}
          />
        </div>
        {page === "server" && serverView === "connection" && (
          <>
            {loading ? (
              <Notice>현재 환경의 연결 상태를 확인하고 있습니다.</Notice>
            ) : status?.endpoint && !editingConnection && !connected ? (
              <ConnectionManager
                key={runtime}
                status={status}
                busy={!!busy}
                native={native}
                repair={() => {
                  setEditingConnection(true);
                  setConnected(false);
                  setEndpoint(status.endpoint || "");
                  setGrafana(status.grafana_url || "");
                  setError("");
                  setSuccess("");
                }}
                settings={() => navigate("settings")}
                run={() => void runCollection()}
                openDashboard={() =>
                  void action("dashboard", async () => {
                    await invoke("open_dashboard");
                  })
                }
                saveDashboard={async (url) => {
                  if (url && !validOrigin(url, true)) {
                    setError("Grafana의 HTTPS 주소를 확인하세요.");
                    return false;
                  }
                  return action("dashboard-save", async () => {
                    await invoke("save_dashboard", { url });
                    await refresh();
                    setSuccess("대시보드 주소를 저장했습니다.");
                  });
                }}
              />
            ) : (
              <>
                {status?.endpoint && (
                  <div className="actions">
                    <button
                      className="text-button"
                      disabled={!!busy}
                      onClick={() =>
                        void action("cancel", async () => {
                          await cancelConnection();
                          setEditingConnection(false);
                          setConnected(false);
                        })
                      }
                    >
                      ← 연결 관리로 돌아가기
                    </button>
                  </div>
                )}
                <div className="stepper">
                  {["서버 확인", "수집 동의", "연결 완료"].map((label, i) => (
                    <div
                      key={label}
                      className={
                        (connected ? 2 : ticket ? 1 : 0) >= i ? "current" : ""
                      }
                    >
                      <span>
                        {(connected ? 2 : ticket ? 1 : 0) > i ? (
                          <Check size={18} />
                        ) : (
                          i + 1
                        )}
                      </span>
                      <strong>{label}</strong>
                    </div>
                  ))}
                </div>
                <div className="content-columns connection-columns">
                  <div>
                    {!ticket && !connected ? (
                      <form
                        className="connection-form"
                        onSubmit={checkConnection}
                      >
                        <Field
                          label="Insights 서버 주소"
                          hint="일반 HTTPS를 지원합니다. Tailscale 주소도 선택해서 사용할 수 있습니다."
                        >
                          <input
                            type="url"
                            required
                            placeholder="https://insights.example.com"
                            value={endpoint}
                            readOnly={!!status?.endpoint}
                            disabled={!!busy}
                            onChange={(e) => setEndpoint(e.target.value)}
                          />
                        </Field>
                        <Field
                          label="Grafana 주소 (선택)"
                          hint="대시보드 접속 주소입니다. 수집기는 Insights API에만 연결합니다."
                        >
                          <input
                            type="url"
                            placeholder="https://grafana.example.com"
                            value={grafana}
                            disabled={!!busy}
                            onChange={(e) => setGrafana(e.target.value)}
                          />
                        </Field>
                        <Field
                          label="기기 등록키"
                          hint="Compose의 ENROLLMENT_TOKEN 값을 입력하세요."
                        >
                          <input
                            type="password"
                            required
                            autoComplete="off"
                            value={key}
                            disabled={!!busy}
                            onChange={(e) => setKey(e.target.value)}
                            placeholder="등록키 입력"
                          />
                        </Field>
                        {status?.endpoint && (
                          <p className="helper">
                            기존 서버 주소는 유지됩니다. 등록키를 확인하고
                            수집에 다시 동의합니다.
                          </p>
                        )}
                        <div className="actions">
                          <button className="primary" disabled={!!busy}>
                            {busy === "check" ? (
                              <RefreshCw className="spin" size={17} />
                            ) : (
                              <ArrowRight size={17} />
                            )}
                            연결 확인
                          </button>
                          <button
                            className="secondary"
                            type="button"
                            disabled={!!busy}
                            onClick={() => {
                              setKey("");
                              setEndpoint(status?.endpoint || "");
                              setGrafana(status?.grafana_url || "");
                              setError("");
                            }}
                          >
                            입력 지우기
                          </button>
                        </div>
                      </form>
                    ) : !connected ? (
                      <div className="consent-panel">
                        <p className="endpoint">전송 대상: {endpoint}</p>
                        <Consent checked={consent} setChecked={setConsent} />
                        <p className="helper">
                          확인 결과는 10분 동안 유효합니다.
                        </p>
                        <div className="actions">
                          <button
                            className="primary"
                            disabled={!consent || !!busy}
                            onClick={() =>
                              void action("connect", async () => {
                                const verifiedTicket = ticket;
                                setTicket("");
                                setConsent(false);
                                await invoke("connect", {
                                  ticket: verifiedTicket,
                                  consent,
                                });
                                setConnected(true);
                                setTicket("");
                                await refresh();
                              })
                            }
                          >
                            동의하고 연결
                            <ArrowRight size={16} />
                          </button>
                          <button
                            className="secondary"
                            disabled={!!busy}
                            onClick={() =>
                              void action("cancel", cancelConnection)
                            }
                          >
                            취소
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div className="connected-panel">
                        <span className="success-icon">
                          <Check size={28} />
                        </span>
                        <h2>설정과 수집 동의를 저장했습니다</h2>
                        <p>
                          첫 전송을 실행하면 서버가 데이터를 수신했는지 확인할
                          수 있습니다.
                        </p>
                        <button
                          className="primary"
                          disabled={!!busy}
                          onClick={() => void runCollection()}
                        >
                          첫 전송 확인
                          <ArrowRight size={16} />
                        </button>
                        <button
                          className="secondary"
                          disabled={!!busy}
                          onClick={() => {
                            setConnected(false);
                            setEditingConnection(false);
                            setSuccess("");
                          }}
                        >
                          연결 관리로 이동
                        </button>
                      </div>
                    )}
                  </div>
                  <aside className="connection-help">
                    <h2>연결 전에 확인해요</h2>
                    {[
                      [Globe2, "HTTPS 연결", "서버의 유효한 TLS 인증서"],
                      [Server, "Insights 서버 준비", "API와 ClickHouse 실행"],
                      [
                        KeyRound,
                        "기기 등록키",
                        "관리자가 발급한 ENROLLMENT_TOKEN",
                      ],
                    ].map(([Icon, title, desc]) => {
                      const I = Icon as typeof Activity;
                      return (
                        <div className="help-item" key={String(title)}>
                          <span>
                            <I size={22} />
                          </span>
                          <div>
                            <strong>{String(title)}</strong>
                            <p>{String(desc)}</p>
                          </div>
                        </div>
                      );
                    })}
                    <p className="helper">
                      Tailscale은 선택 사항입니다. 연결 확인 단계에서는 등록키를
                      저장하거나 수집기를 등록하지 않습니다.
                    </p>
                  </aside>
                </div>
              </>
            )}
          </>
        )}
        {page === "overview" && (
          <Overview
            status={status}
            core={core}
            native={native}
            busy={!!busy || loading}
            navigate={(target) =>
              target === "settings"
                ? navigate("settings")
                : navigate("server", target)
            }
            refresh={() => void action("refresh", refresh)}
            diagnose={() =>
              void action("core", async () => {
                const result = await invoke<CoreStatus>("core_diagnostic");
                setCore(result);
                setSuccess(
                  result.status === "PASS"
                    ? "Core 패키지 무결성과 실행 진단을 통과했습니다. 실제 훅 실행은 별도 확인이 필요합니다."
                    : "Core 진단에서 확인이 필요한 항목이 있습니다.",
                );
              })
            }
          />
        )}
        {page === "settings" && (
          <SettingsPage
            key={runtime}
            status={status}
            theme={theme}
            setTheme={setTheme}
            busy={!!busy || loading}
            native={native}
            stop={() => void stopCollection()}
            connect={() => navigate("server", "connection")}
            resume={() =>
              action("resume", async () => {
                await invoke("resume_collection", { runtime, consent: true });
                await refresh();
                setSuccess(
                  "수집 동의를 저장하고 자동 수집을 다시 켰습니다. 서버 메뉴에서 전송 결과를 확인하세요.",
                );
              })
            }
          />
        )}
        {!native && (
          <div className="preview-note">
            <CircleHelp size={15} />
            브라우저는 화면 미리보기입니다. 서버 연결·상태 진단·파일 생성은
            데스크톱 앱에서 실행됩니다.
          </div>
        )}
      </main>
    </div>
  );
}
