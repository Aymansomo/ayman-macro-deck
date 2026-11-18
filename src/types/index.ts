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
}

export interface Profile {
  id: string;
  name: string;
  layout?: KeyboardLayout;
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
  type: "text" | "number" | "select" | "path" | "boolean";
  options?: string[];
  placeholder?: string;
  defaultValue?: string | number | boolean;
  dynamic?: string;
}
