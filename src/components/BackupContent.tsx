import { useCallback, useEffect, useState } from "react";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  backupData,
  listConfigBackups,
  loadConfigRaw,
  openDataFolder,
  restoreConfigBackup,
  restoreData,
} from "../lib/backend";
import { useAppStore } from "../store";

/** Settings → Backup: one file with everything the app owns, plus restore. */
export function BackupContent() {
  const loadConfig = useAppStore((s) => s.loadConfig);
  const [busy, setBusy] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [lastPath, setLastPath] = useState("");
  const [backups, setBackups] = useState<string[]>([]);
  const [pendingRestore, setPendingRestore] = useState<string | null>(null);

  const refreshBackups = useCallback(async () => {
    try {
      setBackups(await listConfigBackups());
    } catch {
      setBackups([]);
    }
  }, []);

  useEffect(() => {
    void refreshBackups();
  }, [refreshBackups]);

  /** The native config is authoritative after a restore: reload the editor. */
  const reloadEditor = async () => {
    try {
      const json = await loadConfigRaw();
      const parsed = JSON.parse(json);
      if (parsed?.version === 1) loadConfig(parsed);
    } catch {
      // The engine still has the restored config; the editor catches up on reopen.
    }
  };

  const handleBackup = async () => {
    setError("");
    setMessage("");
    const stamp = new Date().toISOString().slice(0, 10);
    const target = await save({
      title: "Save a full backup",
      defaultPath: `AYMAN-MACRO-DECK-backup-${stamp}.json`,
      filters: [{ name: "Backup", extensions: ["json"] }],
    }).catch(() => null);
    if (!target) return;

    setBusy("backup");
    try {
      const summary = await backupData(target);
      setLastPath(summary.path);
      setMessage(
        `Backed up ${summary.profiles} profile(s) and ${summary.buttons} button(s).`,
      );
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy("");
    }
  };

  const handlePickRestore = async () => {
    setError("");
    setMessage("");
    const picked = await open({
      title: "Choose a backup file",
      multiple: false,
      filters: [{ name: "Backup", extensions: ["json"] }],
    }).catch(() => null);
    if (typeof picked === "string") setPendingRestore(picked);
  };

  const handleRestore = async () => {
    if (!pendingRestore) return;
    setBusy("restore");
    setError("");
    setMessage("");
    try {
      const summary = await restoreData(pendingRestore);
      await reloadEditor();
      setPendingRestore(null);
      setMessage(
        `Restored ${summary.profiles} profile(s) and ${summary.buttons} button(s).`,
      );
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy("");
    }
  };

  const handleRestoreAutomatic = async (name: string) => {
    setBusy(name);
    setError("");
    setMessage("");
    try {
      await restoreConfigBackup(name);
      await reloadEditor();
      setMessage("Restored the automatic backup.");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusy("");
    }
  };

  return (
    <div className="space-y-4">
      <div className="settings-hero is-ok">
        <div className="settings-hero-icon">
          <span style={{ fontSize: 20 }}>💾</span>
        </div>
        <div className="min-w-0 flex-1">
          <div className="settings-hero-title">Everything in one file</div>
          <div className="settings-hero-sub">
            Profiles, pages, every button and action, audio/steam/appearance
            settings, app links and the page that was open.
          </div>
        </div>
        <button
          type="button"
          onClick={() => void handleBackup()}
          disabled={busy === "backup"}
          className="ui-btn ui-btn-primary"
        >
          {busy === "backup" ? "Saving…" : "Back up everything"}
        </button>
      </div>

      <section className="settings-card">
        <div className="settings-card-head">
          <div className="ui-label">What the file contains</div>
        </div>
        <div className="space-y-2">
          <div className="settings-row">
            <span>Profiles, pages and buttons</span>
            <span>all, with their actions and long-press actions</span>
          </div>
          <div className="settings-row">
            <span>Settings</span>
            <span>keyboard, audio devices, Steam accounts, appearance</span>
          </div>
          <div className="settings-row">
            <span>App links and layouts</span>
            <span>auto-switch apps, monitor and window placement</span>
          </div>
          <div className="settings-row">
            <span>Open page per profile</span>
            <span>applied right away</span>
          </div>
          <p className="settings-hint">
            Not inside the file: button images and the keypad/phone pairing,
            which belong to this PC. Use “Open data folder” to copy those.
          </p>
        </div>
      </section>

      <section className="settings-card">
        <div className="settings-card-head">
          <div className="ui-label">Restore</div>
        </div>
        <div className="space-y-2">
          {pendingRestore ? (
            <>
              <p className="settings-hint font-mono break-all">{pendingRestore}</p>
              <p className="settings-warning">
                This replaces every profile, button and setting. The current
                configuration is saved as an automatic backup first.
              </p>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => void handleRestore()}
                  disabled={busy === "restore"}
                  className="ui-btn ui-btn-primary"
                >
                  {busy === "restore" ? "Restoring…" : "Restore this file"}
                </button>
                <button
                  type="button"
                  onClick={() => setPendingRestore(null)}
                  className="ui-btn ui-btn-ghost"
                >
                  Cancel
                </button>
              </div>
            </>
          ) : (
            <div className="settings-row">
              <div>
                <div className="settings-row-title">Restore from a backup file</div>
                <div className="settings-hint">
                  Pick a file made by “Back up everything” — on this PC or a new
                  one.
                </div>
              </div>
              <button
                type="button"
                onClick={() => void handlePickRestore()}
                className="ui-btn ui-btn-ghost"
              >
                Choose file…
              </button>
            </div>
          )}
          {lastPath && (
            <p className="settings-hint font-mono break-all">{lastPath}</p>
          )}
        </div>
      </section>

      <section className="settings-card">
        <div className="settings-card-head">
          <div className="ui-label">Automatic backups ({backups.length})</div>
          <button
            type="button"
            onClick={() => void refreshBackups()}
            className="ui-btn ui-btn-ghost ui-btn-sm"
          >
            Refresh
          </button>
        </div>
        {backups.length === 0 ? (
          <p className="settings-empty">
            Every save keeps a copy here. None yet.
          </p>
        ) : (
          <div className="max-h-52 overflow-y-auto space-y-1">
            {backups.map((name) => (
              <div
                key={name}
                className="flex items-center justify-between gap-2 rounded-md border border-[var(--color-border)] bg-[var(--color-bg-primary)] px-2 py-1"
              >
                <span className="text-[11px] font-mono text-[var(--color-text-secondary)] truncate">
                  {name}
                </span>
                <button
                  type="button"
                  onClick={() => void handleRestoreAutomatic(name)}
                  disabled={busy === name}
                  className="text-[10px] text-[var(--color-accent)] hover:underline transition-colors cursor-pointer shrink-0"
                >
                  {busy === name ? "Restoring…" : "Restore"}
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      <section className="settings-card">
        <div className="settings-card-head">
          <div className="ui-label">Data folder</div>
        </div>
        <div className="settings-row">
          <div>
            <div className="settings-row-title">Files the app owns</div>
            <div className="settings-hint font-mono">
              %APPDATA%\com.ayman.macrodeck
            </div>
            <div className="settings-hint">
              config.json, session.json, images\, device.json, remote.json, logs\
            </div>
          </div>
          <button
            type="button"
            onClick={() => void openDataFolder().catch(() => {})}
            className="ui-btn ui-btn-ghost"
          >
            Open folder
          </button>
        </div>
      </section>

      {error && <p className="settings-error">{error}</p>}
      {message && <p className="settings-message">{message}</p>}
    </div>
  );
}
