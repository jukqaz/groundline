import { Theme as DesignTheme } from "@radix-ui/themes";
import "@radix-ui/themes/styles.css";
import React, {
  useEffect,
  useEffectEvent,
  useRef,
  useState,
  type FormEvent,
} from "react";
import { invoke, isTauri } from "@tauri-apps/api/core";
import { setTheme as setNativeTheme } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import {
  Activity,
  ArrowRight,
  Check,
  CircleHelp,
  Home,
  RefreshCw,
  Server,
  Settings2,
} from "lucide-react";
import {
  errorMessage,
  initialSetup,
  type Setup,
  resolveTheme,
  storedTheme,
  validOrigin,
  validDashboardOrigin,
  type Runtime,
  type Theme,
  type Status,
  defaultPreferences,
  type AppPreferences,
} from "./model";
import { Button, Input, Choice, Field, Notice } from "./ui";
import {
  Overview,
  ConnectionManager,
  SettingsPage,
  Consent,
} from "./work-pages";
import { ServerPage } from "./server-page";
import { ConnectionImport } from "./operations";
import { version } from "../package.json";
import "./style.css";

const native = isTauri();
const nativeMac = native && navigator.platform.startsWith("Mac");
type Page = "overview" | "server" | "settings";
type ServerView = "connection" | "compose";
type ConnectionSection = "collection" | "dashboard";
export function App() {
  const [page, setPage] = useState<Page>("overview");
  const [serverView, setServerView] = useState<ServerView>("connection");
  const [connectionSection, setConnectionSection] =
    useState<ConnectionSection | null>(null);
  const [setup, setSetup] = useState<Setup>(initialSetup);
  const [theme, setTheme] = useState<Theme>(() => {
    try {
      return storedTheme(localStorage.getItem("groundline-theme"));
    } catch {
      return "system";
    }
  });
  const [appearance, setAppearance] = useState<"light" | "dark">(() =>
    resolveTheme(theme, matchMedia("(prefers-color-scheme: dark)").matches),
  );
  const [runtime, setRuntime] = useState<Runtime>("codex_app");
  const [preferences, setPreferences] =
    useState<AppPreferences>(defaultPreferences);
  const [preferencesLoaded, setPreferencesLoaded] = useState(!native);
  const [preferencesError, setPreferencesError] = useState("");
  const [themeError, setThemeError] = useState("");
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState("");
  const [loading, setLoading] = useState(native);
  const [editingConnection, setEditingConnection] = useState(false);
  const running = useRef(false);
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
  const [statusReadFailed, setStatusReadFailed] = useState(false);
  const navigateFromTray = useEffectEvent((payload: string) => {
    if (payload === "collection" || payload === "dashboard")
      navigate("server", "connection", payload);
    else if (["overview", "server", "settings"].includes(payload))
      navigate(payload as Page);
  });
  useEffect(() => {
    if (!native) return;
    let disposed = false;
    let unsubscribe: (() => void) | undefined;
    void listen<string>("groundline:navigate", ({ payload }) => {
      navigateFromTray(payload);
    })
      .then((fn) => {
        if (disposed) fn();
        else unsubscribe = fn;
      })
      .catch(() => {});
    return () => {
      disposed = true;
      unsubscribe?.();
    };
  }, []);
  useEffect(() => {
    if (!native) return;
    let current = true;
    invoke<AppPreferences>("get_app_preferences")
      .then((value) => {
        if (current) {
          setPreferences(value);
          setRuntime(value.runtime);
        }
      })
      .catch((error) => {
        if (current) setPreferencesError(errorMessage(error));
      })
      .finally(() => {
        if (current) setPreferencesLoaded(true);
      });
    return () => {
      current = false;
    };
  }, []);
  useEffect(() => {
    heading.current?.focus({ preventScroll: true });
    window.scrollTo({ top: 0 });
    if (page === "server" && serverView === "connection" && connectionSection) {
      const section = document.getElementById(connectionSection);
      section?.scrollIntoView({ block: "start" });
      section?.focus({ preventScroll: true });
    }
  }, [page, serverView, connectionSection]);
  useEffect(() => {
    if (error) window.scrollTo({ top: 0 });
  }, [error]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const resolved = resolveTheme(theme, media.matches);
      document.documentElement.dataset.theme = resolved;
      document.documentElement.style.colorScheme = resolved;
      setAppearance(resolved);
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
    if (!native) return;
    let current = true;
    // Keep native titles and menus in sync. Null restores OS theme tracking.
    void setNativeTheme(theme === "system" ? null : theme)
      .then(() => {
        if (current) setThemeError("");
      })
      .catch(() => {
        if (current)
          setThemeError(
            "창 테마를 적용하지 못했습니다. 테마를 다시 선택하세요.",
          );
      });
    return () => {
      current = false;
    };
  }, [theme]);
  useEffect(() => {
    if (!preferencesLoaded) return;
    let current = true;
    setStatus(null);
    setStatusReadFailed(false);
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
    if (native)
      invoke<Status>("snapshot", { runtime })
        .then((value) => {
          if (current) {
            setStatus(value);
            setEndpoint(value.endpoint || "");
            setGrafana(value.grafana_url || "");
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
  }, [runtime, preferencesLoaded]);
  useEffect(() => {
    if (!native || !preferencesLoaded) return;
    let current = true;
    let refreshing = false;
    const update = async () => {
      if (document.hidden || running.current || refreshing) return;
      refreshing = true;
      try {
        const value = await invoke<Status>("snapshot", { runtime });
        if (current) {
          setStatus(value);
          setStatusReadFailed(false);
        }
      } catch {
        if (current) setStatusReadFailed(true);
      } finally {
        refreshing = false;
      }
    };
    const timer = setInterval(() => void update(), 10_000);
    window.addEventListener("focus", update);
    document.addEventListener("visibilitychange", update);
    return () => {
      current = false;
      clearInterval(timer);
      window.removeEventListener("focus", update);
      document.removeEventListener("visibilitychange", update);
    };
  }, [runtime, preferencesLoaded]);
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
  function navigate(
    next: Page,
    view: ServerView = "connection",
    section: ConnectionSection | null = null,
  ) {
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
      setConnectionSection(section);
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
    setStatusReadFailed(false);
  }
  async function savePreferences(patch: Partial<AppPreferences>) {
    await action("preferences", async () => {
      const value = await invoke<AppPreferences>("save_app_preferences", {
        preferences: { ...preferences, ...patch },
      });
      setPreferences(value);
      if (value.runtime !== runtime) setRuntime(value.runtime);
      setSuccess("앱 실행 설정을 저장했습니다.");
    });
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
  function resumeCollection() {
    return action("resume", async () => {
      await invoke("resume_collection", { runtime, consent: true });
      await refresh();
      setSuccess("수집 동의를 저장하고 자동 수집을 다시 켰습니다.");
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
    if (grafana && !validDashboardOrigin(grafana)) {
      setError("Grafana 주소를 확인하세요. 외부 서버는 HTTPS가 필요합니다.");
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
  const titles = { overview: "개요", server: "서버", settings: "설정" };
  return (
    <DesignTheme
      appearance={appearance}
      accentColor="teal"
      grayColor="gray"
      radius="medium"
      panelBackground="solid"
      className="groundline-theme"
      data-native-macos={nativeMac || undefined}
    >
      {nativeMac && (
        <div className="window-titlebar" data-tauri-drag-region>
          GroundLine Desktop
        </div>
      )}
      <div className="shell">
        <aside className="sidebar">
          <div className="brand">
            <span className="brand-mark">
              <Activity size={30} />
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
                ["settings", Settings2, "설정"],
              ] as const
            ).map(([id, Icon, label]) => (
              <Button
                key={id}
                className={page === id ? "nav-item selected" : "nav-item"}
                onClick={() => navigate(id)}
                disabled={!!busy || loading}
                aria-current={page === id ? "page" : undefined}
              >
                <Icon size={18} />
                {label}
              </Button>
            ))}
          </nav>
          <div className="sidebar-bottom">
            <p className="version">v{version}</p>
          </div>
        </aside>
        <main aria-busy={!!busy || loading}>
          <header className="page-header">
            <h1 ref={heading} tabIndex={-1}>
              {titles[page]}
            </h1>
            <div className="header-controls">
              {(page === "overview" ||
                (page === "server" && serverView === "connection")) && (
                <div className="environment-bar">
                  <label>
                    대상 환경{" "}
                    <Choice
                      label="대상 환경"
                      value={runtime}
                      disabled={!!busy || loading}
                      options={[
                        ["codex_app", "Codex App"],
                        ["codex_cli", "Codex CLI"],
                      ]}
                      onValueChange={(value) => {
                        const next = value as Runtime;
                        void action("runtime", async () => {
                          await cancelConnection();
                          const value = await invoke<AppPreferences>(
                            "save_app_preferences",
                            { preferences: { ...preferences, runtime: next } },
                          );
                          setPreferences(value);
                          setRuntime(value.runtime);
                        });
                      }}
                    />
                  </label>
                </div>
              )}
              {(page === "overview" ||
                (page === "server" && serverView === "connection")) && (
                <Button
                  className="secondary header-action"
                  disabled={!native || !!busy || loading}
                  onClick={() => void action("refresh", refresh)}
                >
                  <RefreshCw
                    size={16}
                    className={busy === "refresh" || loading ? "spin" : ""}
                  />
                  새로고침
                </Button>
              )}
            </div>
          </header>
          <div className="page-body">
            {page === "server" && serverView === "compose" && (
              <div className="server-back">
                <Button
                  className="text-button"
                  disabled={!!busy || loading}
                  onClick={() => navigate("server", "connection")}
                >
                  ← 연결 관리
                </Button>
              </div>
            )}
            {error && <Notice kind="error">{error}</Notice>}
            {themeError && <Notice kind="error">{themeError}</Notice>}
            {statusReadFailed && (
              <Notice kind="error">
                상태 갱신에 실패했습니다. 아래 정보는 마지막 확인 기록입니다.
                다시 상태를 확인하세요.
              </Notice>
            )}
            {success && (
              <Notice kind="success" onDismiss={() => setSuccess("")}>
                {success}
              </Notice>
            )}
            {busy && ["check", "connect", "run", "export"].includes(busy) && (
              <Notice>
                {
                  (
                    {
                      check: "서버와 등록키를 확인하고 있습니다…",
                      connect: "연결 설정과 수집 동의를 저장하고 있습니다…",
                      run: "수집하고 서버 수신 결과를 확인하고 있습니다…",
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
                    runtime={runtime}
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
                    stop={() => void stopCollection()}
                    resume={resumeCollection}
                    run={() => void runCollection()}
                    openDashboard={() =>
                      void action("dashboard", async () => {
                        await invoke("open_dashboard");
                      })
                    }
                    saveDashboard={async (url) => {
                      if (url && !validDashboardOrigin(url)) {
                        setError(
                          "Grafana 주소를 확인하세요. 외부 서버는 HTTPS가 필요합니다.",
                        );
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
                        <Button
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
                        </Button>
                      </div>
                    )}
                    <div className="stepper">
                      {["서버 확인", "수집 동의", "연결 완료"].map(
                        (label, i) => (
                          <div
                            key={label}
                            className={
                              (connected ? 2 : ticket ? 1 : 0) >= i
                                ? "current"
                                : ""
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
                        ),
                      )}
                    </div>
                    <div className="connection-content">
                      <div>
                        {!ticket && !connected ? (
                          <form
                            className="connection-form"
                            onSubmit={checkConnection}
                          >
                            <ConnectionImport
                              key={runtime}
                              disabled={!!busy}
                              apply={async (value) => {
                                if (
                                  status?.endpoint &&
                                  new URL(status.endpoint).origin !==
                                    new URL(value.api_url).origin
                                )
                                  throw new Error(
                                    "endpoint_change_requires_review",
                                  );
                                const applied = await action(
                                  "import",
                                  async () => {
                                    await cancelConnection();
                                    setKey("");
                                    setEndpoint(value.api_url);
                                    setGrafana(value.grafana_url);
                                  },
                                );
                                if (!applied)
                                  throw new Error("connection_import_failed");
                              }}
                            />
                            <Field
                              label="Insights 서버 주소"
                              hint="일반 HTTPS를 지원합니다. Tailscale 주소도 선택해서 사용할 수 있습니다."
                            >
                              <Input
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
                              hint="외부 서버는 HTTPS, 이 기기의 localhost는 HTTP도 지원합니다."
                            >
                              <Input
                                type="url"
                                placeholder="https://grafana.example.com"
                                value={grafana}
                                disabled={!!busy}
                                onChange={(e) => setGrafana(e.target.value)}
                              />
                            </Field>
                            <Field
                              label="기기 등록키"
                              hint="서버 관리자가 제공한 Insights 등록키를 입력하세요."
                            >
                              <Input
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
                              <Button className="primary" disabled={!!busy}>
                                {busy === "check" ? (
                                  <RefreshCw className="spin" size={17} />
                                ) : (
                                  <ArrowRight size={17} />
                                )}
                                연결 확인
                              </Button>
                              <Button
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
                              </Button>
                            </div>
                          </form>
                        ) : !connected ? (
                          <div className="consent-panel">
                            <p className="endpoint">전송 대상: {endpoint}</p>
                            <Consent
                              checked={consent}
                              setChecked={setConsent}
                            />
                            <p className="helper">
                              확인 결과는 10분 동안 유효합니다.
                            </p>
                            <div className="actions">
                              <Button
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
                              </Button>
                              <Button
                                className="secondary"
                                disabled={!!busy}
                                onClick={() =>
                                  void action("cancel", cancelConnection)
                                }
                              >
                                취소
                              </Button>
                            </div>
                          </div>
                        ) : (
                          <div className="connected-panel">
                            <span className="success-icon">
                              <Check size={28} />
                            </span>
                            <h2>설정과 수집 동의를 저장했습니다</h2>
                            <p>
                              첫 전송을 실행하면 서버가 데이터를 수신했는지
                              확인할 수 있습니다.
                            </p>
                            <Button
                              className="primary"
                              disabled={!!busy}
                              onClick={() => void runCollection()}
                            >
                              첫 전송 확인
                              <ArrowRight size={16} />
                            </Button>
                            <Button
                              className="secondary"
                              disabled={!!busy}
                              onClick={() => {
                                setConnected(false);
                                setEditingConnection(false);
                                setSuccess("");
                              }}
                            >
                              연결 관리로 이동
                            </Button>
                          </div>
                        )}
                      </div>
                    </div>
                  </>
                )}
              </>
            )}
            {page === "server" && serverView === "connection" && (
              <div className="server-tools">
                <Button
                  className="text-button"
                  disabled={!!busy || loading}
                  onClick={() => navigate("server", "compose")}
                >
                  <Server size={15} />새 서버 구성
                </Button>
              </div>
            )}
            {page === "overview" && (
              <Overview
                runtime={runtime}
                dashboard={() =>
                  status?.grafana_url
                    ? void action("dashboard", async () => {
                        await invoke("open_dashboard");
                      })
                    : navigate("server", "connection", "dashboard")
                }
                status={status}
                native={native}
                busy={!!busy || loading}
                navigate={(target) =>
                  navigate(
                    "server",
                    "connection",
                    target === "collection" ? "collection" : null,
                  )
                }
                refresh={() => void action("refresh", refresh)}
              />
            )}
            {page === "settings" && (
              <SettingsPage
                setAlerts={(alerts_enabled) =>
                  void savePreferences({ alerts_enabled })
                }
                theme={theme}
                setTheme={setTheme}
                preferences={preferences}
                preferencesError={preferencesError}
                setCloseAction={(close_action) =>
                  void savePreferences({ close_action })
                }
                busy={!!busy || loading}
                native={native}
              />
            )}
            {!native && (
              <div className="preview-note">
                <CircleHelp size={15} />
                브라우저는 화면 미리보기입니다. 서버 연결·상태 진단·파일 생성은
                데스크톱 앱에서 실행됩니다.
              </div>
            )}
          </div>
        </main>
      </div>
    </DesignTheme>
  );
}
