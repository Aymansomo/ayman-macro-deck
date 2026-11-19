import { useAppStore } from "../store";
import type { MacroEngineStatus } from "../lib/backend";
import { ProfileSelector } from "./ProfileSelector";

interface Props {
  engineStatus: MacroEngineStatus | null;
}

export function StatusBar({ engineStatus }: Props) {
  const isGamingMode = useAppStore((s) => s.isGamingMode);
  const device = engineStatus?.device;

  return (
    <div className="status-bar">
      <div className="flex items-center gap-4">
        <ProfileSelector />
      </div>
      <div className="flex items-center gap-4 text-xs">
        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${isGamingMode ? "bg-[var(--color-warning)]" : "bg-[var(--color-text-muted)]"}`}
          />
          <span className="text-[var(--color-text-secondary)]">
            {isGamingMode ? "Gaming Mode ON" : "Normal Mode"}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${!device?.selectedId ? "bg-[var(--color-danger)]" : device.selectedConnected ? "bg-[var(--color-success)]" : "bg-[var(--color-warning)]"}`}
          />
          <span className="text-[var(--color-text-secondary)]">
            {!device?.selectedId
              ? "No keypad selected"
              : device.selectedConnected
                ? "Keypad connected"
                : "Keypad disconnected"}
          </span>
        </div>
        <div className="flex items-center gap-1.5">
          <span
            className={`w-2 h-2 rounded-full ${engineStatus?.keyboard.running ? "bg-[var(--color-success)]" : "bg-[var(--color-danger)]"}`}
          />
          <span className="text-[var(--color-text-secondary)]">
            {engineStatus?.keyboard.paused
              ? "Engine paused"
              : engineStatus?.keyboard.running
                ? "Engine running"
                : "Engine stopped"}
          </span>
        </div>
      </div>
    </div>
  );
}
