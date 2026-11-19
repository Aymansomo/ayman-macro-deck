import { useState } from "react";
import { useAppStore } from "../store";
import type { MacroEngineStatus } from "../lib/backend";
import { ProfileManager } from "./ProfileManager";
import { UpdateButton } from "./UpdateButton";
import { SteamAccountsPanel } from "./SteamAccountsPanel";
import { SettingsPanel } from "./SettingsPanel";

interface Props {
  engineStatus: MacroEngineStatus | null;
  onOpenDeviceManager: () => void;
}

export function Header({ engineStatus, onOpenDeviceManager }: Props) {
  const config = useAppStore((s) => s.config);
  const [managerOpen, setManagerOpen] = useState(false);
  const [steamOpen, setSteamOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );
  const device = engineStatus?.device;
  const deviceTone = !device?.selectedId
    ? "bg-[var(--color-danger)]"
    : device.selectedConnected
      ? "bg-[var(--color-success)]"
      : "bg-[var(--color-warning)]";

  return (
    <>
      <header className="app-header">
        <div className="brand">
          <span className="brand-mark">AM</span>
          <div>
            <div className="brand-title">AYMAN MACRO DECK</div>
            <div className="brand-subtitle">Control surface</div>
          </div>
        </div>
        <div className="header-actions">
          <UpdateButton />
          <div className="flex items-center gap-2 text-sm text-[var(--color-text-secondary)]">
            <span className="w-2 h-2 rounded-full bg-[var(--color-success)] inline-block" />
            {activeProfile?.name ?? "No Profile"}
          </div>
          <button
            onClick={onOpenDeviceManager}
            className="relative w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Device Manager — select the physical keypad"
          >
            ⌨
            <span
              className={`absolute -top-0.5 -right-0.5 w-2 h-2 rounded-full border border-[var(--color-bg-primary)] ${deviceTone}`}
            />
          </button>
          <button
            onClick={() => setSteamOpen(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Steam Accounts"
          >
            🎮
          </button>
          <button
            onClick={() => setSettingsOpen(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Settings — phone remote & hotkeys"
          >
            🔧
          </button>
          <button
            onClick={() => setManagerOpen(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Manage Profiles"
          >
            ⚙
          </button>
        </div>
      </header>
      <ProfileManager open={managerOpen} onClose={() => setManagerOpen(false)} />
      <SteamAccountsPanel open={steamOpen} onClose={() => setSteamOpen(false)} />
      <SettingsPanel open={settingsOpen} onClose={() => setSettingsOpen(false)} />
    </>
  );
}
