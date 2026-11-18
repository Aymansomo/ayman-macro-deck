import type { KeyboardLayout } from "../types";

export interface PhysicalKey {
  /** Matches Rust `KeyId::id()` exactly. */
  id: string;
  label: string;
  hint?: string;
  width?: number;
  rowSpan?: number;
  tone?: "accent" | "muted";
}

export interface KeyGroup {
  id: string;
  title: string;
  rows: PhysicalKey[][];
}

export const LAYOUT_OPTIONS: Array<{
  value: KeyboardLayout;
  label: string;
  description: string;
}> = [
  {
    value: "numpad",
    label: "Numpad Only",
    description: "Classic 1-9, / and * deck",
  },
  {
    value: "numpadNav",
    label: "Numpad + Navigation",
    description: "Numpad, editing cluster and arrow keys",
  },
  {
    value: "full",
    label: "Full Keyboard",
    description: "Function row, typing block, navigation and numpad",
  },
];

export function normalizeLayout(value: unknown): KeyboardLayout {
  return value === "full" || value === "numpadNav" || value === "numpad"
    ? value
    : "numpad";
}

export function keyLabel(physicalKeyId: string): string {
  const all = LAYOUT_GROUPS.full.flatMap((group) => group.rows.flat());
  return all.find((key) => key.id === physicalKeyId)?.label ?? physicalKeyId;
}

/** Legacy 1-11 slot for old hardware/config compatibility. */
export function legacySlotForPhysicalKey(physicalKeyId: string): number | undefined {
  const numpad = /^Numpad([0-9])$/.exec(physicalKeyId);
  if (numpad) {
    const value = Number(numpad[1]);
    return value >= 1 && value <= 9 ? value : undefined;
  }
  if (physicalKeyId === "NumpadDivide") return 10;
  if (physicalKeyId === "NumpadMultiply") return 11;
  return undefined;
}

function k(
  id: string,
  label: string,
  options: Omit<PhysicalKey, "id" | "label"> = {},
): PhysicalKey {
  return { id, label, ...options };
}

const FUNCTION_ROW: PhysicalKey[] = [
  k("Scan001", "Esc", { tone: "muted" }),
  k("F1", "F1"), k("F2", "F2"), k("F3", "F3"), k("F4", "F4"),
  k("F5", "F5"), k("F6", "F6"), k("F7", "F7"), k("F8", "F8"),
  k("F9", "F9"), k("F10", "F10"), k("F11", "F11"), k("F12", "F12"),
  k("Scan037E0", "PrtSc", { tone: "muted" }),
  k("Scan046", "ScrLk", { tone: "muted" }),
];

const TYPING_ROWS: PhysicalKey[][] = [
  [
    k("Scan029", "`"), k("Digit1", "1"), k("Digit2", "2"), k("Digit3", "3"),
    k("Digit4", "4"), k("Digit5", "5"), k("Digit6", "6"), k("Digit7", "7"),
    k("Digit8", "8"), k("Digit9", "9"), k("Digit0", "0"),
    k("Scan00C", "-"), k("Scan00D", "="),
    k("Scan00E", "Backspace", { width: 2, tone: "muted" }),
  ],
  [
    k("Scan00F", "Tab", { width: 1.5, tone: "muted" }),
    k("KeyQ", "Q"), k("KeyW", "W"), k("KeyE", "E"), k("KeyR", "R"),
    k("KeyT", "T"), k("KeyY", "Y"), k("KeyU", "U"), k("KeyI", "I"),
    k("KeyO", "O"), k("KeyP", "P"),
    k("Scan01A", "["), k("Scan01B", "]"),
    k("Scan02B", "\\", { width: 1.5 }),
  ],
  [
    k("Scan03A", "Caps", { width: 1.75, tone: "muted" }),
    k("KeyA", "A"), k("KeyS", "S"), k("KeyD", "D"), k("KeyF", "F"),
    k("KeyG", "G"), k("KeyH", "H"), k("KeyJ", "J"), k("KeyK", "K"),
    k("KeyL", "L"),
    k("Scan027", ";"), k("Scan028", "'"),
    k("Scan01C", "Enter", { width: 2.25, tone: "accent" }),
  ],
  [
    k("Scan02A", "Shift", { width: 2.25, tone: "muted" }),
    k("KeyZ", "Z"), k("KeyX", "X"), k("KeyC", "C"), k("KeyV", "V"),
    k("KeyB", "B"), k("KeyN", "N"), k("KeyM", "M"),
    k("Scan033", ","), k("Scan034", "."), k("Scan035", "/"),
    k("Scan036", "Shift", { width: 2.75, tone: "muted" }),
  ],
  [
    k("Scan01D", "Ctrl", { width: 1.25, tone: "muted" }),
    k("Scan05BE0", "Win", { width: 1.25, tone: "muted" }),
    k("Scan038", "Alt", { width: 1.25, tone: "muted" }),
    k("Scan039", "", { width: 6.25 }),
    k("Scan038E0", "Alt", { width: 1.25, tone: "muted" }),
    k("Scan05CE0", "Win", { width: 1.25, tone: "muted" }),
    k("Scan05DE0", "Menu", { width: 1.25, tone: "muted" }),
    k("Scan01DE0", "Ctrl", { width: 1.25, tone: "muted" }),
  ],
];

