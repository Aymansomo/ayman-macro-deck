import { useState } from "react";
import {
  setMacroEngineRunning,
  type MacroEngineStatus,
} from "../lib/backend";
import { useAppStore } from "../store";

interface Props {
  engineStatus: MacroEngineStatus | null;
  recentActions: Array<{
    id: string;
    buttonName: string;
    keyId?: string;
    message: string;
    ok: boolean;
    at: number;
  }>;
  onOpenDeviceManager: () => void;
  onEngineStatusChange: (status: MacroEngineStatus) => void;
}

function shortDeviceName(status: MacroEngineStatus | null): string {
  const device = status?.device.devices.find(
    (candidate) => candidate.id === status.device.selectedId,
  );
  if (!device) return "None selected";
  return device.hardwareId || device.name || device.id;
}

export function DashboardBar({
  engineStatus,
  recentActions,
  onOpenDeviceManager,
  onEngineStatusChange,
}: Props) {
  const config = useAppStore((s) => s.config);
  const isGamingMode = useAppStore((s) => s.isGamingMode);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const activeProfile = config.profiles.find(
    (profile) => profile.id === config.activeProfileId,
  );
  const actionCount =
    activeProfile?.buttons.reduce((sum, button) => sum + button.actions.length, 0) ?? 0;
  const configuredCount =
    activeProfile?.buttons.filter((button) => button.actions.length > 0).length ?? 0;
  const paused = engineStatus?.keyboard.paused ?? false;
  const running = engineStatus?.keyboard.running ?? false;
  const lastAction = recentActions[0];

  const toggleEngine = async () => {
    setBusy(true);
    setError("");
    try {
      const next = await setMacroEngineRunning(paused || !running);
      onEngineStatusChange(next);
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="dashboard">
      <div className="dashboard-grid">
        <div className="dashboard-card">
          <div className="text-[9px] tracking-widest text-[var(--color-text-muted)]">
            ACTIVE PROFILE
          </div>
          <div className="text-xs font-semibold text-[var(--color-text-primary)] mt-0.5">
            {activeProfile?.name ?? "None"}
          </div>
          <div className="text-[10px] text-[var(--color-text-muted)]">
            {isGamingMode ? "Gaming mode" : "Normal mode"}
          </div>
        </div>

        <div className="dashboard-card">
          <div className="text-[9px] tracking-widest text-[var(--color-text-muted)]">
            MACRO KEYPAD
          </div>
          <div className="text-xs font-semibold text-[var(--color-text-primary)] mt-0.5 truncate max-w-[220px]">
            {shortDeviceName(engineStatus)}
          </div>
          <button
            onClick={onOpenDeviceManager}
            className="text-[10px] text-[var(--color-accent)] hover:underline cursor-pointer"
          >
            Open Device Manager
          </button>
        </div>

        <div className="dashboard-card">
          <div className="text-[9px] tracking-widest text-[var(--color-text-muted)]">
            ENGINE
          </div>
          <div className="flex items-center gap-1.5 mt-0.5">
            <span
              className={`w-2 h-2 rounded-full ${
                paused
                  ? "bg-[var(--color-warning)]"
                  : running
                    ? "bg-[var(--color-success)]"
                    : "bg-[var(--color-danger)]"
              }`}
            />
            <span className="text-xs font-semibold text-[var(--color-text-primary)]">
              {paused ? "Paused" : running ? "Running" : "Stopped"}
            </span>
          </div>
          <button
            onClick={() => void toggleEngine()}
            disabled={busy}
            className="text-[10px] text-[var(--color-accent)] hover:underline cursor-pointer disabled:opacity-40"
          >
            {busy ? "Working…" : paused ? "Resume" : running ? "Pause" : "Start"}
          </button>
        </div>

        <div className="dashboard-card">
          <div className="text-[9px] tracking-widest text-[var(--color-text-muted)]">
            THIS LAYOUT
          </div>
          <div className="text-xs font-semibold text-[var(--color-text-primary)] mt-0.5">
            {configuredCount} keys · {actionCount} actions
          </div>
          <div className="text-[10px] text-[var(--color-text-muted)]">
            {engineStatus?.keyboard.triggers ?? 0} triggers this session
          </div>
        </div>

        <div className="dashboard-card dashboard-action">
          <div className="text-[9px] tracking-widest text-[var(--color-text-muted)]">
            LAST ACTION
          </div>
          {lastAction ? (
            <div className="mt-0.5">
              <div className="text-xs font-semibold text-[var(--color-text-primary)] truncate">
                <span className={lastAction.ok ? "text-[var(--color-success)]" : "text-[var(--color-danger)]"}>
                  {lastAction.ok ? "✓" : "✕"}
                </span>{" "}
                {lastAction.buttonName}
              </div>
              <div
                className="text-[10px] text-[var(--color-text-muted)] truncate"
                title={lastAction.message}
              >
                {lastAction.keyId ? `${lastAction.keyId} · ` : ""}{lastAction.message}
              </div>
            </div>
          ) : (
            <div className="text-[10px] text-[var(--color-text-muted)] mt-1">
              Waiting for a keypad trigger…
            </div>
          )}
        </div>
      </div>
      {error && <p className="text-[10px] text-[var(--color-danger)] mt-2">{error}</p>}
    </div>
  );
}
