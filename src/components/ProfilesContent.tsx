import { useState, useRef, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../store";
import {
  applyProfileApps,
  listMonitors,
  listRunningApps,
  listenProfileAppsResult,
  openLogsFolder,
  type MonitorInfo,
  type RunningApp,
} from "../lib/backend";
import type { ProfileApp } from "../types";
import { ConfirmationDialog } from "./ConfirmationDialog";

const PLACEMENTS: { value: NonNullable<ProfileApp["placement"]>; label: string }[] = [
  { value: "keep", label: "No move" },
  { value: "full", label: "Full screen" },
  { value: "left", label: "Left half" },
  { value: "right", label: "Right half" },
  { value: "top", label: "Top half" },
  { value: "bottom", label: "Bottom half" },
];

function appName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

export function ProfilesContent() {
  const config = useAppStore((s) => s.config);
  const setActiveProfile = useAppStore((s) => s.setActiveProfile);
  const createProfile = useAppStore((s) => s.createProfile);
  const deleteProfile = useAppStore((s) => s.deleteProfile);
  const renameProfile = useAppStore((s) => s.renameProfile);
  const duplicateProfile = useAppStore((s) => s.duplicateProfile);
  const setProfileLinkedApp = useAppStore((s) => s.setProfileLinkedApp);
  const addProfileApp = useAppStore((s) => s.addProfileApp);
  const updateProfileApp = useAppStore((s) => s.updateProfileApp);
  const removeProfileApp = useAppStore((s) => s.removeProfileApp);
  const saveConfig = useAppStore((s) => s.saveConfig);

  const [newName, setNewName] = useState("");
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [duplicateTarget, setDuplicateTarget] = useState<string | null>(null);
  const [duplicateName, setDuplicateName] = useState("");
  const [toolsMessage, setToolsMessage] = useState("");
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  // Profile whose "add running app" picker is open, plus what it can offer.
  const [pickerFor, setPickerFor] = useState<string | null>(null);
  const [runningApps, setRunningApps] = useState<RunningApp[]>([]);
  // Result lines of the last "Try now" run, per profile, as apps finish.
  const [tryResult, setTryResult] = useState<{
    profileId: string;
    lines: string[];
    running: boolean;
  } | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (renamingId && renameInputRef.current) {
      renameInputRef.current.focus();
      renameInputRef.current.select();
    }
  }, [renamingId]);

  useEffect(() => {
    void listMonitors().then(setMonitors).catch(() => setMonitors([]));
  }, []);

  // Each app of a "Try now" run reports back on its own.
  useEffect(() => {
    let active = true;
    let stop: (() => void) | undefined;
    void listenProfileAppsResult((event) => {
      setTryResult((current) => {
        if (!current || current.profileId !== event.profileId) return current;
        return {
          ...current,
          lines: [...current.lines, event.line],
          running: !event.done,
        };
      });
    }).then((unlisten) => {
      if (active) stop = unlisten;
      else unlisten();
    });
    return () => {
      active = false;
      stop?.();
    };
  }, []);

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

  const handleLinkApp = async (profileId: string) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: "Applications", extensions: ["exe"] },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (typeof selected !== "string") return;
      const fileName = selected.split(/[\\/]/).pop() ?? selected;
      setProfileLinkedApp(profileId, fileName);
      void saveConfig();
    } catch {
      // the user cancelled the dialog
    }
  };

  const handleAddApp = async (profileId: string) => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          { name: "Programs", extensions: ["exe", "lnk", "bat", "cmd"] },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (typeof selected !== "string") return;
      addProfileApp(profileId, selected);
      void saveConfig();
    } catch {
      // the user cancelled the dialog
    }
  };

  const openRunningPicker = async (profileId: string) => {
    try {
      const apps = await listRunningApps();
      setRunningApps(apps);
      setPickerFor(profileId);
    } catch {
      setRunningApps([]);
      setPickerFor(profileId);
    }
  };

  const handleTryApps = async (profileId: string) => {
    setTryResult({ profileId, lines: [], running: true });
    try {
      // Returns immediately; results arrive as `profile-apps-result` events.
      await applyProfileApps(profileId);
    } catch (error) {
      setTryResult({ profileId, lines: [String(error)], running: false });
    }
  };

  const changeApp = (
    profileId: string,
    appId: string,
    patch: Partial<ProfileApp>,
  ) => {
    updateProfileApp(profileId, appId, patch);
    void saveConfig();
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
          <label className="text-xs text-[var(--color-text-muted)] mb-1 block">
            PROFILES ({config.profiles.length})
          </label>
          <p className="text-[10px] text-[var(--color-text-muted)] mb-3 leading-relaxed">
            Link a profile to an app and it switches automatically while that
            app is in front — and back to the previous profile when it closes.
            A profile can also start apps and place their window on a monitor,
            full screen or as one half of a split screen.
          </p>
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

                      {!isRenaming && (
                        <div className="flex items-center gap-1.5 mt-1.5 flex-wrap">
                          {profile.linkedApp ? (
                            <>
                              <span
                                className="px-1.5 py-0.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[10px] font-mono text-[var(--color-text-secondary)]"
                                title="This profile activates automatically while this app is in front"
                              >
                                auto: {profile.linkedApp}
                              </span>
                              <button
                                type="button"
                                onClick={() => {
                                  setProfileLinkedApp(profile.id, "");
                                  void saveConfig();
                                }}
                                className="text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-danger)] transition-colors cursor-pointer"
                              >
                                Clear
                              </button>
                            </>
                          ) : (
                            <button
                              type="button"
                              onClick={() => void handleLinkApp(profile.id)}
                              className="text-[10px] text-[var(--color-accent)] hover:underline transition-colors cursor-pointer"
                            >
                              + Auto-switch with an app
                            </button>
                          )}
                        </div>
                      )}

                      {!isRenaming && (
                        <div className="mt-2 space-y-1.5">
                          {(profile.apps ?? []).map((app) => (
                            <div
                              key={app.id}
                              className="flex items-center gap-1.5 flex-wrap rounded-md border border-[var(--color-border)] bg-[var(--color-bg-primary)] px-2 py-1"
                            >
                              <span
                                className="text-[10px] font-mono text-[var(--color-text-secondary)] truncate max-w-[160px]"
                                title={app.path}
                              >
                                {appName(app.path)}
                              </span>
                              <select
                                value={
                                  app.monitorName &&
                                  monitors.some((m) => m.name === app.monitorName)
                                    ? app.monitorName
                                    : String(app.monitor ?? 0)
                                }
                                onChange={(event) => {
                                  // Store the device name too: the index moves
                                  // when screens are rearranged, the name does not.
                                  const picked = monitors.find(
                                    (m) => m.name === event.target.value,
                                  );
                                  changeApp(profile.id, app.id, {
                                    monitor: picked?.index ?? 0,
                                    monitorName: picked?.name,
                                  });
                                }}
                                className="text-[10px] px-1 py-0.5 rounded bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[var(--color-text-secondary)] outline-none cursor-pointer"
                                title="Monitor this app opens on"
                              >
                                {monitors.length === 0 && (
                                  <option value="0">Monitor 1</option>
                                )}
                                {monitors.map((monitor) => (
                                  <option key={monitor.name} value={monitor.name}>
                                    Monitor {monitor.index + 1}
                                    {monitor.primary ? " (main)" : ""} ·{" "}
                                    {monitor.width}×{monitor.height}
                                  </option>
                                ))}
                              </select>
                              <select
                                value={app.placement ?? "keep"}
                                onChange={(event) =>
                                  changeApp(profile.id, app.id, {
                                    placement: event.target
                                      .value as NonNullable<ProfileApp["placement"]>,
                                  })
                                }
                                className="text-[10px] px-1 py-0.5 rounded bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[var(--color-text-secondary)] outline-none cursor-pointer"
                                title="Where the window is placed"
                              >
                                {PLACEMENTS.map((option) => (
                                  <option key={option.value} value={option.value}>
                                    {option.label}
                                  </option>
                                ))}
                              </select>
                              {app.monitorName &&
                                monitors.length > 0 &&
                                !monitors.some(
                                  (m) => m.name === app.monitorName,
                                ) && (
                                  <span
                                    className="text-[10px] text-[var(--color-warning)]"
                                    title="This screen is not connected right now; the app will use the main monitor"
                                  >
                                    ⚠ {app.monitorName} missing
                                  </span>
                                )}
                              <button
                                type="button"
                                onClick={() => {
                                  removeProfileApp(profile.id, app.id);
                                  void saveConfig();
                                }}
                                className="text-[11px] leading-none text-[var(--color-text-muted)] hover:text-[var(--color-danger)] transition-colors cursor-pointer"
                                title="Remove this app"
                              >
                                ×
                              </button>
                            </div>
                          ))}
                          <div className="flex items-center gap-3 flex-wrap">
                            <button
                              type="button"
                              onClick={() => void handleAddApp(profile.id)}
                              className="text-[10px] text-[var(--color-accent)] hover:underline transition-colors cursor-pointer"
                            >
                              + Run app on activate
                            </button>
                            <button
                              type="button"
                              onClick={() => void openRunningPicker(profile.id)}
                              className="text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer"
                              title="Open the app first (even from the tray), then pick it here"
                            >
                              Pick a running app…
                            </button>
                            {(profile.apps?.length ?? 0) > 0 && (
                              <button
                                type="button"
                                onClick={() => void handleTryApps(profile.id)}
                                className="text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer"
                                title="Start and place these apps now"
                              >
                                Try now
                              </button>
                            )}
                          </div>

                          {tryResult?.profileId === profile.id && (
                            <div className="space-y-0.5">
                              {tryResult.lines.map((line, index) => (
                                <p
                                  key={index}
                                  className="text-[10px] text-[var(--color-text-secondary)] leading-relaxed"
                                >
                                  {line}
                                </p>
                              ))}
                              {tryResult.running && (
                                <p className="text-[10px] text-[var(--color-text-muted)]">
                                  Working…
                                </p>
                              )}
                            </div>
                          )}

                          {(profile.apps?.length ?? 0) === 0 && (
                            <p className="text-[10px] text-[var(--color-text-muted)] leading-relaxed">
                              After adding an app, its row shows the monitor and
                              placement pickers — full screen, or left/right
                              half for a split screen.
                            </p>
                          )}

                          {pickerFor === profile.id && (
                            <div className="rounded-md border border-[var(--color-border)] bg-[var(--color-bg-card)] p-2 space-y-1">
                              <div className="text-[10px] text-[var(--color-text-muted)]">
                                Running apps — pick one to add it; its full path
                                is saved.
                              </div>
                              <div className="max-h-40 overflow-y-auto space-y-0.5">
                                {runningApps.length === 0 && (
                                  <div className="text-[10px] text-[var(--color-text-muted)] py-1">
                                    No apps with an open window found. Open the
                                    app once (its tray menu is enough), then
                                    try again.
                                  </div>
                                )}
                                {runningApps.map((app) => (
                                  <button
                                    key={app.path}
                                    type="button"
                                    onClick={() => {
                                      addProfileApp(profile.id, app.path);
                                      void saveConfig();
                                      setPickerFor(null);
                                    }}
                                    className="w-full text-left px-2 py-1 rounded hover:bg-[var(--color-bg-card-hover)] transition-colors cursor-pointer"
                                  >
                                    <div className="text-[11px] text-[var(--color-text-primary)] truncate">
                                      {app.name}
                                    </div>
                                    <div className="text-[10px] text-[var(--color-text-muted)] truncate">
                                      {app.path}
                                    </div>
                                  </button>
                                ))}
                              </div>
                              <button
                                type="button"
                                onClick={() => setPickerFor(null)}
                                className="text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] transition-colors cursor-pointer"
                              >
                                Close
                              </button>
                            </div>
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
              onClick={() => void openLogsFolder().catch((e) => setToolsMessage(String(e)))}
              className="px-3 py-1.5 rounded-lg text-[11px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] cursor-pointer"
            >
              Open logs folder
            </button>
          </div>
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
