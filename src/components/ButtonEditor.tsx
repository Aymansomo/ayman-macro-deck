import { useState, useEffect } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../store";
import { ActionEditor } from "./ActionEditor";
import { keyLabel } from "../lib/keyboardLayouts";
import { getApplicationInfo } from "../lib/backend";
import { ConfirmationDialog } from "./ConfirmationDialog";
import type {
  ButtonAction,
  MacroButton,
  ImageSearchResult,
} from "../types";

const TRIGGER_PATTERN =
  /^(?:(?:Ctrl|Alt|Shift|Win)\+)*(?:Numpad\d|NumpadDivide|NumpadMultiply|NumpadSubtract|NumpadAdd|NumpadDecimal|NumpadEnter|Insert|Home|PageUp|Delete|End|PageDown|Arrow(?:Up|Down|Left|Right)|F\d{1,2}|Key[A-Z]|Digit\d|Scan[0-9A-F]{1,4}(?:E0)?)$/i;

function validateTrigger(value: string): string {
  const trimmed = value.trim();
  if (!trimmed) return "";
  return TRIGGER_PATTERN.test(trimmed)
    ? ""
    : "Use a key like Numpad1, Ctrl+Shift+Numpad4, KeyA or Scan035E0";
}

interface Props {
  button: MacroButton;
  onClose: () => void;
  running?: boolean;
  lastAction?: { message: string; ok: boolean; at: number } | null;
}

