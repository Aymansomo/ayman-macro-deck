import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../store";
import {
  ACTION_REGISTRY,
  createAction,
  getActionDef,
} from "../lib/actionRegistry";
import type { ActionDefinition, ButtonAction, ParamField } from "../types";

interface Props {
  actions: ButtonAction[];
  onChange: (actions: ButtonAction[]) => void;
  onPickApplication: (actionId: string) => void;
  appPickerError?: string;
}

/**
 * Stream-Deck style multi-action editor: every action is a step, the list runs
 * from top to bottom, and each step can wait before the next one starts.
 */
export function ActionEditor({
  actions,
  onChange,
  onPickApplication,
  appPickerError,
}: Props) {
  const [pickerOpen, setPickerOpen] = useState(actions.length === 0);
  const [query, setQuery] = useState("");

  const patchAction = (actionId: string, changes: Partial<ButtonAction>) => {
    onChange(
      actions.map((action) =>
        action.id === actionId ? { ...action, ...changes } : action,
      ),
    );
  };

  const addAction = (type: string) => {
    const action = createAction(type);
    onChange([...actions, action]);
    setPickerOpen(false);
    setQuery("");
    if (type === "open_application") onPickApplication(action.id);
  };

  const removeAction = (actionId: string) => {
    onChange(actions.filter((action) => action.id !== actionId));
  };

  const moveAction = (index: number, direction: -1 | 1) => {
    const next = [...actions];
    const target = index + direction;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    onChange(next);
  };

  const needle = query.trim().toLowerCase();
  const matches = ACTION_REGISTRY.filter(
    (def) =>
      !needle ||
      def.name.toLowerCase().includes(needle) ||
      def.description.toLowerCase().includes(needle) ||
      def.type.toLowerCase().includes(needle),
  );

  return (
    <div className="space-y-3">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-[10px] font-semibold tracking-wider text-[var(--color-text-muted)]">
            ACTIONS
          </span>
          <span className="px-1.5 py-0.5 rounded-md bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[10px] font-mono text-[var(--color-text-secondary)]">
            {actions.length}
          </span>
        </div>
        {actions.length > 1 && (
          <span className="text-[10px] text-[var(--color-text-muted)]">
            runs top → bottom
          </span>
        )}
      </div>

      {actions.length === 0 && (
        <div className="rounded-xl border border-dashed border-[var(--color-border)] px-4 py-5 text-center">
          <div className="text-lg mb-1">⚡</div>
          <p className="text-xs text-[var(--color-text-secondary)]">
            This button does nothing yet.
          </p>
          <p className="text-[10px] text-[var(--color-text-muted)] mt-0.5">
            Add one action, or chain several to run them in order.
          </p>
        </div>
      )}

      <div className="space-y-2">
        {actions.map((action, index) => {
          const def = getActionDef(action.type);
          return (
            <div
              key={action.id}
              className="group relative rounded-xl border border-[var(--color-border)] bg-[var(--color-bg-card)] overflow-hidden transition-colors hover:border-[var(--color-border-hover)]"
            >
              <div className="flex items-center gap-2 px-3 py-2.5">
                <span className="w-6 h-6 shrink-0 rounded-full bg-[var(--color-accent)]/15 border border-[var(--color-accent)]/30 text-[var(--color-accent)] text-[11px] font-semibold flex items-center justify-center">
                  {index + 1}
                </span>
                <span className="text-sm leading-none">{def?.icon ?? "⚡"}</span>
                <span className="flex-1 min-w-0 truncate text-xs font-medium text-[var(--color-text-primary)]">
                  {def?.name ?? action.type}
                </span>
                <div className="flex items-center gap-0.5">
                  <IconButton
                    label="Move up"
                    disabled={index === 0}
                    onClick={() => moveAction(index, -1)}
                  >
                    ▲
                  </IconButton>
                  <IconButton
                    label="Move down"
                    disabled={index === actions.length - 1}
                    onClick={() => moveAction(index, 1)}
                  >
                    ▼
                  </IconButton>
                  <IconButton
                    label="Remove action"
                    danger
                    onClick={() => removeAction(action.id)}
                  >
                    ✕
                  </IconButton>
                </div>
              </div>

              {def && def.paramSchema.length > 0 && (
                <div className="px-3 pb-2.5 space-y-2">
                  {def.paramSchema.map((field) => (
                    <ParamInput
                      key={field.key}
                      field={field}
                      def={def}
                      value={action.params[field.key] ?? ""}
                      onChange={(value) =>
                        patchAction(action.id, {
                          params: { ...action.params, [field.key]: value },
                        })
                      }
                      onPickApplication={() => onPickApplication(action.id)}
                    />
                  ))}
                </div>
              )}

              <div className="flex items-center gap-1.5 flex-wrap px-3 py-2 border-t border-[var(--color-border)]/70 bg-[var(--color-bg-primary)]/40">
                <span className="text-[10px] text-[var(--color-text-muted)]">
                  Wait before next
                </span>
                {[0, 250, 500, 1000, 2000].map((ms) => (
                  <button
                    key={ms}
                    type="button"
                    onClick={() =>
                      patchAction(action.id, {
                        delayAfter: ms === 0 ? undefined : ms,
                      })
                    }
                    className={`px-2 py-0.5 rounded-md border text-[10px] font-medium transition-colors cursor-pointer ${
                      (action.delayAfter ?? 0) === ms
                        ? "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
                        : "bg-[var(--color-bg-card)] border-[var(--color-border)] text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)]"
                    }`}
                  >
                    {ms === 0 ? "none" : ms >= 1000 ? `${ms / 1000}s` : `${ms}ms`}
                  </button>
                ))}
                <input
                  type="number"
                  min={0}
                  step={100}
                  value={action.delayAfter ?? 0}
                  onChange={(event) =>
                    patchAction(action.id, {
                      delayAfter: Number(event.target.value) || undefined,
                    })
                  }
                  className="w-16 px-1.5 py-0.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[10px] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)]"
                />
                <span className="text-[10px] text-[var(--color-text-muted)]">ms</span>
              </div>
            </div>
          );
        })}
      </div>

      <button
        type="button"
        onClick={() => setPickerOpen((openState) => !openState)}
        className={`w-full flex items-center justify-center gap-2 px-3 py-2.5 rounded-xl border text-xs font-medium transition-colors cursor-pointer ${
          pickerOpen
            ? "bg-[var(--color-accent)]/10 border-[var(--color-accent)]/40 text-[var(--color-accent)]"
            : "bg-[var(--color-bg-card)] border-dashed border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)]"
        }`}
      >
        <span className="text-sm leading-none">＋</span>
        {actions.length === 0 ? "Add an action" : "Add another action"}
      </button>

      {pickerOpen && (
        <div className="rounded-xl border border-[var(--color-border)] bg-[var(--color-bg-primary)] p-2.5 space-y-2">
          <input
            autoFocus
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            onKeyDown={(event) => {
              if (event.key === "Escape") setPickerOpen(false);
              if (event.key === "Enter" && matches.length > 0) {
                addAction(matches[0].type);
              }
            }}
            placeholder="Search actions…"
            className="w-full px-2.5 py-1.5 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)] text-xs text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)]"
          />
          <div className="grid grid-cols-2 gap-1.5 max-h-64 overflow-y-auto">
            {matches.map((def) => (
              <button
                key={def.type}
                type="button"
                onClick={() => addAction(def.type)}
                title={def.description}
                className="flex items-center gap-2 px-2.5 py-2 rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-card)] text-left transition-colors cursor-pointer hover:border-[var(--color-accent)]/50 hover:bg-[var(--color-accent)]/10"
              >
                <span className="text-sm leading-none shrink-0">{def.icon}</span>
                <span className="text-[11px] font-medium text-[var(--color-text-primary)] truncate">
                  {def.name}
                </span>
              </button>
            ))}
            {matches.length === 0 && (
              <p className="col-span-2 text-[10px] text-[var(--color-text-muted)] px-1 py-2">
                No action matches “{query}”.
              </p>
            )}
          </div>
        </div>
      )}

      {appPickerError && (
        <p className="text-[10px] text-[var(--color-danger)]">{appPickerError}</p>
      )}
    </div>
  );
}

