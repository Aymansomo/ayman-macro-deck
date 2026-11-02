import { useState } from "react";
import { useAppStore } from "../store";
import { ProfileManager } from "./ProfileManager";
import { UpdateButton } from "./UpdateButton";
import { Rdr2Panel } from "./Rdr2Panel";
import { SteamAccountsPanel } from "./SteamAccountsPanel";

export function Header() {
  const config = useAppStore((s) => s.config);
  const [managerOpen, setManagerOpen] = useState(false);
  const [rdr2Open, setRdr2Open] = useState(false);
  const [steamOpen, setSteamOpen] = useState(false);
  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );

  return (
    <>
      <div className="flex items-center justify-between px-5 py-3 border-b border-[var(--color-border)]">
        <div className="flex items-center gap-3">
          <div className="text-lg font-bold tracking-wider text-[var(--color-text-primary)]">
            SOMO MACRO DECK
          </div>
        </div>
        <div className="flex items-center gap-3">
          <UpdateButton />
          <div className="flex items-center gap-2 text-sm text-[var(--color-text-secondary)]">
            <span className="w-2 h-2 rounded-full bg-[var(--color-success)] inline-block" />
            {activeProfile?.name ?? "No Profile"}
          </div>
          <button
            onClick={() => setSteamOpen(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Steam Accounts"
          >
            🎮
          </button>
          <button
            onClick={() => setRdr2Open(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="RDR2 Cycle"
          >
            🐎
          </button>
          <button
            onClick={() => setManagerOpen(true)}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer text-sm"
            title="Manage Profiles"
          >
            ⚙
          </button>
        </div>
      </div>
      <ProfileManager open={managerOpen} onClose={() => setManagerOpen(false)} />
      <Rdr2Panel open={rdr2Open} onClose={() => setRdr2Open(false)} />
      <SteamAccountsPanel open={steamOpen} onClose={() => setSteamOpen(false)} />
    </>
  );
}
