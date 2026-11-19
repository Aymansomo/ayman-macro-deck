import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
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

const GAP = 8;
const LIMITS: Record<KeyboardLayout, { min: number; max: number }> = {
  numpad: { min: 62, max: 104 },
  numpadNav: { min: 52, max: 84 },
  full: { min: 44, max: 68 },
};

function rowUnits(row: { width?: number }[]): number {
  return row.reduce((total, key) => total + (key.width ?? 1), 0);
}

export function KeyboardView({
  layout,
  buttons,
  selectedButtonId,
  activeKeys,
  runningButtonId,
  onSelectKey,
}: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [available, setAvailable] = useState({ width: 900, height: 600 });

  useEffect(() => {
    const container = containerRef.current;
    const parent = container?.parentElement;
    if (!parent) return;

    const measure = () => {
      const width = parent.clientWidth || 900;
      const height = parent.clientHeight || 600;
      setAvailable((previous) =>
        previous.width === width && previous.height === height
          ? previous
          : { width, height },
      );
    };

    measure();
    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", measure);
      return () => window.removeEventListener("resize", measure);
    }

    const observer = new ResizeObserver(measure);
    observer.observe(parent);
    return () => observer.disconnect();
  }, []);

  const byPhysicalId = useMemo(
    () =>
      new Map(
        buttons
          .filter((button) => Boolean(button.physicalKeyId))
          .map((button) => [button.physicalKeyId as string, button]),
      ),
    [buttons],
  );

  const groups = LAYOUT_GROUPS[layout];

  const metrics = useMemo(() => {
    const widest = Math.max(
      1,
      ...groups.flatMap((group) => group.rows.map((row) => rowUnits(row))),
    );
    const tallest = Math.max(1, ...groups.map((group) => group.rows.length));
    return { widest, tallest };
  }, [groups]);

  const unit = useMemo(() => {
    const { min, max } = LIMITS[layout];
    const widthUnit =
      (Math.max(available.width - 56, min) - (metrics.widest - 1) * GAP) /
      metrics.widest;
    const heightUnit =
      (Math.max(available.height - 84, min * 3) -
        (metrics.tallest - 1) * GAP) /
      metrics.tallest;
    const fitted = Math.min(widthUnit, heightUnit);
    return Math.round(Math.max(min, Math.min(max, fitted)));
  }, [available, layout, metrics]);

  const style = {
    "--key-unit": `${unit}px`,
  } as CSSProperties;

  return (
    <div ref={containerRef} className="keyboard-groups" style={style}>
      {groups.map((group) => (
        <section key={group.id} className="keyboard-group">
          <div className="keyboard-group-title">{group.title}</div>
          <div
            className="keyboard-rows"
            style={{
              width: `${Math.max(
                ...group.rows.map((row) => {
                  const units = rowUnits(row);
                  return units * unit + (units - 1) * GAP;
                }),
              )}px`,
              height: `${group.rows.length * unit + (group.rows.length - 1) * GAP}px`,
            }}
          >
            {group.rows.map((row, rowIndex) => (
              <div
                key={`${group.id}-${rowIndex}`}
                className="keyboard-row"
                style={{ top: `${rowIndex * (unit + GAP)}px` }}
              >
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
                      gap={GAP}
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
