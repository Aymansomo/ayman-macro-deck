import { useAppStore } from "../store";
import { ProfileSelector } from "./ProfileSelector";

export function StatusBar() {
  const isGamingMode = useAppStore((s) => s.isGamingMode);
  const keyboardConnected = useAppStore((s) => s.keyboardConnected);

  return (
    <div className="flex items-center justify-between px-5 py-2 border-t border-[var(--color-border)] bg-[var(--color-bg-secondary)]">
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
            className={`w-2 h-2 rounded-full ${keyboardConnected ? "bg-[var(--color-success)]" : "bg-[var(--color-danger)]"}`}
          />
          <span className="text-[var(--color-text-secondary)]">
            {keyboardConnected ? "Keyboard Connected" : "Keyboard Disconnected"}
          </span>
        </div>
      </div>
    </div>
  );
}