const NUMPAD_ROWS: PhysicalKey[][] = [
  [
    k("Scan045", "Num", { tone: "muted" }),
    k("NumpadDivide", "/", { tone: "accent" }),
    k("NumpadMultiply", "*", { tone: "accent" }),
    k("NumpadSubtract", "-"),
  ],
  [
    k("Numpad7", "7"), k("Numpad8", "8"), k("Numpad9", "9"),
    k("NumpadAdd", "+", { rowSpan: 2 }),
  ],
  [
    k("Numpad4", "4"), k("Numpad5", "5"), k("Numpad6", "6"),
  ],
  [
    k("Numpad1", "1"), k("Numpad2", "2"), k("Numpad3", "3"),
    k("NumpadEnter", "Enter", { rowSpan: 2, tone: "accent" }),
  ],
  [
    k("Numpad0", "0", { width: 2 }), k("NumpadDecimal", "."),
  ],
];

const EDITING_ROWS: PhysicalKey[][] = [
  [
    k("Insert", "Ins", { tone: "muted" }),
    k("Home", "Home", { tone: "muted" }),
    k("PageUp", "PgUp", { tone: "muted" }),
  ],
  [
    k("Delete", "Del", { tone: "muted" }),
    k("End", "End", { tone: "muted" }),
    k("PageDown", "PgDn", { tone: "muted" }),
  ],
];

const ARROW_ROWS: PhysicalKey[][] = [
  [k("ArrowUp", "↑", { width: 1, tone: "accent" })],
  [
    k("ArrowLeft", "←", { tone: "accent" }),
    k("ArrowDown", "↓", { tone: "accent" }),
    k("ArrowRight", "→", { tone: "accent" }),
  ],
];

export const LAYOUT_GROUPS: Record<KeyboardLayout, KeyGroup[]> = {
  numpad: [
    { id: "numpad", title: "Numpad", rows: NUMPAD_ROWS },
  ],
  numpadNav: [
    { id: "numpad", title: "Numpad", rows: NUMPAD_ROWS },
    { id: "editing", title: "Editing", rows: EDITING_ROWS },
    { id: "arrows", title: "Arrows", rows: ARROW_ROWS },
  ],
  full: [
    { id: "function", title: "Function", rows: [FUNCTION_ROW] },
    { id: "typing", title: "Typing", rows: TYPING_ROWS },
    { id: "editing", title: "Editing", rows: EDITING_ROWS },
    { id: "arrows", title: "Arrows", rows: ARROW_ROWS },
    { id: "numpad", title: "Numpad", rows: NUMPAD_ROWS },
  ],
};

export function allKeysForLayout(layout: KeyboardLayout): PhysicalKey[] {
  return LAYOUT_GROUPS[layout].flatMap((group) => group.rows.flat());
}
