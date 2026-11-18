import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Props {
  open: boolean;
  onClose: () => void;
}

interface RemoteInfo {
  enabled: boolean;
  running: boolean;
  port: number;
  pair_code: string;
  addresses: string[];
  error: string;
}

interface ShortcutInfo {
  shortcut: string;
  button_id: string;
  button_name: string;
}

interface AppState {
  gamingMode: boolean;
  mode: string;
  paused: boolean;
  keyboardRunning: boolean;
  keyboardError: string | null;
  shortcuts: ShortcutInfo[];
  shortcutCount: number;
  triggers: number;
  eventsSeen: number;
  boundProfileName: string | null;
  configError: string | null;
  uptimeSeconds: number;
}

export function SettingsPanel({ open, onClose }: Props) {
  const [remote, setRemote] = useState<RemoteInfo | null>(null);
  const [state, setState] = useState<AppState | null>(null);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");

  const refresh = useCallback(async () => {
    try {
      const [remoteInfo, appState] = await Promise.all([
        invoke<RemoteInfo>("get_remote_info"),
        invoke<AppState>("get_app_state"),
      ]);
      setRemote(remoteInfo);
      setState(appState);
    } catch (e) {
      setMessage(String(e));
    }
  }, []);

  useEffect(() => {
    if (open) void refresh();
  }, [open, refresh]);

  useEffect(() => {
    if (!open) return;
    const timer = setInterval(() => void refresh(), 3000);
    return () => clearInterval(timer);
  }, [open, refresh]);

  if (!open) return null;

  const toggleRemote = async () => {
    if (!remote) return;
    setBusy("remote");
    setMessage("");
    try {
      const info = await invoke<RemoteInfo>("set_remote_enabled", {
        enabled: !remote.running,
      });
      setRemote(info);
      setMessage(info.running ? "Phone connection is on" : "Phone connection is off");
    } catch (e) {
      setMessage(String(e));
      await refresh();
    } finally {
      setBusy("");
    }
  };

  const newCode = async () => {
    setBusy("code");
    try {
      setRemote(await invoke<RemoteInfo>("regenerate_pairing"));
      setMessage("New pairing code generated — pair the phone again");
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy("");
    }
  };

  const setPaused = async (paused: boolean) => {
    setBusy("keyboard");
    setMessage("");
    try {
      setState(await invoke<AppState>("set_shortcuts_paused", { paused }));
      setMessage(paused ? "Shortcuts paused — keys type normally" : "Shortcuts resumed");
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy("");
    }
  };

  const reload = async () => {
    setBusy("keyboard");
    setMessage("");
    try {
      setState(await invoke<AppState>("reload_shortcuts"));
      setMessage("Shortcuts re-registered from the active profile");
    } catch (e) {
      setMessage(String(e));
    } finally {
      setBusy("");
    }
  };

  const address = remote?.addresses[0];
  const engineHealthy = state?.keyboardRunning && !state?.keyboardError;

  return (
    <div
      className="fixed inset-0 z-[90] flex justify-end bg-black/40"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-[440px] h-full bg-[var(--color-bg-secondary)] border-l border-[var(--color-border)] shadow-2xl flex flex-col animate-slideRight">
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--color-border)]">
          <h2 className="text-base font-semibold text-[var(--color-text-primary)]">
            🔧 Settings
          </h2>
          <button
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>

        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-6">
          {/* Native keyboard engine */}
          <section className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider">
                KEYBOARD ENGINE (NATIVE)
              </label>
              <div className="flex items-center gap-1.5 text-[10px]">
                <span
                  className={`w-2 h-2 rounded-full ${
                    engineHealthy
                      ? state?.paused
                        ? "bg-[var(--color-warning)]"
                        : "bg-[var(--color-success)]"
                      : "bg-[var(--color-danger)]"
                  }`}
                />
                <span className="text-[var(--color-text-secondary)]">
                  {state?.keyboardRunning
                    ? state.paused
                      ? "Paused"
                      : "Hooked"
                    : "Stopped"}
                </span>
              </div>
            </div>

            <p className="text-xs text-[var(--color-text-secondary)] leading-relaxed">
              The numpad is captured by the Rust backend — no AutoHotkey is
              involved. Bound keys are consumed; everything else types normally.
            </p>

            <div className="rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] p-3 space-y-3">
              <div className="grid grid-cols-2 gap-3">
                <Field label="MODE">
                  <div className="font-mono text-sm text-[var(--color-text-primary)]">
                    {state?.mode ?? "—"}
                  </div>
                </Field>
                <Field label="BOUND PROFILE">
                  <div className="text-sm text-[var(--color-text-primary)]">
                    {state?.boundProfileName ?? "—"}
                  </div>
                </Field>
              </div>

              <Field label={`REGISTERED SHORTCUTS (${state?.shortcutCount ?? 0})`}>
                <div className="space-y-1 max-h-40 overflow-y-auto">
                  {(state?.shortcuts ?? []).length === 0 && (
                    <p className="text-[10px] text-[var(--color-text-muted)]">
                      No shortcuts in the active profile.
                    </p>
                  )}
                  {(state?.shortcuts ?? []).map((shortcut) => (
                    <div
                      key={`${shortcut.shortcut}-${shortcut.button_id}`}
                      className="flex items-center justify-between gap-2 px-2 py-1 rounded bg-[var(--color-bg-primary)] border border-[var(--color-border)]"
                    >
                      <span className="text-[10px] font-mono text-[var(--color-accent)]">
                        {shortcut.shortcut}
                      </span>
                      <span className="text-[10px] text-[var(--color-text-secondary)] truncate">
                        {shortcut.button_name}
                      </span>
                    </div>
                  ))}
                </div>
              </Field>

              {state?.keyboardError && (
                <p className="text-[10px] text-[var(--color-danger)]">
                  {state.keyboardError}
                </p>
              )}
              {state?.configError && (
                <p className="text-[10px] text-[var(--color-warning)]">
                  {state.configError}
                </p>
              )}

              <div className="flex gap-2">
                <button
                  onClick={() => void setPaused(!state?.paused)}
                  disabled={busy === "keyboard" || !state?.keyboardRunning}
                  className="flex-1 px-3 py-2 rounded-lg text-xs font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer disabled:opacity-40"
                >
                  {state?.paused ? "Resume shortcuts" : "Pause shortcuts"}
                </button>
                <button
                  onClick={() => void reload()}
                  disabled={busy === "keyboard"}
                  className="px-3 py-2 rounded-lg text-xs text-[var(--color-text-secondary)] border border-[var(--color-border)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer disabled:opacity-40"
                >
                  Reload
                </button>
              </div>

              <p className="text-[10px] text-[var(--color-text-muted)] font-mono">
                keys seen {state?.eventsSeen ?? 0} · triggers {state?.triggers ?? 0}
              </p>
            </div>
          </section>

          {/* Phone remote */}
          <section className="space-y-3">
            <div className="flex items-center justify-between">
              <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider">
                PHONE REMOTE
              </label>
              <div className="flex items-center gap-1.5 text-[10px]">
                <span
                  className={`w-2 h-2 rounded-full ${remote?.running ? "bg-[var(--color-success)]" : "bg-[var(--color-danger)]"}`}
                />
                <span className="text-[var(--color-text-secondary)]">
                  {remote?.running ? "Listening" : "Off"}
                </span>
              </div>
            </div>

            <p className="text-xs text-[var(--color-text-secondary)] leading-relaxed">
              Install the <b className="text-[var(--color-text-primary)]">AYMAN DECK</b> APK
              on your phone, keep it on the same WiFi, and enter the address and
              pairing code below.
            </p>

            <div className="rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] p-3 space-y-3">
              <Field label="PC ADDRESS">
                <div className="font-mono text-sm text-[var(--color-text-primary)] select-text">
                  {remote?.running && address
                    ? `${address}:${remote.port}`
                    : "not available"}
                </div>
              </Field>
              <Field label="PAIRING CODE">
                <div className="flex items-center gap-2">
                  <div className="font-mono text-2xl tracking-[0.3em] text-[var(--color-accent)] select-text">
                    {remote?.pair_code ?? "------"}
                  </div>
                  <button
                    onClick={newCode}
                    disabled={busy === "code"}
                    className="px-2 py-1 rounded-lg bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer disabled:opacity-40"
                  >
                    new code
                  </button>
                </div>
              </Field>
              {remote && remote.addresses.length > 1 && (
                <p className="text-[10px] text-[var(--color-text-muted)] font-mono">
                  other addresses: {remote.addresses.slice(1).join(", ")}
                </p>
              )}
              {remote?.error && (
                <p className="text-[10px] text-[var(--color-warning)]">
                  {remote.error}
                </p>
              )}
              <button
                onClick={toggleRemote}
                disabled={busy === "remote"}
                className={`w-full px-4 py-2 rounded-lg text-sm font-medium transition-colors cursor-pointer disabled:opacity-40 ${
                  remote?.running
                    ? "bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-danger)]"
                    : "bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white"
                }`}
              >
                {remote?.running ? "Turn phone remote off" : "Turn phone remote on"}
              </button>
              <p className="text-[10px] text-[var(--color-text-muted)]">
                Home network only. Windows may ask once for firewall access —
                allow it on private networks.
              </p>
            </div>
          </section>

          {message && (
            <p className="text-[11px] text-[var(--color-text-secondary)]">{message}</p>
          )}
        </div>

        <div className="px-5 py-3 border-t border-[var(--color-border)] bg-[var(--color-bg-primary)]">
          <button
            onClick={() => void refresh()}
            className="w-full px-4 py-2 rounded-lg text-xs text-[var(--color-text-secondary)] border border-[var(--color-border)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer"
          >
            ⟳ Refresh status
          </button>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1 block">
        {label}
      </label>
      {children}
    </div>
  );
}
