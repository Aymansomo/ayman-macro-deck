import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../store";

interface Props {
  open: boolean;
  onClose: () => void;
}

export function SteamAccountsPanel({ open, onClose }: Props) {
  const steam = useAppStore((s) => s.config.settings.steam);
  const addSteamAccount = useAppStore((s) => s.addSteamAccount);
  const renameSteamAccount = useAppStore((s) => s.renameSteamAccount);
  const removeSteamAccount = useAppStore((s) => s.removeSteamAccount);
  const setSteamProfilesFolder = useAppStore((s) => s.setSteamProfilesFolder);

  const [folderDraft, setFolderDraft] = useState(steam.profileShortcutsFolder);
  const [shortcuts, setShortcuts] = useState<string[]>([]);
  const [scanError, setScanError] = useState("");
  const [scanning, setScanning] = useState(false);

  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [newName, setNewName] = useState("");
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);

  useEffect(() => {
    if (!open) return;
    setFolderDraft(steam.profileShortcutsFolder);
    void scan(steam.profileShortcutsFolder, true);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, steam.profileShortcutsFolder]);

  if (!open) return null;

  const accounts = steam.profileShortcuts;

  const scan = async (folder: string, run: boolean) => {
    if (!run) return;
    setScanning(true);
    setScanError("");
    try {
      const found = await invoke<string[]>("list_lnk_files", { folder });
      setShortcuts(found);
      if (found.length === 0) setScanError("No .lnk files found in this folder");
    } catch (e) {
      setScanError(String(e));
      setShortcuts([]);
    } finally {
      setScanning(false);
    }
  };

  const commitFolder = () => {
    const f = folderDraft.trim();
    if (f && f !== steam.profileShortcutsFolder) setSteamProfilesFolder(f);
  };

  const doAdd = () => {
    if (!name.trim() || !path.trim()) return;
    addSteamAccount(name.trim(), path.trim());
    setName("");
  };

  const startRename = (acc: string) => {
    setRenaming(acc);
    setNewName(acc);
  };

  const handleRemove = (acc: string) => {
    if (confirmRemove === acc) {
      removeSteamAccount(acc);
      setConfirmRemove(null);
    } else {
      setConfirmRemove(acc);
    }
  };

  return (
    <div
      className="fixed inset-0 z-[90] flex justify-end bg-black/40"
      onMouseDown={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className="w-[420px] h-full bg-[var(--color-bg-secondary)] border-l border-[var(--color-border)] shadow-2xl flex flex-col animate-slideRight">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--color-border)]">
          <h2 className="text-base font-semibold text-[var(--color-text-primary)]">
            🎮 Steam Accounts
          </h2>
          <button
            onClick={onClose}
            className="w-8 h-8 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>

        {/* Explanation */}
        <div className="px-5 py-4 border-b border-[var(--color-border)]">
          <label className="text-xs text-[var(--color-text-muted)] mb-2 block">
            HOW IT WORKS
          </label>
          <p className="text-xs text-[var(--color-text-secondary)] leading-relaxed">
            These are the shortcuts the{" "}
            <b className="text-[var(--color-text-primary)]">
              "Switch Steam Account"
            </b>{" "}
            action runs. The Account Name you type in a button must match one of
            the names below exactly.
          </p>
        </div>

        {/* Folder + detected shortcuts */}
        <div className="px-5 py-4 space-y-4">
          <Field label="STEAM SHORTCUTS FOLDER">
            <div className="flex gap-2">
              <input
                type="text"
                value={folderDraft}
                onChange={(e) => setFolderDraft(e.target.value)}
                onBlur={commitFolder}
                placeholder="E:\...\SteamProfiles"
                className="flex-1 px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors font-mono"
              />
              <button
                onClick={() => {
                  commitFolder();
                  void scan(folderDraft.trim(), true);
                }}
                className="px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer whitespace-nowrap"
                title="Rescan folder"
              >
                {scanning ? "..." : "⟳"}
              </button>
            </div>
          </Field>

          {scanError && (
            <p className="text-[10px] text-[var(--color-warning)]">{scanError}</p>
          )}
          {shortcuts.length > 0 && (
            <div>
              <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
                DETECTED SHORTCUTS ({shortcuts.length}) — click to use
              </label>
              <div className="space-y-1 max-h-32 overflow-y-auto">
                {shortcuts.map((s) => (
                  <button
                    key={s}
                    onClick={() => setPath(s)}
                    className={`w-full text-left px-2 py-1.5 rounded-lg border text-[10px] font-mono truncate transition-colors cursor-pointer ${
                      path === s
                        ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)] text-[var(--color-accent)]"
                        : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-secondary)] hover:border-[var(--color-border-hover)]"
                    }`}
                    title={s}
                  >
                    {s.split("\\").pop()}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        {/* Accounts list */}
        <div className="flex-1 overflow-y-auto px-5 py-4 space-y-3">
          <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider block">
            REGISTERED ACCOUNTS ({Object.keys(accounts).length})
          </label>
          {Object.keys(accounts).length === 0 && (
            <p className="text-xs text-[var(--color-text-muted)]">
              No accounts registered yet. Add one below.
            </p>
          )}
          {Object.entries(accounts)
            .sort(([a], [b]) => a.localeCompare(b))
            .map(([acc, accPath]) => (
              <div
                key={acc}
                className="rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] overflow-hidden"
              >
                {renaming === acc ? (
                  <div className="p-3 space-y-2">
                    <input
                      type="text"
                      value={newName}
                      onChange={(e) => setNewName(e.target.value)}
                      autoFocus
                      placeholder="Account name"
                      className="w-full px-2 py-1.5 rounded bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] font-mono"
                    />
                    <div className="flex gap-2">
                      <button
                        onClick={() => {
                          if (newName.trim()) renameSteamAccount(acc, newName.trim());
                          setRenaming(null);
                        }}
                        className="flex-1 px-3 py-1.5 rounded-lg text-xs font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer"
                      >
                        Save
                      </button>
                      <button
                        onClick={() => setRenaming(null)}
                        className="px-3 py-1.5 rounded-lg text-xs text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] border border-[var(--color-border)] transition-colors cursor-pointer"
                      >
                        Cancel
                      </button>
                    </div>
                  </div>
                ) : (
                  <div className="p-3">
                    <div className="flex items-center justify-between gap-2 mb-1.5">
                      <span className="text-sm font-medium text-[var(--color-text-primary)]">
                        {acc}
                      </span>
                      <div className="flex items-center gap-1">
                        <button
                          onClick={() => startRename(acc)}
                          className="w-6 h-6 flex items-center justify-center rounded text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)] transition-colors cursor-pointer"
                          title="Rename"
                        >
                          ✎
                        </button>
                        <button
                          onClick={() => handleRemove(acc)}
                          className={`px-2 h-6 flex items-center justify-center rounded text-[10px] transition-colors cursor-pointer border ${
                            confirmRemove === acc
                              ? "bg-[var(--color-danger)]/15 text-[var(--color-danger)] border-[var(--color-danger)]/40"
                              : "text-[var(--color-text-muted)] hover:text-[var(--color-danger)] hover:bg-red-500/10 border-transparent"
                          }`}
                          title="Remove"
                        >
                          {confirmRemove === acc ? "Remove?" : "✕"}
                        </button>
                      </div>
                    </div>
                    <div className="text-[10px] font-mono text-[var(--color-text-muted)] truncate" title={accPath}>
                      {accPath}
                    </div>
                  </div>
                )}
              </div>
            ))}
        </div>

        {/* Add account */}
        <div className="px-5 py-4 border-t border-[var(--color-border)] bg-[var(--color-bg-primary)] space-y-3">
          <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider block">
            ADD ACCOUNT
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Account name (e.g. qj219721)"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
          <input
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            onFocus={() => void scan(folderDraft.trim(), true)}
            placeholder='Shortcut path, e.g. "E:\...\SteamProfiles\Switch to X (Offline) [Steam].lnk"'
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs font-mono text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
          <button
            onClick={doAdd}
            disabled={!name.trim() || !path.trim()}
            className="w-full px-4 py-2 rounded-lg text-sm font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
          >
            Add Account
          </button>
          <p className="text-[10px] text-[var(--color-text-muted)]">
            Changes save automatically. The name must match exactly what you
            type in the "Switch Steam Account" action.
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