function IconButton({
  children,
  label,
  onClick,
  disabled,
  danger,
}: {
  children: React.ReactNode;
  label: string;
  onClick: () => void;
  disabled?: boolean;
  danger?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      title={label}
      aria-label={label}
      className={`w-6 h-6 flex items-center justify-center rounded-md text-[10px] transition-colors cursor-pointer disabled:opacity-20 disabled:cursor-not-allowed ${
        danger
          ? "text-[var(--color-text-muted)] hover:text-[var(--color-danger)] hover:bg-[var(--color-danger)]/10"
          : "text-[var(--color-text-muted)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)]"
      }`}
    >
      {children}
    </button>
  );
}

const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

/** Key names the Rust sender understands (letters, digits, F-keys and named keys). */
function keyNameFromEvent(event: KeyboardEvent): string | null {
  const code = event.code;
  if (!code) return null;

  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  if (/^F([1-9]|1\d|2[0-4])$/.test(code)) return code;

  if (code.startsWith("Numpad")) {
    const rest = code.slice("Numpad".length);
    if (/^\d$/.test(rest)) return rest;
    if (rest === "Enter") return "Enter";
    return null;
  }

  const named: Record<string, string> = {
    Space: "Space",
    Enter: "Enter",
    Tab: "Tab",
    Escape: "Esc",
    Backspace: "Backspace",
    Delete: "Delete",
    Insert: "Insert",
    Home: "Home",
    End: "End",
    PageUp: "PageUp",
    PageDown: "PageDown",
    ArrowUp: "Up",
    ArrowDown: "Down",
    ArrowLeft: "Left",
    ArrowRight: "Right",
    PrintScreen: "PrintScreen",
    CapsLock: "CapsLock",
    NumLock: "NumLock",
    ScrollLock: "ScrollLock",
  };

  return named[code] ?? null;
}

