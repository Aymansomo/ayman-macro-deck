import { LAYOUT_GROUPS } from "../lib/keyboardLayouts";
import type { KeyboardLayout, MacroButton } from "../types";
import { KeyboardKey } from "./KeyboardKey";

interface Props {
  layout: KeyboardLayout;
  buttons: MacroButton[];
  selectedButtonId: string | null;
  activeKeys: Set<string>;
  runningButtonId: string | null;
  onSelectKey: (physicalKeyId: string) => void;
}

const UNIT: Record<KeyboardLayout, number> = {
  numpad: 62,
  numpadNav: 46,
  full: 34,
};

export function KeyboardView({
  layout,
  buttons,
  selectedButtonId,
  activeKeys,
  runningButtonId,
  onSelectKey,
}: Props) {
  const byPhysicalId = new Map(
    buttons
      .filter((button) => Boolean(button.physicalKeyId))
      .map((button) => [button.physicalKeyId as string, button]),
  );
  const groups = LAYOUT_GROUPS[layout];
  const unit = UNIT[layout];

  return (
    <div className="flex flex-wrap items-start justify-center gap-x-8 gap-y-6">
      {groups.map((group) => (
        <section key={group.id} className="shrink-0">
          <div className="text-[9px] tracking-[0.18em] text-[var(--color-text-muted)] mb-2 uppercase">
            {group.title}
          </div>
          <div className="space-y-1.5">
            {group.rows.map((row, rowIndex) => (
              <div key={`${group.id}-${rowIndex}`} className="flex gap-1.5">
                {row.map((definition) => {
                  const button = byPhysicalId.get(definition.id);
                  return (
                    <KeyboardKey
                      key={definition.id}
                      definition={definition}
                      button={button}
                      selected={Boolean(button && button.id === selectedButtonId)}
                      active={activeKeys.has(definition.id)}
                      running={Boolean(button && button.id === runningButtonId)}
                      unit={unit}
                      onSelect={onSelectKey}
                    />
                  );
                })}
              </div>
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}
