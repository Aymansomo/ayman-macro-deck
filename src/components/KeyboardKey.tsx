import type { MacroButton } from "../types";
import type { PhysicalKey } from "../lib/keyboardLayouts";

interface Props {
  definition: PhysicalKey;
  button?: MacroButton;
  selected: boolean;
  active: boolean;
  running: boolean;
  unit: number;
  onSelect: (physicalKeyId: string) => void;
}

export function KeyboardKey({
  definition,
  button,
  selected,
  active,
  running,
  unit,
  onSelect,
}: Props) {
  const gap = 6;
  const width = unit * (definition.width ?? 1) + gap * ((definition.width ?? 1) - 1);
  const height =
    unit * (definition.rowSpan ?? 1) + gap * ((definition.rowSpan ?? 1) - 1);
  const assigned = Boolean(button);
  const enabled = button?.enabled ?? false;

  const toneClass = definition.tone === "accent"
    ? "text-[var(--color-accent)]"
    : definition.tone === "muted"
      ? "text-[var(--color-text-muted)]"
      : "text-[var(--color-text-secondary)]";

  return (
    <button
      type="button"
      onClick={() => onSelect(definition.id)}
      title={`${definition.id}${button ? ` → ${button.name}` : " — click to assign"}`}
      style={{ width, height, minWidth: width }}
      className={[
        "relative shrink-0 rounded-md border flex flex-col items-center justify-center overflow-hidden",
        "transition-all duration-100 select-none cursor-pointer",
        active
          ? "scale-[0.94] bg-[var(--color-accent)] border-[var(--color-accent)] text-white shadow-[0_0_22px_var(--color-accent-glow)]"
          : selected
            ? "bg-[var(--color-accent)]/20 border-[var(--color-accent)] shadow-[0_0_18px_var(--color-accent-glow)]"
            : button?.enabled
              ? "bg-[var(--color-bg-card-hover)] border-[var(--color-border-hover)] hover:border-[var(--color-accent)]"
              : "bg-[var(--color-bg-card)] border-[var(--color-border)] hover:border-[var(--color-border-hover)]",
        assigned ? "" : "border-dashed",
        enabled ? "" : "opacity-45",
        running ? "animate-pulse" : "",
      ].join(" ")}
    >
      <span className={`text-[11px] font-semibold leading-none ${active ? "text-white" : toneClass}`}>
        {definition.label || "Space"}
      </span>
      {assigned && button && (
        <>
          <span className="text-[13px] leading-none mt-1">{button.icon || "◆"}</span>
          <span
            className={`max-w-[92%] truncate text-[8px] leading-none mt-0.5 ${
              active ? "text-white/80" : "text-[var(--color-text-muted)]"
            }`}
          >
            {button.name}
          </span>
        </>
      )}
      {!assigned && (
        <span className="text-[7px] leading-none mt-1 text-[var(--color-text-muted)] opacity-70">
          assign
        </span>
      )}
      {definition.rowSpan && definition.rowSpan > 1 && (
        <span className="absolute bottom-1 text-[7px] text-[var(--color-text-muted)] tracking-widest">
          ⋮
        </span>
      )}
    </button>
  );
}
