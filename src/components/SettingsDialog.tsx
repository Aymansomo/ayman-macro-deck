import { useCallback, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getStartWithWindows, openLogsFolder, setStartWithWindows } from "../lib/backend";
import { useAppStore } from "../store";
import { DeviceManagerContent } from "./DeviceManagerContent";
import { Icon, type IconName } from "./Icon";
import { ProfilesContent } from "./ProfilesContent";
import { SteamAccountsContent } from "./SteamAccountsContent";
import { UpdateButton } from "./UpdateButton";

export type SettingsTab =
  | "engine"
  | "keypad"
  | "phone"
  | "steam"
  | "profiles"
  | "app";

interface Props {
  open: boolean;
  tab: SettingsTab;
  onTabChange: (tab: SettingsTab) => void;
  onClose: () => void;
  selectedButtonId: string | null;
  onAssignKey: (keyId: string) => boolean;
}

interface TabMeta {
  id: SettingsTab;
  label: string;
  icon: IconName;
  title: string;
  description: string;
}

const TABS: TabMeta[] = [
  {
    id: "engine",
    label: "Keyboard engine",
    icon: "power",
    title: "Keyboard engine",
    description:
      "Native numpad capture, the shortcuts it is listening for and the live counters.",
  },
  {
    id: "keypad",
    label: "Macro keypad",
    icon: "keyboard",
    title: "Macro keypad",
    description: "Choose the physical keyboard that works as your macro deck.",
  },
  {
    id: "phone",
    label: "Phone remote",
    icon: "phone",
    title: "Phone remote",
    description: "Press any deck key from your phone over the local network.",
  },
  {
    id: "steam",
    label: "Steam accounts",
    icon: "gamepad",
    title: "Steam accounts",
    description:
      "Shortcuts used by the Switch Steam Account action, matched by account name.",
  },
  {
    id: "profiles",
    label: "Profiles & config",
    icon: "layers",
    title: "Profiles & config",
    description:
      "Create and switch profiles, then export, import or restore your configuration.",
  },
  {
    id: "app",
    label: "App & updates",
    icon: "info",
    title: "App & updates",
    description: "Version, update checks, logs and integrations.",
  },
];

export function SettingsDialog({
  open,
  tab,
  onTabChange,
  onClose,
  selectedButtonId,
  onAssignKey,
}: Props) {
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setVersion)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") onClose();
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, onClose]);

  if (!open) return null;

  const active = TABS.find((meta) => meta.id === tab) ?? TABS[0];

  return (
    <div
      className="modal-overlay"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="settings-dialog" role="dialog" aria-modal="true" aria-label="Settings">
        <aside className="settings-rail">
          <div className="settings-rail-head">
            <span className="brand-mark">AM</span>
            <div className="min-w-0">
              <div className="settings-rail-title">Settings</div>
              <div className="settings-rail-sub">AYMAN MACRO DECK</div>
            </div>
          </div>

          <nav className="settings-tabs">
            {TABS.map((meta) => (
              <button
                key={meta.id}
                type="button"
                onClick={() => onTabChange(meta.id)}
                className={`settings-tab ${meta.id === active.id ? "is-active" : ""}`}
              >
                <Icon name={meta.icon} size={16} />
                <span className="truncate">{meta.label}</span>
              </button>
            ))}
          </nav>

          <div className="settings-rail-foot">
            <span className="status-dot" style={{ background: "var(--color-success)" }} />
            <span>No AutoHotkey · native Rust</span>
            {version && <span className="settings-version">v{version}</span>}
          </div>
        </aside>

        <div className="settings-content">
          <header className="settings-content-head">
            <div className="min-w-0">
              <h2 className="settings-title">{active.title}</h2>
              <p className="settings-subtitle">{active.description}</p>
            </div>
            <button
              type="button"
              onClick={onClose}
              className="icon-button"
              title="Close settings (Esc)"
            >
              <Icon name="close" size={16} />
            </button>
          </header>

          <div className="settings-body">
            {active.id === "engine" && <EngineSection />}
            {active.id === "keypad" && (
              <DeviceManagerContent
                selectedButtonId={selectedButtonId}
                onAssignKey={onAssignKey}
              />
            )}
            {active.id === "phone" && <PhoneSection />}
            {active.id === "steam" && <SteamAccountsContent />}
            {active.id === "profiles" && <ProfilesContent />}
            {active.id === "app" && <AppSection version={version} />}
          </div>
        </div>
      </div>
    </div>
  );
}