/** Records a real key combination instead of typing it by hand. */
function ShortcutRecorder({
  value,
  onChange,
}: {
  value: string;
  onChange: (value: string) => void;
}) {
  const [capturing, setCapturing] = useState(false);
  const [live, setLive] = useState<string[]>([]);
  const [hint, setHint] = useState("");

  useEffect(() => {
    if (!capturing) return;

    const handleDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.code === "Escape") {
        setCapturing(false);
        setLive([]);
        setHint("");
        return;
      }

      const modifiers: string[] = [];
      if (event.ctrlKey) modifiers.push("Ctrl");
      if (event.altKey) modifiers.push("Alt");
      if (event.shiftKey) modifiers.push("Shift");
      if (event.metaKey) modifiers.push("Win");

      if (MODIFIER_CODES.has(event.code)) {
        setLive(modifiers);
        setHint("Now press the main key…");
        return;
      }

      const main = keyNameFromEvent(event);
      if (!main) {
        setHint(
          "That key cannot be sent yet. Use a letter, number, F-key, arrow or Enter/Tab/Esc.",
        );
        return;
      }

      const combo = [...modifiers, main].join("+");
      setLive([...modifiers, main]);
      onChange(combo);
      setCapturing(false);
      setLive([]);
      setHint("");
    };

    const handleUp = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();
    };

    window.addEventListener("keydown", handleDown, true);
    window.addEventListener("keyup", handleUp, true);
    return () => {
      window.removeEventListener("keydown", handleDown, true);
      window.removeEventListener("keyup", handleUp, true);
    };
  }, [capturing, onChange]);

  const keys = capturing ? live : value ? value.split("+") : [];

  return (
    <div>
      <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
        Shortcut
      </label>
      <div
        className={`rounded-lg border bg-[var(--color-bg-primary)] px-2 py-2 transition-colors ${
          capturing
            ? "border-[var(--color-accent)] ring-2 ring-[var(--color-accent)]/25"
            : "border-[var(--color-border)]"
        }`}
      >
        <div className="flex items-center gap-1 flex-wrap min-h-6">
          {keys.length === 0 ? (
            <span className="text-[11px] text-[var(--color-text-muted)] px-1">
              {capturing ? "Listening…" : "No shortcut recorded yet"}
            </span>
          ) : (
            keys.map((key, index) => (
              <span
                key={`${key}-${index}`}
                className="px-1.5 py-0.5 rounded-md bg-[var(--color-bg-card)] border border-[var(--color-border)] text-[11px] font-mono text-[var(--color-text-primary)]"
              >
                {key}
              </span>
            ))
          )}
        </div>
        <div className="flex items-center gap-2 mt-2">
          <button
            type="button"
            onClick={() => {
              setCapturing((current) => !current);
              setLive([]);
              setHint(capturing ? "" : "Press the keys you want to send…");
            }}
            className={`px-2.5 py-1 rounded-md text-[11px] font-medium border transition-colors cursor-pointer ${
              capturing
                ? "bg-[var(--color-accent)] text-white border-[var(--color-accent)] animate-pulse"
                : "bg-[var(--color-accent)]/15 border-[var(--color-accent)]/40 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25"
            }`}
          >
            {capturing ? "Listening…" : value ? "Record again" : "Click, then press keys"}
          </button>
          {value && !capturing && (
            <button
              type="button"
              onClick={() => onChange("")}
              className="px-2 py-1 rounded-md text-[11px] text-[var(--color-text-muted)] hover:text-[var(--color-danger)] transition-colors cursor-pointer"
            >
              Clear
            </button>
          )}
        </div>
      </div>
      <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
        {hint || "Example: hold Ctrl and press A to record Ctrl+A."}
      </p>
    </div>
  );
}

