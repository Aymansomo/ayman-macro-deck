import { useState, useRef, useEffect } from "react";
import { useAppStore } from "../store";
import {
  exportConfig,
  importConfig,
  listConfigBackups,
  loadConfigRaw,
  openLogsFolder,
  restoreConfigBackup,
} from "../lib/backend";
import { ConfirmationDialog } from "./ConfirmationDialog";

export function ProfilesContent() {
  const config = useAppStore((s) => s.config);
  const setActiveProfile = useAppStore((s) => s.setActiveProfile);
  const createProfile = useAppStore((s) => s.createProfile);
  const deleteProfile = useAppStore((s) => s.deleteProfile);
  const renameProfile = useAppStore((s) => s.renameProfile);
  const duplicateProfile = useAppStore((s) => s.duplicateProfile);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const loadConfig = useAppStore((s) => s.loadConfig);

  const [newName, setNewName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [duplicateTarget, setDuplicateTarget] = useState<string | null>(null);
  const [duplicateName, setDuplicateName] = useState("");
  const [backups, setBackups] = useState<string[]>([]);
  const [toolsMessage, setToolsMessage] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const importRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (renamingId && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renamingId]);

  useEffect(() => {
    void listConfigBackups().then(setBackups).catch(() => setBackups([]));
  }, []);

  const syncConfig = async () => {
    const json = await loadConfigRaw();
    const parsed = JSON.parse(json);
    if (parsed?.version === 1) loadConfig(parsed);
  };

  const handleExport = async () => {
    try {
      const json = await exportConfig();
      const url = URL.createObjectURL(
        new Blob([json], { type: "application/json" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = `ayman-macro-deck-config-${Date.now()}.json`;
      link.click();
      URL.revokeObjectURL(url);
      setToolsMessage("Config exported");
    } catch (error) {
      setToolsMessage(String(error));
    }
  };

  const handleImportFile = async (file: File | undefined) => {
    if (!file) return;
    try {
      await importConfig(await file.text());
      await syncConfig();
      setToolsMessage(`Imported ${file.name}`);
    } catch (error) {
      setToolsMessage(String(error));
    }
  };

  const handleRestore = async (name: string) => {
    try {
      await restoreConfigBackup(name);
      await syncConfig();
      setToolsMessage(`Restored ${name}`);
      setBackups(await listConfigBackups());
    } catch (error) {
      setToolsMessage(String(error));
    }
  };

  const handleCreate = () => {
    const trimmed = newName.trim();
    if (!trimmed) return;
    createProfile(trimmed);
    setNewName("");
  };

  const handleRename = (id: string) => {
    const trimmed = renameValue.trim();
    if (!trimmed) return;
    renameProfile(id, trimmed);
    saveConfig();
    setRenamingId(null);
    setRenameValue("");
  };

  const handleDelete = () => {
    if (!deleteTarget) return;
    deleteProfile(deleteTarget);
    saveConfig();
    setDeleteTarget(null);
  };

  const handleDuplicate = () => {
    if (!duplicateTarget) return;
    const trimmed = duplicateName.trim();
    if (!trimmed) return;
    duplicateProfile(duplicateTarget, trimmed);
    saveConfig();
    setDuplicateTarget(null);
    setDuplicateName("");
  };

  const deleteTargetProfile = config.profiles.find((p) => p.id === deleteTarget);

  return (
    <div className="space-y-5">
      <div className="space-y-5">
        {/* Create new profile */}
        <div className="px-5 py-4 border-b border-[var(--color-border)]">
          <label className="text-xs text-[var(--color-text-muted)] mb-2 block">
            NEW PROFILE
          </label>
          <div className="flex gap-2">
            <input
              ref={inputRef}
              type="text"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
              }}
              placeholder="Profile name..."
              className="flex-1 px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
            />
            <button
              onClick={handleCreate}
              disabled={!newName.trim()}
              className="px-4 py-2 rounded-lg bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm font-medium transition-colors cursor-pointer"
            >
              Create
            </button>
          </div>
        </div>

        {/* Profile list */}
        <div className="flex-1 overflow-y-auto px-5 py-3">
          <label className="text-xs text-[var(--color-text-muted)] mb-3 block">
            PROFILES ({config.profiles.length})
          </label>
          <div className="space-y-2">
            {config.profiles.map((profile) => {
              const isActive = profile.id === config.activeProfileId;
              const isRenaming = renamingId === profile.id;

              return (
                <div
                  key={profile.id}
                  className={`rounded-lg border transition-colors ${
                    isActive
                      ? "bg-[var(--color-accent)]/10 border-[var(--color-accent)]/30"
                      : "bg-[var(--color-bg-card)] border-[var(--color-border)] hover:border-[var(--color-border-hover)]"
                  }`}
                >
                  <div className="flex items-center gap-3 px-4 py-3">
                    {/* Profile name / rename input */}
                    <div className="flex-1 min-w-0">
                      {isRenaming ? (
                        <input
                          ref={renameInputRef}
                          type="text"
                          value={renameValue}
                          onChange={(e) => setRenameValue(e.target.value)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleRename(profile.id);
                            if (e.key === "Escape") {
                              setRenamingId(null);
                              setRenameValue("");
                            }
                          }}
                          onBlur={() => handleRename(profile.id)}
                          className="w-full px-2 py-1 rounded bg-[var(--color-bg-primary)] border border-[var(--color-accent)] text-sm text-[var(--color-text-primary)] outline-none"
                        />
                      ) : (
                        <div className="flex items-center gap-2">
                          <span className="text-sm font-medium text-[var(--color-text-primary)] truncate">
                            {profile.name}
                          </span>
                          {isActive && (
                            <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-accent)]/20 text-[var(--color-accent)] font-medium">
                              ACTIVE
                            </span>
                          )}
                        </div>
                      )}
                    </div>

                    {/* Actions */}
                    {!isRenaming && (
                      <div className="flex items-center gap-1">
                        {!isActive && (
                          <button
                            onClick={() => {
                              setActiveProfile(profile.id);
                              saveConfig();
                            }}
                            className="px-2 py-1 rounded text-[11px] text-[var(--color-accent)] hover:bg-[var(--color-accent)]/10 transition-colors cursor-pointer"
                            title="Switch to this profile"
                          >
                            Switch
                          </button>
                        )}
                        <button
                          onClick={() => {
                            setRenamingId(profile.id);
                            setRenameValue(profile.name);
                          }}
                          className="w-7 h-7 flex items-center justify-center rounded text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)] transition-colors cursor-pointer"
                          title="Rename"
                        >
                          ✎
                        </button>
                        <button
                          onClick={() => {
                            setDuplicateTarget(profile.id);
                            setDuplicateName(`${profile.name} (Copy)`);
                          }}
                          className="w-7 h-7 flex items-center justify-center rounded text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)] transition-colors cursor-pointer"
                          title="Duplicate"
                        >
                          ⧉
                        </button>
                        <button
                          onClick={() => setDeleteTarget(profile.id)}
                          className="w-7 h-7 flex items-center justify-center rounded text-[var(--color-text-muted)] hover:text-[var(--color-danger)] hover:bg-red-500/10 transition-colors cursor-pointer"
                          title="Delete"
                        >
                          🗑
                        </button>
                      </div>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        <div className="border-t border-[var(--color-border)] px-5 py-4 space-y-3">
          <div className="text-[10px] font-medium tracking-wider text-[var(--color-text-muted)]">
            TOOLS
          </div>
          <div className="flex flex-wrap gap-2">
            <button
              onClick={() => void handleExport()}
              className="px-3 py-1.5 rounded-lg text-[11px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer"
            >
              Export config
            </button>
            <button
              onClick={() => importRef.current?.click()}
              className="px-3 py-1.5 rounded-lg text-[11px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer"
            >
              Import config
            </button>
            <input
              ref={importRef}
              type="file"
              accept="application/json,.json"
              className="hidden"
              onChange={(event) => {
                void handleImportFile(event.target.files?.[0]);
                event.target.value = "";
              }}
            />
            <button
              onClick={() => void openLogsFolder().catch((e) => setToolsMessage(String(e)))}
              className="px-3 py-1.5 rounded-lg text-[11px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer"
            >
              Open logs folder
            </button>
          </div>
          {backups.length > 0 && (
            <div className="space-y-1.5">
              <div className="text-[10px] text-[var(--color-text-muted)]">
                Config backups — select to restore
              </div>
              <select
                defaultValue=""
                onChange={(event) => {
                  if (event.target.value) void handleRestore(event.target.value);
                }}
                className="w-full px-2 py-1.5 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-secondary)] outline-none cursor-pointer"
              >
                <option value="">Choose a backup…</option>
                {backups.map((name) => (
                  <option key={name} value={name}>
                    {name}
                  </option>
                ))}
              </select>
            </div>
          )}
          {toolsMessage && (
            <div className="text-[10px] text-[var(--color-text-muted)]">
              {toolsMessage}
            </div>
          )}
        </div>
      </div>

      {/* Delete confirmation */}
      <ConfirmationDialog
        open={!!deleteTarget}
        title="Delete Profile"
        message={
          deleteTargetProfile
            ? `Are you sure you want to delete "${deleteTargetProfile.name}"? This cannot be undone.`
            : ""
        }
        confirmLabel="Delete"
        danger
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
      />

      {/* Duplicate dialog */}
      <ConfirmationDialog
        open={!!duplicateTarget}
        title="Duplicate Profile"
        message="Enter a name for the duplicated profile."
        confirmLabel="Duplicate"
        onConfirm={handleDuplicate}
        onCancel={() => {
          setDuplicateTarget(null);
          setDuplicateName("");
        }}
      >
        <input
          type="text"
          value={duplicateName}
          onChange={(e) => setDuplicateName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleDuplicate();
          }}
          className="w-full px-3 py-2 mt-3 rounded-lg bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] transition-colors"
          autoFocus
        />
      </ConfirmationDialog>
    </div>
  );
}