/* ------------------------------------------------------------------ shared */

function Stat({
  label,
  value,
  mono = false,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="settings-stat">
      <div className="ui-label">{label}</div>
      <div className={`settings-stat-value ${mono ? "font-mono" : ""}`} title={value}>
        {value}
      </div>
    </div>
  );
}

function SectionCard({
  title,
  action,
  children,
}: {
  title: string;
  action?: React.ReactNode;
  children: React.ReactNode;
}) {
  return (
    <section className="settings-card">
      <div className="settings-card-head">
        <div className="ui-label">{title}</div>
        {action}
      </div>
      {children}
    </section>
  );
}

/* ----------------------------------------------------------------- engine */

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

interface ShortcutInfo {
  shortcut: string;
  button_id: string;
  button_name: string;
}

function EngineSection() {
  const [state, setState] = useState<AppState | null>(null);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");

  const refresh = useCallback(async () => {
    try {
      setState(await invoke<AppState>("get_app_state"));
    } catch (error) {
      setMessage(String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 3000);
    return () => clearInterval(timer);
  }, [refresh]);

  const setPaused = async (paused: boolean) => {
    setBusy("keyboard");
    setMessage("");
    try {
      setState(await invoke<AppState>("set_shortcuts_paused", { paused }));
      setMessage(
        paused ? "Shortcuts paused — keys type normally" : "Shortcuts resumed",
      );
    } catch (error) {
      setMessage(String(error));
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
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const healthy = Boolean(state?.keyboardRunning) && !state?.keyboardError;
  const tone = !healthy ? "is-bad" : state?.paused ? "is-warn" : "is-ok";
  const statusText = !state
    ? "Reading engine state…"
    : !state.keyboardRunning
      ? "Engine stopped"
      : state.paused
        ? "Shortcuts paused"
        : "Listening for keypad keys";
  const shortcuts = state?.shortcuts ?? [];

  return (
    <div className="space-y-4">
      <div className={`settings-hero ${tone}`}>
        <div className="settings-hero-icon">
          <Icon name="keyboard" size={22} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="settings-hero-title">{statusText}</div>
          <div className="settings-hero-sub">
            Windows low-level keyboard hook in Rust. Bound keys are consumed,
            everything else types normally.
          </div>
        </div>
        <button
          type="button"
          onClick={() => void setPaused(!state?.paused)}
          disabled={busy === "keyboard" || !state?.keyboardRunning}
          className={`ui-btn ${state?.paused ? "ui-btn-primary" : "ui-btn-ghost"}`}
        >
          {state?.paused ? "Resume" : "Pause"}
        </button>
      </div>

      <div className="settings-stats">
        <Stat label="MODE" value={state?.mode ?? "—"} />
        <Stat label="BOUND PROFILE" value={state?.boundProfileName ?? "—"} />
        <Stat label="KEYS SEEN" value={String(state?.eventsSeen ?? 0)} mono />
        <Stat label="TRIGGERS" value={String(state?.triggers ?? 0)} mono />
      </div>

      <SectionCard
        title={`Registered shortcuts (${state?.shortcutCount ?? 0})`}
        action={
          <button
            type="button"
            onClick={() => void reload()}
            disabled={busy === "keyboard"}
            className="ui-btn ui-btn-ghost ui-btn-sm"
          >
            <Icon name="refresh" size={13} />
            Reload
          </button>
        }
      >
        {shortcuts.length === 0 ? (
          <p className="settings-empty">
            No shortcuts in the active profile. Assign actions to keys on the
            deck and they appear here.
          </p>
        ) : (
          <div className="shortcut-list">
            {shortcuts.map((shortcut) => (
              <div
                key={`${shortcut.shortcut}-${shortcut.button_id}`}
                className="shortcut-row"
              >
                <kbd className="shortcut-key">{shortcut.shortcut}</kbd>
                <span className="shortcut-name" title={shortcut.button_name}>
                  {shortcut.button_name}
                </span>
              </div>
            ))}
          </div>
        )}
      </SectionCard>

      {state?.keyboardError && (
        <p className="settings-error">
          <Icon name="alert" size={14} /> {state.keyboardError}
        </p>
      )}
      {state?.configError && (
        <p className="settings-warning">
          <Icon name="alert" size={14} /> {state.configError}
        </p>
      )}
      {message && <p className="settings-message">{message}</p>}
    </div>
  );
}

/* ------------------------------------------------------------------ phone */

interface RemoteInfo {
  enabled: boolean;
  running: boolean;
  port: number;
  pair_code: string;
  addresses: string[];
  error: string;
}

function PhoneSection() {
  const [remote, setRemote] = useState<RemoteInfo | null>(null);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");

  const refresh = useCallback(async () => {
    try {
      setRemote(await invoke<RemoteInfo>("get_remote_info"));
    } catch (error) {
      setMessage(String(error));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 3000);
    return () => clearInterval(timer);
  }, [refresh]);

  const toggle = async () => {
    if (!remote) return;
    setBusy("remote");
    setMessage("");
    try {
      const info = await invoke<RemoteInfo>("set_remote_enabled", {
        enabled: !remote.running,
      });
      setRemote(info);
      setMessage(
        info.running ? "Phone connection is on" : "Phone connection is off",
      );
    } catch (error) {
      setMessage(String(error));
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
    } catch (error) {
      setMessage(String(error));
    } finally {
      setBusy("");
    }
  };

  const address = remote?.addresses[0];

  return (
    <div className="space-y-4">
      <div className={`settings-hero ${remote?.running ? "is-ok" : "is-bad"}`}>
        <div className="settings-hero-icon">
          <Icon name="phone" size={22} />
        </div>
        <div className="min-w-0 flex-1">
          <div className="settings-hero-title">
            {remote?.running ? "Phone remote is on" : "Phone remote is off"}
          </div>
          <div className="settings-hero-sub">
            Install the AYMAN DECK app on your phone, keep it on the same Wi-Fi
            and enter the address and pairing code below.
          </div>
        </div>
        <button
          type="button"
          onClick={() => void toggle()}
          disabled={busy === "remote"}
          className={`ui-btn ${remote?.running ? "ui-btn-ghost" : "ui-btn-primary"}`}
        >
          {remote?.running ? "Turn off" : "Turn on"}
        </button>
      </div>

      <div className="settings-stats">
        <Stat
          label="PC ADDRESS"
          value={
            remote?.running && address ? `${address}:${remote.port}` : "not available"
          }
          mono
        />
        <div className="settings-stat">
          <div className="ui-label">PAIRING CODE</div>
          <div className="flex items-center gap-2">
            <span className="settings-pair-code">{remote?.pair_code ?? "------"}</span>
            <button
              type="button"
              onClick={() => void newCode()}
              disabled={busy === "code"}
              className="ui-btn ui-btn-ghost ui-btn-sm"
            >
              New code
            </button>
          </div>
        </div>
      </div>

      <SectionCard title="Connection">
        <div className="space-y-2">
          <div className="settings-row">
            <span>Status</span>
            <span className="flex items-center gap-2">
              <span
                className="status-dot"
                style={{
                  background: remote?.running
                    ? "var(--color-success)"
                    : "var(--color-danger)",
                }}
              />
              {remote?.running ? "Listening" : "Stopped"}
            </span>
          </div>
          <div className="settings-row">
            <span>Port</span>
            <span className="font-mono">{remote?.port ?? "—"}</span>
          </div>
          {remote && remote.addresses.length > 1 && (
            <div className="settings-row">
              <span>Other addresses</span>
              <span className="font-mono text-right break-all">
                {remote.addresses.slice(1).join(", ")}
              </span>
            </div>
          )}
          {remote?.error && <p className="settings-warning">{remote.error}</p>}
          <p className="settings-hint">
            Home network only. Windows may ask once for firewall access — allow
            it on private networks.
          </p>
        </div>
      </SectionCard>

      {message && <p className="settings-message">{message}</p>}
    </div>
  );
}

/* -------------------------------------------------------------------- app */

function AppSection({ version }: { version: string }) {
  const apiKey = useAppStore(
    (s) => s.config.settings.images?.steamgriddbApiKey ?? "",
  );
  const setImageSearchApiKey = useAppStore((s) => s.setImageSearchApiKey);
  const [startsWithWindows, setStartsWithWindows] = useState<boolean | null>(null);
  const [startupBusy, setStartupBusy] = useState(false);
  const [startupMessage, setStartupMessage] = useState("");

  useEffect(() => {
    getStartWithWindows()
      .then(setStartsWithWindows)
      .catch((error) => {
        setStartsWithWindows(false);
        setStartupMessage(String(error));
      });
  }, []);

  const toggleStartup = async (enabled: boolean) => {
    setStartupBusy(true);
    setStartupMessage("");
    try {
      const applied = await setStartWithWindows(enabled);
      setStartsWithWindows(applied);
      setStartupMessage(
        applied
          ? "AYMAN MACRO DECK now starts with Windows"
          : "AYMAN MACRO DECK starts only when you open it",
      );
    } catch (error) {
      setStartupMessage(String(error));
      setStartsWithWindows(await getStartWithWindows().catch(() => false));
    } finally {
      setStartupBusy(false);
    }
  };

  return (
    <div className="space-y-4">
      <div className="settings-hero is-ok">
        <span className="brand-mark">AM</span>
        <div className="min-w-0 flex-1">
          <div className="settings-hero-title">AYMAN MACRO DECK</div>
          <div className="settings-hero-sub">
            Native keyboard engine, profiles and phone remote — no AutoHotkey
            required.
          </div>
        </div>
        <span className="settings-version">{version ? `v${version}` : "—"}</span>
      </div>

      <SectionCard title="Updates">
        <div className="settings-row">
          <div>
            <div className="settings-row-title">Check for a new release</div>
            <div className="settings-hint">
              Updates install from GitHub releases and restart the app.
            </div>
          </div>
          <UpdateButton />
        </div>
      </SectionCard>

      <SectionCard title="Startup">
        <div className="settings-row">
          <div>
            <div className="settings-row-title">Start when Windows starts</div>
            <div className="settings-hint">
              Launches with your PC so the keypad and phone remote work
              without opening the app first. Stored in your Windows startup
              list — no admin rights needed.
            </div>
          </div>
          <button
            type="button"
            onClick={() => void toggleStartup(!startsWithWindows)}
            disabled={startupBusy || startsWithWindows === null}
            className={`ui-btn ${startsWithWindows ? "ui-btn-ghost" : "ui-btn-primary"}`}
            title={
              startsWithWindows
                ? "Stop launching AYMAN MACRO DECK with Windows"
                : "Launch AYMAN MACRO DECK with Windows"
            }
          >
            <Icon name="power" size={14} />
            {startupBusy
              ? "Saving…"
              : startsWithWindows
                ? "Turn off"
                : "Turn on"}
          </button>
        </div>
        <div className="settings-row">
          <span>Status</span>
          <span className="flex items-center gap-2">
            <span
              className="status-dot"
              style={{
                background: startsWithWindows
                  ? "var(--color-success)"
                  : "var(--color-danger)",
              }}
            />
            {startsWithWindows === null
              ? "Reading…"
              : startsWithWindows
                ? "Starts with Windows"
                : "Not in startup list"}
          </span>
        </div>
        {startupMessage && <p className="settings-hint">{startupMessage}</p>}
      </SectionCard>

      <SectionCard title="Diagnostics">
        <div className="settings-row">
          <div>
            <div className="settings-row-title">Application logs</div>
            <div className="settings-hint font-mono">
              %APPDATA%\com.ayman.macrodeck\logs\app.log
            </div>
          </div>
          <button
            type="button"
            onClick={() => void openLogsFolder().catch(() => {})}
            className="ui-btn ui-btn-ghost"
          >
            <Icon name="folder" size={14} />
            Open folder
          </button>
        </div>
        <div className="settings-row">
          <div>
            <div className="settings-row-title">Configuration file</div>
            <div className="settings-hint font-mono">
              %APPDATA%\com.ayman.macrodeck\config.json
            </div>
          </div>
          <span className="settings-hint">Backed up automatically</span>
        </div>
      </SectionCard>

      <SectionCard title="Integrations">
        <div>
          <div className="settings-row-title">SteamGridDB API key</div>
          <div className="settings-hint">
            Optional. Used by the key image search in the button editor.
            {apiKey ? " A key is saved." : " No key saved yet."}
          </div>
          <input
            type="password"
            value={apiKey}
            onChange={(event) => setImageSearchApiKey(event.target.value)}
            placeholder="SteamGridDB API key"
            className="settings-input mt-2"
          />
          <p className="settings-hint mt-1">
            Free key at steamgriddb.com. Auto search tries SteamGridDB first,
            then the web.
          </p>
        </div>
      </SectionCard>
    </div>
  );
}
