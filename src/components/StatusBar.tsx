import { useAppStore } from "../store";
import type { MacroEngineStatus } from "../lib/backend";
import { ProfileSelector } from "./ProfileSelector";

interface Props {
  engineStatus: MacroEngineStatus | null;
}

export function StatusBar({ engineStatus }: Props) {
  const isGamingMode = useAppStore((s) => s.isGamingMode);
  const device = engineStatus?.device;
  const paused = engineStatus?.keyboard.paused ?? false;
  const running = engineStatus?.keyboard.running ?? false;

  return (
    <div className="status-bar">
      <ProfileSelector />
      <div className="status-chips">
        <span className="status-chip">
          <span
            className="status-dot"
            style={{
              background: isGamingMode
                ? "var(--color-warning)"
                : "var(--color-text-muted)",
            }}
          />
          {isGamingMode ? "Gaming mode" : "Normal mode"}
        </span>
        <span className="status-chip">
          <span
            className="status-dot"
            style={{
              background: !device?.selectedId
                ? "var(--color-danger)"
                : device.selectedConnected
                  ? "var(--color-success)"
                  : "var(--color-warning)",
            }}
          />
          {!device?.selectedId
            ? "No keypad"
            : device.selectedConnected
              ? "Keypad connected"
              : "Keypad disconnected"}
        </span>
        <span className="status-chip">
          <span
            className="status-dot"
            style={{
              background: !running
                ? "var(--color-danger)"
                : paused
                  ? "var(--color-warning)"
                  : "var(--color-success)",
            }}
          />
          {paused ? "Engine paused" : running ? "Engine running" : "Engine stopped"}
        </span>
      </div>
    </div>
  );
}
