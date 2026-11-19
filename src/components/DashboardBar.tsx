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
  const engineTone = !running
    ? "var(--color-danger)"
    : paused
      ? "var(--color-warning)"
      : "var(--color-success)";

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
          <div className="dashboard-card-label">Active profile</div>
          <div className="dashboard-card-value truncate" title={activeProfile?.name}>
            {activeProfile?.name ?? "None"}
          </div>
          <div className="dashboard-card-meta">
            <span
              className="status-dot"
              style={{
                background: isGamingMode
                  ? "var(--color-warning)"
                  : "var(--color-text-muted)",
              }}
            />
            {isGamingMode ? "Gaming mode" : "Normal mode"}
          </div>
        </div>

        <div className="dashboard-card">
          <div className="dashboard-card-label">Macro keypad</div>
          <div className="dashboard-card-value truncate" title={shortDeviceName(engineStatus)}>
            {shortDeviceName(engineStatus)}
          </div>
          <button
            type="button"
            onClick={onOpenDeviceManager}
            className="dashboard-card-action"
          >
            Change keypad
          </button>
        </div>

        <div className="dashboard-card">
          <div className="dashboard-card-label">Engine</div>
          <div className="dashboard-card-value">
            <span className="status-dot" style={{ background: engineTone }} />
            {paused ? "Paused" : running ? "Running" : "Stopped"}
          </div>
          <button
            type="button"
            onClick={() => void toggleEngine()}
            disabled={busy}
            className="dashboard-card-action"
          >
            {busy ? "Working…" : paused ? "Resume shortcuts" : running ? "Pause shortcuts" : "Start engine"}
          </button>
        </div>

        <div className="dashboard-card">
          <div className="dashboard-card-label">This layout</div>
          <div className="dashboard-card-value">
            {configuredCount} keys · {actionCount} actions
          </div>
          <div className="dashboard-card-meta">
            {engineStatus?.keyboard.triggers ?? 0} triggers this session
          </div>
        </div>

        <div className="dashboard-card dashboard-card-last">
          <div className="dashboard-card-label">Last action</div>
          {lastAction ? (
            <>
              <div className="dashboard-card-value truncate">
                <span
                  className={
                    lastAction.ok
                      ? "text-[var(--color-success)]"
                      : "text-[var(--color-danger)]"
                  }
                >
                  {lastAction.ok ? "✓" : "✕"}
                </span>{" "}
                {lastAction.buttonName}
              </div>
              <div
                className="dashboard-card-meta truncate"
                title={lastAction.message}
              >
                {lastAction.keyId ? `${lastAction.keyId} · ` : ""}
                {lastAction.message}
              </div>
            </>
          ) : (
            <div className="dashboard-card-meta">Waiting for a keypad trigger…</div>
          )}
        </div>
      </div>
      {error && <p className="text-[10px] text-[var(--color-danger)] mt-2">{error}</p>}
    </div>
  );
}
