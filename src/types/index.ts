export interface ButtonAction {
  id: string;
  type: string;
  name: string;
  params: Record<string, string | number | boolean>;
  delayAfter?: number;
}

export type KeyboardLayout = "full" | "numpad" | "numpadNav";

export interface MacroButton {
  id: string;
  key: number;
  keyName?: string;
  /** Stable Raw Input identity; preferred over the legacy `key` slot. */
  physicalKeyId?: string;
  /** Explicit physical trigger written for keys outside the legacy 1-11 slots. */
  trigger?: string;
  name: string;
  icon: string;
  imagePath?: string;
  description: string;
  enabled: boolean;
  backgroundColor?: string;
  iconColor?: string;
  actions: ButtonAction[];
  /** Actions that run when the key is held down instead of tapped. */
  longPressActions?: ButtonAction[];
  /** Hold threshold in milliseconds (default 450). */
  longPressMs?: number;
  /** Press alternates between `actions` and `alternateActions`. */
  toggle?: boolean;
  alternateActions?: ButtonAction[];
  /** System-wide hotkey such as `Ctrl+Alt+F9`. */
  globalHotkey?: string;
}

/** A deck page (folder) with its own key assignments. */
export interface ProfilePage {
  id: string;
  name: string;
  buttons: MacroButton[];
}

export type ProfilePlacement = "keep" | "full" | "left" | "right" | "top" | "bottom";

/** An app a profile starts (and optionally moves to a monitor) when activated. */
export interface ProfileApp {
  id: string;
  path: string;
  args?: string;
  /** Zero-based index from `list_monitors()`; only a fallback for the name. */
  monitor?: number;
  /** Monitor device name, e.g. `DISPLAY2`: what survives rearranging screens. */
  monitorName?: string;
  placement?: ProfilePlacement;
}

export interface Profile {
  id: string;
  name: string;
  layout?: KeyboardLayout;
  /** Executable name that activates this profile while it is in front. */
  linkedApp?: string;
  /** Apps started when this profile becomes active. */
  apps?: ProfileApp[];
  /** Extra pages (folders); `buttons` is the main page. */
  pages?: ProfilePage[];
  buttons: MacroButton[];
  createdAt: number;
  updatedAt: number;
}

export interface AppConfig {
  version: number;
  activeProfileId: string;
  profiles: Profile[];
  settings: AppSettings;
}

export interface AppSettings {
  general: {
    startWithWindows: boolean;
    minimizeToTray: boolean;
    startMinimized: boolean;
    notifications: boolean;
    /** Switch profile automatically when a linked app is in the foreground. */
    autoSwitchProfiles?: boolean;
  };
  keyboard: {
    ahkExecutablePath: string;
    ahkScriptPath: string;
  };
  steam: {
    steamExecutable: string;
    profileShortcuts: Record<string, string>;
    profileShortcutsFolder: string;
  };
  audio: {
    soundVolumeViewPath: string;
    devices: Record<string, string>;
  };
  appearance: {
    accentColor: string;
    buttonSize: number;
    gridSize: number;
  };
  images: {
    steamgriddbApiKey: string;
  };
}

export interface ImageSearchResult {
  title: string;
  url: string;
  thumbnail: string;
  source: string;
}

export interface ActionDefinition {
  type: string;
  name: string;
  description: string;
  icon: string;
  paramSchema: ParamField[];
  loadOptions?: (paramKey: string) => Promise<string[]>;
}

export interface ParamField {
  key: string;
  label: string;
  type:
    | "text"
    | "number"
    | "select"
    | "path"
    | "boolean"
    | "keycapture"
    | "audiofile";
  options?: string[];
  placeholder?: string;
  defaultValue?: string | number | boolean;
  dynamic?: string;
}