export function ButtonEditor({ button, onClose, running, lastAction }: Props) {
  const config = useAppStore((s) => s.config);
  const updateButton = useAppStore((s) => s.updateButton);
  const removeButton = useAppStore((s) => s.removeButton);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const setGamingMode = useAppStore((s) => s.setGamingMode);
  const apiKey = useAppStore(
    (s) => s.config.settings.images?.steamgriddbApiKey ?? "",
  );
  const setImageSearchApiKey = useAppStore((s) => s.setImageSearchApiKey);

  const [draft, setDraft] = useState<MacroButton>(button);
  const [saved, setSaved] = useState(false);

  const [searchOpen, setSearchOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [sourceMode, setSourceMode] = useState<"auto" | "steamgriddb" | "web">(
    "auto",
  );
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState("");
  const [results, setResults] = useState<ImageSearchResult[]>([]);
  const [downloadingUrl, setDownloadingUrl] = useState<string | null>(null);
  const [showKeyInput, setShowKeyInput] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [triggerError, setTriggerError] = useState("");
  const [appPickerError, setAppPickerError] = useState("");
  const [advancedOpen, setAdvancedOpen] = useState(false);
  // Which action set is being edited: tap, hold (long press) or the second
  // state of a toggle button.
  const [actionTab, setActionTab] = useState<"press" | "long" | "alternate">(
    "press",
  );

  useEffect(() => {
    setDraft(button);
    setSaved(false);
    setSearchOpen(false);
    setQuery("");
    setSourceMode("auto");
    setResults([]);
    setSearchError("");
    setShowKeyInput(false);
    setConfirmRemove(false);
    setTriggerError(validateTrigger(button.trigger ?? ""));
    setAppPickerError("");
    setAdvancedOpen(false);
    setActionTab("press");
  }, [button.id]);

  const activeProfile = config.profiles.find(
    (p) => p.id === config.activeProfileId,
  );
  if (!activeProfile) return null;

  const isDirty = JSON.stringify(draft) !== JSON.stringify(button);

  const patch = (changes: Partial<MacroButton>) => {
    setDraft((prev) => ({ ...prev, ...changes }));
    setSaved(false);
  };

  const commit = async (target: MacroButton) => {
    const error = validateTrigger(target.trigger ?? "");
    if (error) {
      setTriggerError(error);
      return;
    }
    updateButton(activeProfile.id, target);
    await saveConfig();
    const latestConfig = useAppStore.getState().config;
    try {
      await invoke("reload_ahk_script", {
        ahkExecutable: latestConfig.settings.keyboard.ahkExecutablePath,
        scriptPath: latestConfig.settings.keyboard.ahkScriptPath,
      });
    } catch {
      // AHK reload failed — non-critical
    }
    if (target.actions.some((a) => a.type === "toggle_gaming_mode")) {
      try {
        const state = await invoke<boolean>("get_gaming_mode");
        setGamingMode(state);
      } catch {
        // ignore
      }
    }
    setSaved(true);
  };

  const handleSave = () => {
    void commit(draft);
  };

  const handleCancel = () => {
    setDraft(button);
    setSaved(false);
  };

  const handleRemove = () => {
    removeButton(activeProfile.id, button.id);
    void saveConfig();
    setConfirmRemove(false);
    onClose();
  };

  const pickImage = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Images",
            extensions: ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico", "avif"],
          },
        ],
      });
      if (typeof selected === "string") {
        patch({ imagePath: selected });
      }
    } catch {
      // ignore
    }
  };

  const chooseApplication = async (actionId: string) => {
    setAppPickerError("");
    try {
      const selected = await open({
        multiple: false,
        filters: [
          {
            name: "Applications and shortcuts",
            extensions: ["exe", "lnk", "bat", "cmd", "msi"],
          },
          { name: "All files", extensions: ["*"] },
        ],
      });
      if (typeof selected !== "string") return;

      const info = await getApplicationInfo(selected);
      // The action can live in any of the three action sets.
      const patchSet = (list: ButtonAction[] | undefined) =>
        list?.map((action) =>
          action.id === actionId
            ? {
                ...action,
                name: `Open ${info.name}`,
                params: { ...action.params, path: info.path },
              }
            : action,
        );
      // Functional update: the action was added a moment ago, so a value
      // captured before this dialog opened would drop it again.
      setDraft((prev) => ({
        ...prev,
        name: prev.name.trim() ? prev.name : info.name,
        description: prev.description.trim()
          ? prev.description
          : `Opens ${info.name}`,
        imagePath: info.iconPath ?? prev.imagePath,
        actions: patchSet(prev.actions) ?? [],
        longPressActions: patchSet(prev.longPressActions),
        alternateActions: patchSet(prev.alternateActions),
      }));
      setSaved(false);
    } catch (error) {
      setAppPickerError(
        typeof error === "string" ? error : "Could not choose that application.",
      );
    }
  };

  const runImageSearch = async (mode = sourceMode) => {
    const q = query.trim();
    if (!q) return;
    setSearching(true);
    setSearchError("");
    setResults([]);
    try {
      const found = await invoke<ImageSearchResult[]>("search_images", {
        query: q,
        apiKey,
        source: mode,
      });
      setResults(found);
      if (found.length === 0) setSearchError("No images found.");
    } catch (err) {
      setSearchError(typeof err === "string" ? err : "Search failed.");
    } finally {
      setSearching(false);
    }
  };

  const chooseSearchResult = async (result: ImageSearchResult) => {
    setDownloadingUrl(result.url);
    setSearchError("");
    try {
      const path = await invoke<string>("download_image", {
        url: result.url,
        name: draft.name || `key-${draft.key}`,
      });
      patch({ imagePath: path });
      setSearchOpen(false);
      setResults([]);
      setQuery("");
    } catch (err) {
      setSearchError(
        typeof err === "string" ? err : "Could not download that image.",
      );
    } finally {
      setDownloadingUrl(null);
    }
  };

  const statusLine = running ? (
    <span className="text-[var(--color-accent)]">Running…</span>
  ) : lastAction ? (
    <span
      className={
        lastAction.ok
          ? "text-[var(--color-success)]"
          : "text-[var(--color-danger)]"
      }
      title={lastAction.message}
    >
      {lastAction.ok ? "✓" : "✕"} {lastAction.message}
    </span>
  ) : null;

  return (
    <div className="button-editor">
      {/* Header: live button preview, like a Stream Deck key */}
      <div className="px-4 pt-4 pb-3 border-b border-[var(--color-border)]">
        <div className="flex items-start justify-between gap-3">
          <div className="flex items-center gap-3 min-w-0">
            <div className="w-14 h-14 shrink-0 rounded-xl bg-[var(--color-bg-card)] border border-[var(--color-border)] flex items-center justify-center overflow-hidden shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]">
              {draft.imagePath ? (
                <img
                  src={convertFileSrc(draft.imagePath)}
                  alt=""
                  draggable={false}
                  className="w-full h-full object-cover"
                />
              ) : (
                <span className="text-2xl">{draft.icon || "⬜"}</span>
              )}
            </div>
            <div className="min-w-0">
              <div className="flex items-center gap-1.5">
                <span className="px-1.5 py-0.5 rounded-md bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[9px] font-mono text-[var(--color-text-secondary)]">
                  {draft.physicalKeyId ? keyLabel(draft.physicalKeyId) : draft.key}
                </span>
                <span className="text-[10px] text-[var(--color-text-muted)]">
                  {draft.actions.length}{" "}
                  {draft.actions.length === 1 ? "action" : "actions"}
                </span>
              </div>
              <h2 className="text-sm font-semibold text-[var(--color-text-primary)] truncate mt-0.5">
                {draft.name.trim() || "Unnamed button"}
              </h2>
              <div className="text-[10px] truncate">{statusLine}</div>
            </div>
          </div>
          <div className="flex items-center gap-1.5 shrink-0">
            <button
              onClick={() => patch({ enabled: !draft.enabled })}
              className={`px-2.5 py-1 rounded-lg text-[11px] font-medium transition-colors cursor-pointer border ${
                draft.enabled
                  ? "bg-[var(--color-success)]/15 text-[var(--color-success)] border-[var(--color-success)]/30"
                  : "bg-[var(--color-danger)]/15 text-[var(--color-danger)] border-[var(--color-danger)]/30"
              }`}
            >
              {draft.enabled ? "On" : "Off"}
            </button>
            <button
              onClick={onClose}
              className="w-7 h-7 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
            >
              ✕
            </button>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-4 py-4 space-y-5">
        <div className="grid grid-cols-[1fr_auto] gap-2">
          <div>
            <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
              NAME
            </label>
            <input
              type="text"
              value={draft.name}
              onChange={(e) => patch({ name: e.target.value })}
              placeholder="Button name"
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
            />
          </div>
          <div className="w-16">
            <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
              ICON
            </label>
            <input
              type="text"
              value={draft.icon}
              onChange={(e) => patch({ icon: e.target.value })}
              placeholder="🎮"
              className="w-full px-2 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-center text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
            />
          </div>
        </div>

        <div>
          <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
            DESCRIPTION
          </label>
          <input
            type="text"
            value={draft.description}
            onChange={(e) => patch({ description: e.target.value })}
            placeholder="What this button does"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
        </div>

        {/* Image */}
        <div>
          <label className="text-[10px] font-medium text-[var(--color-text-muted)] tracking-wider mb-1.5 block">
            BUTTON IMAGE
          </label>
          <div className="flex gap-2">
            <button
              type="button"
              onClick={pickImage}
              className="flex-1 px-3 py-2 rounded-lg text-xs font-medium bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer"
            >
              {draft.imagePath ? "Change image…" : "Choose image…"}
            </button>
            <button
              type="button"
              onClick={() => setSearchOpen((v) => !v)}
              className={`px-3 py-2 rounded-lg text-xs font-medium border transition-colors cursor-pointer ${
                searchOpen
                  ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
                  : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)]"
              }`}
            >
              {searchOpen ? "Hide" : "Search"}
            </button>
            {draft.imagePath && (
              <button
                type="button"
                onClick={() => patch({ imagePath: undefined })}
                className="px-3 py-2 rounded-lg text-xs font-medium text-[var(--color-danger)] bg-[var(--color-danger)]/10 border border-[var(--color-danger)]/30 hover:bg-[var(--color-danger)]/20 transition-colors cursor-pointer"
              >
                Clear
              </button>
            )}
          </div>

          {searchOpen && (
            <div className="mt-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)] p-3 space-y-2">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") void runImageSearch();
                  }}
                  placeholder="e.g. Ghost of Tsushima"
                  className="flex-1 min-w-0 px-2 py-1.5 rounded bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
                />
                <button
                  type="button"
                  onClick={() => void runImageSearch()}
                  disabled={searching || !query.trim()}
                  className="px-3 py-1.5 rounded-lg text-xs font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
                >
                  {searching ? "…" : "Search"}
                </button>
              </div>

              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-1">
                  {(["auto", "steamgriddb", "web"] as const).map((mode) => {
                    const label =
                      mode === "auto"
                        ? "Auto"
                        : mode === "steamgriddb"
                          ? "SteamGridDB"
                          : "Web";
                    return (
                      <button
                        key={mode}
                        type="button"
                        disabled={mode === "steamgriddb" && !apiKey}
                        onClick={() => {
                          setSourceMode(mode);
                          if (query.trim()) void runImageSearch(mode);
                        }}
                        className={`px-2 py-1 rounded border text-[10px] font-medium transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed ${
                          sourceMode === mode
                            ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
                            : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]"
                        }`}
                      >
                        {label}
                      </button>
                    );
                  })}
                </div>
                <button
                  type="button"
                  onClick={() => setShowKeyInput((v) => !v)}
                  className="text-[10px] text-[var(--color-accent)] hover:underline cursor-pointer shrink-0"
                >
                  {apiKey ? "API key" : "Add API key"}
                </button>
              </div>

              {showKeyInput && (
                <div>
                  <input
                    type="password"
                    value={apiKey}
                    onChange={(e) => setImageSearchApiKey(e.target.value)}
                    placeholder="SteamGridDB API key"
                    className="w-full px-2 py-1.5 rounded bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
                  />
                  <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
                    Free key at steamgriddb.com. Auto searches SteamGridDB
                    first and falls back to web search when nothing is found.
                  </p>
                </div>
              )}

              {searchError && (
                <p className="text-[10px] text-[var(--color-danger)] break-words">
                  {searchError}
                </p>
              )}

              {results.length > 0 && (
                <div className="grid grid-cols-4 gap-1.5 max-h-52 overflow-y-auto">
                  {results.map((result, index) => (
                    <button
                      key={`${result.url}-${index}`}
                      type="button"
                      onClick={() => void chooseSearchResult(result)}
                      disabled={downloadingUrl !== null}
                      title={result.title}
                      className="relative aspect-square rounded-md overflow-hidden border border-[var(--color-border)] hover:border-[var(--color-accent)] transition-colors cursor-pointer disabled:cursor-wait"
                    >
                      <img
                        src={result.thumbnail}
                        alt={result.title}
                        referrerPolicy="no-referrer"
                        loading="lazy"
                        className="w-full h-full object-cover bg-[var(--color-bg-card)]"
                      />
                      {downloadingUrl === result.url && (
                        <span className="absolute inset-0 flex items-center justify-center bg-black/60 text-[10px] text-white">
                          …
                        </span>
                      )}
                    </button>
                  ))}
                </div>
              )}
            </div>
          )}
        </div>

        {/* Actions: tap, hold (long press) and the second toggle state */}
        <div className="space-y-2">
          <div className="flex items-center gap-1 rounded-lg bg-[var(--color-bg-card)]/60 border border-[var(--color-border)] p-1">
            {(
              [
                {
                  id: "press" as const,
                  label: "Tap",
                  count: draft.actions.length,
                },
                {
                  id: "long" as const,
                  label: "Hold",
                  count: (draft.longPressActions ?? []).length,
                },
                {
                  id: "alternate" as const,
                  label: "2nd state",
                  count: (draft.alternateActions ?? []).length,
                },
              ]
            ).map((tab) => (
              <button
                key={tab.id}
                type="button"
                onClick={() => setActionTab(tab.id)}
                className={`flex-1 px-2 py-1.5 rounded-md text-[11px] transition-colors cursor-pointer ${
                  actionTab === tab.id
                    ? "bg-[var(--color-accent)] text-white font-medium"
                    : "text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-card-hover)]"
                }`}
              >
                {tab.label}
                {tab.count > 0 && (
                  <span className="ml-1 opacity-70">{tab.count}</span>
                )}
              </button>
            ))}
          </div>

          {actionTab === "press" && (
            <ActionEditor
              actions={draft.actions}
              onChange={(actions) => patch({ actions })}
              onPickApplication={(actionId) => void chooseApplication(actionId)}
              appPickerError={appPickerError}
            />
          )}

          {actionTab === "long" && (
            <div className="space-y-2">
              <p className="text-[10px] text-[var(--color-text-muted)] leading-relaxed">
                Runs instead of the Tap actions when the key is held down.
              </p>
              <div className="flex items-center gap-2">
                <span className="text-[10px] text-[var(--color-text-muted)]">
                  Hold for
                </span>
                {[300, 450, 700, 1000].map((ms) => (
                  <button
                    key={ms}
                    type="button"
                    onClick={() => patch({ longPressMs: ms })}
                    className={`px-2 py-1 rounded-md text-[10px] border transition-colors cursor-pointer ${
                      (draft.longPressMs ?? 450) === ms
                        ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
                        : "border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]"
                    }`}
                  >
                    {ms} ms
                  </button>
                ))}
              </div>
              <ActionEditor
                actions={draft.longPressActions ?? []}
                onChange={(longPressActions) =>
                  patch({
                    longPressActions,
                    longPressMs: draft.longPressMs ?? 450,
                  })
                }
                onPickApplication={(actionId) =>
                  void chooseApplication(actionId)
                }
                appPickerError={appPickerError}
              />
            </div>
          )}

          {actionTab === "alternate" && (
            <div className="space-y-2">
              <p className="text-[10px] text-[var(--color-text-muted)] leading-relaxed">
                With actions here, every press flips the button between Tap and
                this second state (on / off), and the deck remembers it while it
                runs.
              </p>
              <ActionEditor
                actions={draft.alternateActions ?? []}
                onChange={(alternateActions) =>
                  patch({
                    alternateActions,
                    toggle: alternateActions.length > 0,
                  })
                }
                onPickApplication={(actionId) =>
                  void chooseApplication(actionId)
                }
                appPickerError={appPickerError}
              />
            </div>
          )}
        </div>

        {/* Advanced: physical key binding */}
        <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-bg-card)]/40">
          <button
            type="button"
            onClick={() => setAdvancedOpen((v) => !v)}
            className="w-full flex items-center justify-between px-3 py-2.5 text-left cursor-pointer"
          >
            <span className="text-[10px] font-semibold tracking-wider text-[var(--color-text-muted)]">
              KEY BINDING
            </span>
            <span className="flex items-center gap-2 text-[10px] text-[var(--color-text-muted)]">
              <span className="font-mono">
                {draft.physicalKeyId
                  ? `Physical: ${draft.physicalKeyId}`
                  : `Slot ${draft.key || "—"}`}
              </span>
              <span>{advancedOpen ? "▲" : "▼"}</span>
            </span>
          </button>

          {advancedOpen && (
            <div className="px-3 pb-3 space-y-3 border-t border-[var(--color-border)]/70 pt-3">
              <div className="text-[10px] text-[var(--color-text-muted)] leading-relaxed">
                Legacy slot {draft.key || "—"} · NumLock-independent. Use the
                Device Manager to calibrate an unknown key.
              </div>
              <div>
                <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
                  Custom trigger (optional)
                </label>
                <input
                  value={draft.trigger ?? ""}
                  onChange={(event) => {
                    const value = event.target.value;
                    setTriggerError(validateTrigger(value));
                    patch({ trigger: value.trim() ? value.trim() : undefined });
                  }}
                  placeholder="Numpad1  ·  Ctrl+Shift+Numpad4  ·  Scan035E0"
                  className={`w-full px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border text-[11px] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] ${
                    triggerError
                      ? "border-[var(--color-danger)]"
                      : "border-[var(--color-border)]"
                  }`}
                />
                {triggerError ? (
                  <p className="text-[10px] text-[var(--color-danger)] mt-1">
                    {triggerError}
                  </p>
                ) : (
                  <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
                    Leave empty to use the physical key above.
                  </p>
                )}
              </div>

              <div>
                <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
                  Global hotkey (optional)
                </label>
                <input
                  value={draft.globalHotkey ?? ""}
                  onChange={(event) =>
                    patch({
                      globalHotkey: event.target.value.trim()
                        ? event.target.value.trim()
                        : undefined,
                    })
                  }
                  placeholder="Ctrl+Alt+F9"
                  className="w-full px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)]"
                />
                <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
                  Runs this button from any app, even with this window closed.
                  Needs at least one modifier (Ctrl, Alt, Shift or Win).
                </p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 px-4 py-3 border-t border-[var(--color-border)] bg-[var(--color-bg-primary)]">
        <button
          onClick={handleSave}
          disabled={!isDirty}
          className="flex-1 px-4 py-2 rounded-lg text-sm font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {saved && !isDirty ? "Saved ✓" : "Save"}
        </button>
        {isDirty && (
          <button
            onClick={handleCancel}
            className="px-4 py-2 rounded-lg text-sm font-medium text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] transition-colors cursor-pointer"
          >
            Cancel
          </button>
        )}
        <button
          onClick={() => setConfirmRemove(true)}
          title="Remove this key assignment from the active profile"
          className="px-3 py-2 rounded-lg text-sm text-[var(--color-danger)] hover:bg-[var(--color-danger)]/10 border border-[var(--color-border)] transition-colors cursor-pointer"
        >
          Remove
        </button>
      </div>

      <ConfirmationDialog
        open={confirmRemove}
        title="Remove key assignment"
        message={`Remove "${draft.name}" from ${activeProfile.name}? The physical key becomes unassigned; no other profile is changed.`}
        confirmLabel="Remove"
        danger
        onConfirm={handleRemove}
        onCancel={() => setConfirmRemove(false)}
      />
    </div>
  );
}
