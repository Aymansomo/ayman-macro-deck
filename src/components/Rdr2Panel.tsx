import { useAppStore } from "../store";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function Rdr2Panel({ open, onClose }: Props) {
  const rdr2 = useAppStore((s) => s.config.settings.rdr2);
  const updateRdr2Settings = useAppStore((s) => s.updateRdr2Settings);

  if (!open) return null;

  const set = (patch: Partial<typeof rdr2>) => updateRdr2Settings(patch);

  return (
    <div
      className="fixed inset-0 z-[90] flex justify-end bg-black/40"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-[400px] h-full bg-[var(--color-bg-secondary)] border-l border-[var(--color-border)] shadow-2xl flex flex-col animate-slideRight">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--color-border)]">
          <h2 className="text-base font-semibold text-[var(--color-text-primary)]">
            🐎 RDR2 Cycle
          </h2>
          <button
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>

        {/* Flow description */}
        <div className="px-5 py-4 border-b border-[var(--color-border)]">
          <label className="text-xs text-[var(--color-text-muted)] mb-2 block">
            FLOW (run from button with "RDR2 Cycle" action)
          </label>
          <ol className="text-xs text-[var(--color-text-secondary)] space-y-1 list-decimal list-inside">
            <li>Backup saves to safe folder</li>
            <li>Uninstall Rockstar Launcher</li>
            <li>Install fresh launcher silently</li>
          </ol>
        </div>

        {/* Settings */}
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-5">
          <Field label="SAVES FOLDER (Profiles)">
            <input
              type="text"
              value={rdr2.savesPath}
              onChange={(e) => set({ savesPath: e.target.value })}
              placeholder="C:\Users\...\Rockstar Games\Red Dead Redemption 2\Profiles"
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors font-mono text-xs"
            />
          </Field>

          <Field label="BACKUP FOLDER (safe)">
            <input
              type="text"
              value={rdr2.backupPath}
              onChange={(e) => set({ backupPath: e.target.value })}
              placeholder="E:\...\RDR2Backup\Saves"
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors font-mono text-xs"
            />
          </Field>

          <Field label="FRESH INSTALLER (.exe)">
            <input
              type="text"
              value={rdr2.installerPath}
              onChange={(e) => set({ installerPath: e.target.value })}
              placeholder="C:\...\Rockstar-Games-Launcher.exe"
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors font-mono text-xs"
            />
          </Field>

          <Field label="UNINSTALL COMMAND (optional)">
            <input
              type="text"
              value={rdr2.uninstallCommand}
              onChange={(e) => set({ uninstallCommand: e.target.value })}
              placeholder='Leave empty to auto-detect, e.g. "%ProgramFiles%\Rockstar Games\Launcher\uninstaller.exe"'
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors font-mono text-xs"
            />
          </Field>

          <button
            onClick={() => set({ confirmBeforeUninstall: !rdr2.confirmBeforeUninstall })}
            className={`w-full flex items-center justify-between px-4 py-3 rounded-lg border transition-colors cursor-pointer ${
              rdr2.confirmBeforeUninstall
                ? "bg-[var(--color-bg-card)] border-[var(--color-border)]"
                : "bg-[var(--color-danger)]/10 border-[var(--color-danger)]/50"
            }`}
          >
            <span className="text-sm text-[var(--color-text-primary)]">
              Ask before uninstall
            </span>
            <span
              className={`text-xs font-medium px-2 py-0.5 rounded transition-colors ${
                rdr2.confirmBeforeUninstall
                  ? "bg-[var(--color-success)]/15 text-[var(--color-success)]"
                  : "bg-[var(--color-danger)]/15 text-[var(--color-danger)]"
              }`}
            >
              {rdr2.confirmBeforeUninstall ? "ON" : "OFF"}
            </span>
          </button>

          <p className="text-[10px] leading-relaxed text-[var(--color-text-muted)]">
            Changes save automatically. Assign the <b>"RDR2 Cycle"</b> action to any
            button you like, then press that button on the numpad.
          </p>
        </div>
      </div>
    </div>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div>
      <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
        {label}
      </label>
      {children}
    </div>
  );
}