function ParamInput({
  field,
  def,
  value,
  onChange,
  onPickApplication,
}: {
  field: ParamField;
  def: ActionDefinition;
  value: string | number | boolean;
  onChange: (value: string | boolean) => void;
  onPickApplication?: () => void;
}) {
  const [options, setOptions] = useState<string[]>(field.options ?? []);
  const [loading, setLoading] = useState(false);
  const [testMessage, setTestMessage] = useState("");
  const profiles = useAppStore((s) => s.config.profiles);

  // Older buttons store the profile name. Migrate them to the stable id so a
  // later rename cannot break the button.
  useEffect(() => {
    if (def.type !== "switch_profile" || field.key !== "profile") return;
    const current = String(value);
    if (!current) return;
    const legacy = profiles.find((p) => p.name === current && p.id !== current);
    if (legacy) onChange(legacy.id);
  }, [def.type, field.key, value, profiles, onChange]);

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

  if (field.type === "keycapture") {
    return <ShortcutRecorder value={String(value)} onChange={onChange} />;
  }

  if (field.type === "audiofile") {
    const pickSound = async () => {
      setTestMessage("");
      try {
        const selected = await open({
          multiple: false,
          filters: [
            { name: "Audio", extensions: ["wav", "mp3", "m4a", "wma", "aac", "flac", "ogg"] },
            { name: "All files", extensions: ["*"] },
          ],
        });
        if (typeof selected === "string") onChange(selected);
      } catch {
        // ignored — the user cancelled the dialog
      }
    };

    const testSound = async () => {
      const path = String(value).trim();
      if (!path) return;
      setTestMessage("");
      try {
        await invoke<string>("execute_action", {
          actionType: "play_sound",
          params: { path },
        });
        setTestMessage("Playing…");
      } catch (error) {
        setTestMessage(String(error));
      }
    };

    return (
      <div>
        <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
          {field.label}
        </label>
        <div className="flex gap-1.5">
          <input
            type="text"
            value={String(value)}
            onChange={(event) => onChange(event.target.value)}
            placeholder={field.placeholder}
            className="flex-1 min-w-0 px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] font-mono outline-none focus:border-[var(--color-accent)]"
          />
          <button
            type="button"
            onClick={() => void pickSound()}
            className="px-2 py-1.5 rounded-md text-[11px] font-medium bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-hover)] transition-colors cursor-pointer whitespace-nowrap"
          >
            Choose…
          </button>
          <button
            type="button"
            onClick={() => void testSound()}
            disabled={!String(value).trim()}
            title="Play this sound now"
            className="w-8 flex items-center justify-center rounded-md bg-[var(--color-accent)]/15 border border-[var(--color-accent)]/40 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25 transition-colors cursor-pointer disabled:opacity-30 disabled:cursor-not-allowed"
          >
            ▶
          </button>
        </div>
        <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
          {testMessage ||
            "WAV, MP3, M4A, WMA… The sound plays in the background while the next action runs."}
        </p>
      </div>
    );
  }

  if (def.type === "switch_profile" && field.key === "profile") {
    return (
      <div>
        <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
          {field.label}
        </label>
        <select
          value={String(value)}
          onChange={(event) => onChange(event.target.value)}
          className="w-full px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] transition-colors cursor-pointer"
        >
          <option value="" disabled>
            Choose a profile
          </option>
          {String(value) && !profiles.some((p) => p.id === String(value)) && (
            <option value={String(value)}>{String(value)} (missing)</option>
          )}
          {profiles.map((profile) => (
            <option key={profile.id} value={profile.id}>
              {profile.name}
            </option>
          ))}
        </select>
        <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
          Pressing this button&apos;s key activates the selected profile.
        </p>
      </div>
    );
  }

  if (def.type === "open_application" && field.key === "path") {
    return (
      <div>
        <label className="text-[10px] text-[var(--color-text-muted)] mb-1 block">
          {field.label}
        </label>
        <div className="flex gap-1.5">
          <input
            type="text"
            value={String(value)}
            onChange={(event) => onChange(event.target.value)}
            placeholder={field.placeholder}
            className="flex-1 min-w-0 px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] font-mono outline-none focus:border-[var(--color-accent)] transition-colors"
          />
          <button
            type="button"
            onClick={onPickApplication}
            className="px-2.5 py-1.5 rounded-md text-[11px] font-medium bg-[var(--color-accent)]/15 border border-[var(--color-accent)]/40 text-[var(--color-accent)] hover:bg-[var(--color-accent)]/25 transition-colors cursor-pointer whitespace-nowrap"
          >
            Choose app…
          </button>
        </div>
        <p className="text-[10px] text-[var(--color-text-muted)] mt-1">
          Picking an app fills its icon, name and description automatically.
        </p>
      </div>
    );
  }

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
          className={`px-2.5 py-1 rounded-md border text-[11px] font-medium transition-colors cursor-pointer ${
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
          onChange={(event) => onChange(event.target.value)}
          className="w-full px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] outline-none focus:border-[var(--color-accent)] transition-colors cursor-pointer"
        >
          <option value="" disabled>
            {loading ? "Loading..." : "Select..."}
          </option>
          {String(value) !== "" && !options.some((o) => o === String(value)) && (
            <option value={String(value)}>{String(value)}</option>
          )}
          {options.map((option) => (
            <option key={option} value={option}>
              {option}
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
        onChange={(event) => onChange(event.target.value)}
        placeholder={field.placeholder}
        className="w-full px-2 py-1.5 rounded-md bg-[var(--color-bg-primary)] border border-[var(--color-border)] text-[11px] text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)] outline-none focus:border-[var(--color-accent)] transition-colors"
      />
    </div>
  );
}
