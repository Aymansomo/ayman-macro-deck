import { useState, useEffect, useRef } from "react";
import { invoke, convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../store";
import { ACTION_REGISTRY, createAction, getActionDef } from "../lib/actionRegistry";
import type {
  MacroButton,
  ButtonAction,
  ParamField,
  ActionDefinition,
  ImageSearchResult,
} from "../types";

interface Props {
  button: MacroButton;
  onClose: () => void;
}

export function ButtonEditor({ button, onClose }: Props) {
  const config = useAppStore((s) => s.config);
  const updateButton = useAppStore((s) => s.updateButton);
  const saveConfig = useAppStore((s) => s.saveConfig);
  const setGamingMode = useAppStore((s) => s.setGamingMode);
  const apiKey = useAppStore(
    (s) => s.config.settings.images?.steamgriddbApiKey ?? "",
  );
  const setImageSearchApiKey = useAppStore((s) => s.setImageSearchApiKey);

  const [draft, setDraft] = useState<MacroButton>(button);
  const [saved, setSaved] = useState(false);
  const draftRef = useRef<MacroButton>(button);

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

  useEffect(() => {
    draftRef.current = draft;
  }, [draft]);

  useEffect(() => {
    setDraft(button);
    setSaved(false);
    setSearchOpen(false);
    setQuery("");
    setSourceMode("auto");
    setResults([]);
    setSearchError("");
    setShowKeyInput(false);
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

  const patchAction = (actionId: string, changes: Partial<ButtonAction>) => {
    setDraft((prev) => ({
      ...prev,
      actions: prev.actions.map((a) =>
        a.id === actionId ? { ...a, ...changes } : a,
      ),
    }));
    setSaved(false);
  };

  const addAction = (type: string) => {
    const action = createAction(type);
    setDraft((prev) => ({ ...prev, actions: [...prev.actions, action] }));
    setSaved(false);
  };

  const removeAction = (actionId: string) => {
    setDraft((prev) => ({
      ...prev,
      actions: prev.actions.filter((a) => a.id !== actionId),
    }));
    setSaved(false);
  };

  const moveAction = (index: number, dir: -1 | 1) => {
    setDraft((prev) => {
      const actions = [...prev.actions];
      const newIndex = index + dir;
      if (newIndex < 0 || newIndex >= actions.length) return prev;
      [actions[index], actions[newIndex]] = [actions[newIndex], actions[index]];
      return { ...prev, actions };
    });
    setSaved(false);
  };

  const commit = async (target: MacroButton) => {
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

  return (
    <div className="w-[360px] h-full bg-[var(--color-bg-secondary)] border-l border-[var(--color-border)] flex flex-col overflow-hidden animate-slideRight">
      {/* Header */}
      <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--color-border)]">
        <h2 className="text-sm font-semibold text-[var(--color-text-primary)]">
          Edit Button 
        </h2>
        <div className="flex items-center gap-2">
          <button
            onClick={() => patch({ enabled: !draft.enabled })}
            className={`px-3 py-1 rounded-lg text-xs font-medium transition-colors cursor-pointer ${
              draft.enabled
                ? "bg-[var(--color-success)]/15 text-[var(--color-success)] border border-[var(--color-success)]/30"
                : "bg-[var(--color-danger)]/15 text-[var(--color-danger)] border border-[var(--color-danger)]/30"
            }`}
          >
            {draft.enabled ? "Enabled" : "Disabled"}
          </button>
          <button
            onClick={onClose}
            className="w-7 h-7 flex items-center justify-center rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            ✕
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-5 space-y-6">
        {/* Key */}
        <Field label="KEY">
          <div className="flex items-center gap-2">
            <span className="w-10 h-10 flex items-center justify-center rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-lg font-mono font-bold text-[var(--color-accent)]">
              {draft.key}
            </span>
            <span className="text-xs text-[var(--color-text-muted)]">
              {draft.key === 10
                ? "Numpad /"
                : draft.key === 11
                  ? "Numpad *"
                  : `Numpad ${draft.key}`}
            </span>
          </div>
        </Field>

        {/* Image */}
        <Field label="IMAGE">
          <div className="flex items-center gap-3">
            <div className="w-14 h-14 shrink-0 flex items-center justify-center rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] overflow-hidden">
              {draft.imagePath ? (
                <img
                  src={convertFileSrc(draft.imagePath)}
                  alt=""
                  draggable={false}
                  className="max-w-full max-h-full object-contain"
                />
              ) : (
                <span className="text-2xl">{draft.icon || "⬜"}</span>
              )}
            </div>
            <div className="flex flex-col gap-2 flex-1 min-w-0">
              <button
                type="button"
                onClick={pickImage}
                className="px-3 py-1.5 rounded-lg text-xs font-medium bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer"
              >
                {draft.imagePath ? "Change image…" : "Choose image…"}
              </button>
              <button
                type="button"
                onClick={() => setSearchOpen((v) => !v)}
                className={`px-3 py-1.5 rounded-lg text-xs font-medium border transition-colors cursor-pointer ${
                  searchOpen
                    ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
                    : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)]"
                }`}
              >
                {searchOpen ? "Hide search" : "Search online…"}
              </button>
              {draft.imagePath && (
                <button
                  type="button"
                  onClick={() => patch({ imagePath: undefined })}
                  className="px-3 py-1.5 rounded-lg text-xs font-medium text-[var(--color-danger)] bg-[var(--color-danger)]/10 border border-[var(--color-danger)]/30 hover:bg-[var(--color-danger)]/20 transition-colors cursor-pointer"
                >
                  Remove image
                </button>
              )}
            </div>
          </div>
          {draft.imagePath && (
            <p className="text-[10px] text-[var(--color-text-muted)] mt-1 break-all">
              {draft.imagePath}
            </p>
          )}
          <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
            PNG, JPG, GIF, WEBP, BMP, SVG or ICO. Leave empty to use the emoji.
          </p>

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
        </Field>

        {/* Emoji fallback */}
        <Field label="EMOJI FALLBACK">
          <input
            type="text"
            value={draft.icon}
            onChange={(e) => patch({ icon: e.target.value })}
            placeholder="🎮"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
        </Field>

        {/* Name */}
        <Field label="NAME">
          <input
            type="text"
            value={draft.name}
            onChange={(e) => patch({ name: e.target.value })}
            placeholder="Button name"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
        </Field>

        {/* Description */}
        <Field label="DESCRIPTION">
          <input
            type="text"
            value={draft.description}
            onChange={(e) => patch({ description: e.target.value })}
            placeholder="What this button does"
            className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-sm text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
          />
        </Field>

        {/* Actions / Macro */}
        <div>
          <Field label={`ACTIONS (${draft.actions.length})`}>
            <></>
          </Field>

          <div className="space-y-3 mt-1">
            {draft.actions.map((action, i) => {
              const def = getActionDef(action.type);
              return (
                <div
                  key={action.id}
                  className="bg-[var(--color-bg-card)] border border-[var(--color-border)] rounded-lg overflow-hidden"
                >
                  {/* Action header */}
                  <div className="flex items-center justify-between px-3 py-2 border-b border-[var(--color-border)]">
                    <div className="flex items-center gap-2">
                      <span className="text-sm">{def?.icon ?? "⚡"}</span>
                      <span className="text-xs font-medium text-[var(--color-text-primary)]">
                        {def?.name ?? action.type}
                      </span>
                      <span className="text-[10px] text-[var(--color-text-muted)] font-mono">
                        #{i + 1}
                      </span>
                    </div>
                    <div className="flex items-center gap-1">
                      <button
                        onClick={() => moveAction(i, -1)}
                        disabled={i === 0}
                        className="w-6 h-6 flex items-center justify-center rounded text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)] disabled:opacity-20 disabled:cursor-not-allowed transition-colors cursor-pointer"
                        title="Move up"
                      >
                        ▲
                      </button>
                      <button
                        onClick={() => moveAction(i, 1)}
                        disabled={i === draft.actions.length - 1}
                        className="w-6 h-6 flex items-center justify-center rounded text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)] disabled:opacity-20 disabled:cursor-not-allowed transition-colors cursor-pointer"
                        title="Move down"
                      >
                        ▼
                      </button>
                      <button
                        onClick={() => removeAction(action.id)}
                        className="w-6 h-6 flex items-center justify-center rounded text-[10px] text-[var(--color-text-muted)] hover:text-[var(--color-danger)] hover:bg-red-500/10 transition-colors cursor-pointer"
                        title="Remove"
                      >
                        ✕
                      </button>
                    </div>
                  </div>

                  {/* Action params */}
                  {def && def.paramSchema.length > 0 && (
                    <div className="px-3 py-2 space-y-2">
                      {def.paramSchema.map((field) => (
                        <ParamInput
                          key={field.key}
                          field={field}
                          def={def}
                          value={String(action.params[field.key] ?? "")}
                          onChange={(v) =>
                            patchAction(action.id, {
                              params: { ...action.params, [field.key]: v },
                            })
                          }
                        />
                      ))}
                    </div>
                  )}

                  {/* Delay */}
                  <div className="px-3 py-2 border-t border-[var(--color-border)]">
                    <div className="flex items-center gap-2">
                      <label className="text-[10px] text-[var(--color-text-muted)]">
                        Delay after (ms):
                      </label>
                      <input
                        type="number"
                        value={action.delayAfter ?? 0}
                        onChange={(e) =>
                          patchAction(action.id, {
                            delayAfter: Number(e.target.value) || 0,
                          })
                        }
                        min={0}
                        step={500}
                        className="w-20 px-2 py-1 rounded bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] transition-colors"
                      />
                    </div>
                  </div>
                </div>
              );
            })}
          </div>

          {/* Add action */}
          <div className="mt-3">
            <select
              onChange={(e) => {
                if (e.target.value) {
                  addAction(e.target.value);
                  e.target.value = "";
                }
              }}
              defaultValue=""
              className="w-full px-3 py-2 rounded-lg bg-[var(--color-bg-card)] border border-dashed border-[var(--color-border)] text-sm text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors cursor-pointer appearance-none"
            >
              <option value="" disabled>
                + Add Action
              </option>
              {ACTION_REGISTRY.map((def) => (
                <option key={def.type} value={def.type}>
                  {def.icon} {def.name}
                </option>
              ))}
            </select>
          </div>
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center gap-2 px-5 py-4 border-t border-[var(--color-border)] bg-[var(--color-bg-primary)]">
        <button
          onClick={handleSave}
          disabled={!isDirty}
          className="flex-1 px-4 py-2 rounded-lg text-sm font-medium bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-white transition-colors cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed"
        >
          {saved ? "Saved ✓" : "Save"}
        </button>
        {isDirty && (
          <button
            onClick={handleCancel}
            className="px-4 py-2 rounded-lg text-sm font-medium text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card)] border border-[var(--color-border)] transition-colors cursor-pointer"
          >
            Cancel
          </button>
        )}
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

function ParamInput({
  field,
  def,
  value,
  onChange,
}: {
  field: ParamField;
  def: ActionDefinition;
  value: string | boolean;
  onChange: (v: string | boolean) => void;
}) {
  const [options, setOptions] = useState<string[]>(field.options ?? []);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    let cancelled = false;
    if (def.loadOptions) {
      setLoading(true);
      def
        .loadOptions(field.key)
        .then((opts) => {
          if (!cancelled) setOptions(opts);
        })
        .finally(() => {
          if (!cancelled) setLoading(false);
        });
    }
    return () => {
      cancelled = true;
    };
  }, [def, field.key]);

  const isSelect = field.type === "select" || options.length > 0;

  if (field.type === "boolean") {
    const on = value === true || value === "true";
    return (
      <div className="flex items-center justify-between gap-2">
        <label className="text-[10px] text-[var(--color-text-muted)]">
          {field.label}
        </label>
        <button
          type="button"
          onClick={() => onChange(!on)}
          className={`px-3 py-1.5 rounded-lg border text-xs font-medium transition-colors cursor-pointer ${
            on
              ? "bg-[var(--color-success)]/15 text-[var(--color-success)] border-[var(--color-success)]/30"
              : "bg-[var(--color-bg-card)] text-[var(--color-text-muted)] border-[var(--color-border)]"
          }`}
        >
          {on ? "ON" : "OFF"}
        </button>
      </div>
    );
  }

  if (isSelect) {
    return (
      <div>
        <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
          {field.label}
        </label>
        <select
          value={String(value)}
          onChange={(e) => onChange(e.target.value)}
          className="w-full px-2 py-1.5 rounded bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] transition-colors cursor-pointer"
        >
          <option value="" disabled>
            {loading ? "Loading..." : "Select..."}
          </option>
          {value !== "" &&
            String(value) !== "" &&
            !options.some((o) => o === String(value)) && (
              <option value={String(value)}>{String(value)}</option>
            )}
          {options.map((opt) => (
            <option key={opt} value={opt}>
              {opt}
            </option>
          ))}
        </select>
        {loading && (
          <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
            Detecting devices...
          </p>
        )}
      </div>
    );
  }

  return (
    <div>
      <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
        {field.label}
      </label>
      <input
        type="text"
        value={String(value)}
        onChange={(e) => onChange(e.target.value)}
        placeholder={field.placeholder}
        className="w-full px-2 py-1.5 rounded bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
      />
    </div>
  );
}
