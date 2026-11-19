import { useAppStore } from "../store";
import type { MacroEngineStatus } from "../lib/backend";
import { Icon } from "./Icon";
import { UpdateButton } from "./UpdateButton";

interface Props {
  engineStatus: MacroEngineStatus | null;
  onOpenSettings: () => void;
}

export function Header({ engineStatus, onOpenSettings }: Props) {
  const config = useAppStore((s) => s.config);
  const isGamingMode = useAppStore((s) => s.isGamingMode);
  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );

  const paused = engineStatus?.keyboard.paused ?? false;
  const running = engineStatus?.keyboard.running ?? false;
  const engineTone = !running
    ? "var(--color-danger)"
    : paused
      ? "var(--color-warning)"
      : "var(--color-success)";

  return (
    <header className="app-header">
      <div className="brand">
        <span className="brand-mark">AM</span>
        <div className="min-w-0">
          <div className="brand-title">AYMAN MACRO DECK</div>
          <div className="brand-subtitle">Control surface</div>
        </div>
      </div>

      <div className="header-actions">
        <div className="header-pill">
          <span
            className="status-dot"
            style={{
              background: isGamingMode
                ? "var(--color-warning)"
                : "var(--color-text-muted)",
            }}
          />
          <span className="truncate max-w-[140px]">
            {activeProfile?.name ?? "No profile"}
          </span>
          <span className="header-pill-sep" />
          <span className="text-[var(--color-text-muted)]">
            {isGamingMode ? "Gaming" : "Normal"}
          </span>
        </div>

        <UpdateButton />

        <button
          type="button"
          onClick={onOpenSettings}
          className="ui-btn ui-btn-ghost header-settings"
          title="Settings — engine, keypad, phone, Steam, profiles"
        >
          <span className="relative flex items-center">
            <Icon name="settings" size={15} />
            <span
              className="settings-dot"
              style={{ background: engineTone }}
            />
          </span>
          Settings
        </button>
      </div>
    </header>
  );
}
