import { useAppStore } from "../store";
import { LAYOUT_OPTIONS, allKeysForLayout, keyLabel } from "../lib/keyboardLayouts";
import type { Profile } from "../types";
import { KeyboardView } from "./KeyboardView";

interface Props {
  profile: Profile;
  selectedButtonId: string | null;
  activeKeys: Set<string>;
  runningButtonId: string | null;
  onSelectButton: (buttonId: string) => void;
}

export function KeyboardEditor({
  profile,
  selectedButtonId,
  activeKeys,
  runningButtonId,
  onSelectButton,
}: Props) {
  const setProfileLayout = useAppStore((s) => s.setProfileLayout);
  const ensureButtonForPhysicalKey = useAppStore(
    (s) => s.ensureButtonForPhysicalKey,
  );
  const layout = profile.layout ?? "numpad";
  const visibleIds = new Set(allKeysForLayout(layout).map((key) => key.id));
  const offLayoutButtons = profile.buttons.filter(
    (button) =>
      Boolean(button.physicalKeyId) && !visibleIds.has(button.physicalKeyId!),
  );

  const handleSelectKey = (physicalKeyId: string) => {
    const buttonId = ensureButtonForPhysicalKey(profile.id, physicalKeyId);
    if (buttonId) onSelectButton(buttonId);
  };

  const configuredCount = profile.buttons.filter((button) => button.actions.length > 0).length;
  const actionCount = profile.buttons.reduce(
    (total, button) => total + button.actions.length,
    0,
  );

  return (
    <div className="flex-1 min-w-0 flex flex-col overflow-hidden">
      <div className="flex flex-wrap items-center justify-between gap-3 px-5 py-3 border-b border-[var(--color-border)] bg-[var(--color-bg-secondary)]/60">
        <div>
          <div className="text-xs font-semibold text-[var(--color-text-primary)]">
            {profile.name} · Keyboard Layout
          </div>
          <div className="text-[10px] text-[var(--color-text-muted)] mt-0.5">
            {configuredCount} configured key{configuredCount === 1 ? "" : "s"} · {actionCount} action
            {actionCount === 1 ? "" : "s"} · click any key to edit or assign
          </div>
        </div>
        <div className="flex items-center gap-1 p-1 rounded-lg bg-[var(--color-bg-card)] border border-[var(--color-border)]">
          {LAYOUT_OPTIONS.map((option) => (
            <button
              key={option.value}
              type="button"
              onClick={() => setProfileLayout(profile.id, option.value)}
              title={option.description}
              className={`px-3 py-1.5 rounded-md text-[11px] transition-colors cursor-pointer ${
                layout === option.value
                  ? "bg-[var(--color-accent)] text-white font-medium"
                  : "text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:bg-[var(--color-bg-card-hover)]"
              }`}
            >
              {option.label}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 overflow-auto p-6">
        <KeyboardView
          layout={layout}
          buttons={profile.buttons}
          selectedButtonId={selectedButtonId}
          activeKeys={activeKeys}
          runningButtonId={runningButtonId}
          onSelectKey={handleSelectKey}
        />

        {offLayoutButtons.length > 0 && (
          <div className="mt-8 mx-auto max-w-3xl rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-card)] p-3">
            <div className="text-[9px] tracking-wider text-[var(--color-text-muted)] mb-2">
              ASSIGNED KEYS OUTSIDE THIS LAYOUT (PRESERVED)
            </div>
            <div className="flex flex-wrap gap-2">
              {offLayoutButtons.map((button) => (
                <button
                  key={button.id}
                  type="button"
                  onClick={() => onSelectButton(button.id)}
                  className="px-2 py-1 rounded-md text-[10px] border border-[var(--color-border)] text-[var(--color-text-secondary)] hover:border-[var(--color-accent)] cursor-pointer"
                >
                  {keyLabel(button.physicalKeyId!)} · {button.name}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
