import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, Profile, MacroButton } from "../types";

function generateId(): string {
  return crypto.randomUUID().slice(0, 8);
}

function createDefaultButton(key: number): MacroButton {
  return {
    id: generateId(),
    key,
    name: `Key ${key}`,
    icon: "",
    description: "",
    enabled: true,
    actions: [],
  };
}

function createProfileSwitchButton(
  key: 10 | 11,
  target: "Gaming" | "Normal",
): MacroButton {
  const label = key === 10 ? "/" : "*";
  return {
    id: generateId(),
    key,
    name: target === "Gaming" ? "Switch to Gaming" : "Switch to Normal",
    icon: target === "Gaming" ? "🎮" : "🖥",
    description: `Numpad ${label} — switch to ${target} profile`,
    enabled: true,
    actions: [
      {
        id: generateId(),
        type: "switch_profile",
        name: "Switch Profile",
        params: { profile: target },
      },
    ],
  };
}

function createDefaultProfile(name: string): Profile {
  const buttons: MacroButton[] = [];
  for (let i = 1; i <= 9; i++) {
    buttons.push(createDefaultButton(i));
  }
  buttons.push(createProfileSwitchButton(10, "Gaming"));
  buttons.push(createProfileSwitchButton(11, "Normal"));
  return {
    id: generateId(),
    name,
    buttons,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

function createGamingProfile(): Profile {
  const buttons: MacroButton[] = [
    {
      id: generateId(),
      key: 1,
      name: "Steam SOMO",
      icon: "🎮",
      description: "Switch to Steam SOMO",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "switch_steam_account",
          name: "Switch Steam Account",
          params: { account: "SOMO" },
        },
      ],
    },
    {
      id: generateId(),
      key: 2,
      name: "Spider-Man",
      icon: "🕷️",
      description: "Switch to spydey and launch Spider-Man",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "switch_steam_account",
          name: "Switch Steam Account",
          params: { account: "spydey" },
        },
        {
          id: generateId(),
          type: "launch_steam_game",
          name: "Launch Steam Game",
          params: { appId: "1817070", name: "Spider-Man" },
          delayAfter: 5000,
        },
      ],
    },
    {
      id: generateId(),
      key: 3,
      name: "Khazan",
      icon: "⚔️",
      description: "Switch to Khazan and launch The First Berserker",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "switch_steam_account",
          name: "Switch Steam Account",
          params: { account: "Khazan The Berserker" },
        },
        {
          id: generateId(),
          type: "launch_steam_game",
          name: "Launch Steam Game",
          params: { appId: "2680010", name: "The First Berserker: Khazan" },
          delayAfter: 5000,
        },
      ],
    },
    {
      id: generateId(),
      key: 4,
      name: "Discord",
      icon: "💬",
      description: "Open Discord",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "C:\\Users\\Ayman\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu\\Programs\\Discord Inc\\Discord.lnk",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 5,
      name: "Fortnite",
      icon: "🔫",
      description: "Launch Fortnite via Epic Games",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "launch_epic_game",
          name: "Launch Epic Game",
          params: {
            appId: "Fortnite",
            uri: "com.epicgames.launcher://apps/Fortnite?action=launch&silent=true",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 6,
      name: "Bluetooth Audio",
      icon: "🔊",
      description: "Open Bluetooth Audio Receiver",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "explorer.exe shell:AppsFolder\\55746MarkSmirnov.BluetoothAudioReveicer_xwrbx6997tsfc!App",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 7,
      name: "WhatsApp",
      icon: "💬",
      description: "Open WhatsApp",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "explorer.exe shell:AppsFolder\\5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 8,
      name: "Playnite",
      icon: "🎮",
      description: "Launch Playnite Fullscreen",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "C:\\Users\\Ayman\\AppData\\Local\\Playnite\\Playnite.FullscreenApp.exe",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 9,
      name: "YouTube",
      icon: "▶️",
      description: "Open YouTube",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_url",
          name: "Open URL",
          params: { url: "https://www.youtube.com/" },
        },
      ],
    },
    createProfileSwitchButton(10, "Gaming"),
    createProfileSwitchButton(11, "Normal"),
  ];
  return {
    id: generateId(),
    name: "Gaming",
    buttons,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

function createNormalProfile(): Profile {
  const buttons: MacroButton[] = [
    {
      id: generateId(),
      key: 1,
      name: "Instagram DMs",
      icon: "📩",
      description: "Open Instagram direct inbox",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_url",
          name: "Open URL",
          params: { url: "https://www.instagram.com/direct/inbox/" },
        },
      ],
    },
    {
      id: generateId(),
      key: 2,
      name: "Headset",
      icon: "🎧",
      description: "Switch default audio to headset",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "change_audio_device",
          name: "Change Audio Device",
          params: { device: "Realtek HD Audio 2nd output" },
        },
      ],
    },
    {
      id: generateId(),
      key: 3,
      name: "Speakers",
      icon: "🔊",
      description: "Switch default audio to speakers",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "change_audio_device",
          name: "Change Audio Device",
          params: { device: "Haut-parleurs" },
        },
      ],
    },
    {
      id: generateId(),
      key: 4,
      name: "Spotify",
      icon: "🎵",
      description: "Open Spotify",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_url",
          name: "Open URL",
          params: { url: "spotify:" },
        },
      ],
    },
    {
      id: generateId(),
      key: 5,
      name: "Notion",
      icon: "📝",
      description: "Open Notion",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "C:\\Users\\Ayman\\AppData\\Local\\Programs\\Notion\\Notion.exe",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 6,
      name: "Bluetooth Audio",
      icon: "🔊",
      description: "Open Bluetooth Audio Receiver",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "explorer.exe shell:AppsFolder\\55746MarkSmirnov.BluetoothAudioReveicer_xwrbx6997tsfc!App",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 7,
      name: "WhatsApp",
      icon: "💬",
      description: "Open WhatsApp",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "explorer.exe shell:AppsFolder\\5319275A.WhatsAppDesktop_cv1g1gvanyjgm!App",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 8,
      name: "Stremio",
      icon: "🎬",
      description: "Open Stremio",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_application",
          name: "Open Application",
          params: {
            path: "C:\\Users\\Ayman\\AppData\\Local\\Programs\\Stremio\\stremio-shell-ng.exe",
          },
        },
      ],
    },
    {
      id: generateId(),
      key: 9,
      name: "YouTube",
      icon: "▶️",
      description: "Open YouTube",
      enabled: true,
      actions: [
        {
          id: generateId(),
          type: "open_url",
          name: "Open URL",
          params: { url: "https://www.youtube.com/" },
        },
      ],
    },
    createProfileSwitchButton(10, "Gaming"),
    createProfileSwitchButton(11, "Normal"),
  ];
  return {
    id: generateId(),
    name: "Normal",
    buttons,
    createdAt: Date.now(),
    updatedAt: Date.now(),
  };
}

function createDefaultConfig(): AppConfig {
  const gamingProfile = createGamingProfile();
  const normalProfile = createNormalProfile();
  return {
    version: 1,
    activeProfileId: gamingProfile.id,
    profiles: [gamingProfile, normalProfile],
    settings: {
      general: {
        startWithWindows: false,
        minimizeToTray: true,
        startMinimized: false,
        notifications: true,
      },
      keyboard: {
        ahkExecutablePath: "C:\\Program Files\\AutoHotkey\\v2\\AutoHotkey.exe",
        ahkScriptPath:
          "E:\\ayman\\ME\\Macro\\script\\steam_mode.ahk",
      },
      steam: {
        steamExecutable: "C:\\Program Files (x86)\\Steam\\steam.exe",
        profileShortcutsFolder: "E:\\ayman\\ME\\Macro\\script\\SteamProfiles",
        profileShortcuts: {
          SOMO: "E:\\ayman\\ME\\Macro\\script\\SteamProfiles\\Switch to SOMO  [Steam].lnk",
          Tesk: "E:\\ayman\\ME\\Macro\\script\\SteamProfiles\\Switch to Tesk (Offline) [Steam].lnk",
          spydey: "E:\\ayman\\ME\\Macro\\script\\SteamProfiles\\Switch to spydey (Offline) [Steam].lnk",
          "Khazan The Berserker":
            "E:\\ayman\\ME\\Macro\\script\\SteamProfiles\\Switch to Khazan The Berserker (Offline) [Steam].lnk",
          "pixel malandro":
            "E:\\ayman\\ME\\Macro\\script\\SteamProfiles\\Switch to pixel malandro (Offline) [Steam].lnk",
        },
      },
      audio: {
        soundVolumeViewPath:
          "E:\\ayman\\ME\\Macro\\SoundVolumeView\\SoundVolumeView.exe",
        devices: {
          Headset: "Realtek HD Audio 2nd output",
          Speakers: "Haut-parleurs",
        },
      },
      appearance: {
        accentColor: "#7c3aed",
        buttonSize: 100,
        gridSize: 3,
      },
      rdr2: {
        savesPath:
          "C:\\Users\\Ayman\\Documents\\Rockstar Games\\Red Dead Redemption 2\\Profiles",
        backupPath: "E:\\ayman\\ME\\Macro\\RDR2Backup\\Saves",
        installerPath:
          "C:\\Users\\Ayman\\Downloads\\Application\\Rockstar-Games-Launcher.exe",
        uninstallCommand: "",
        confirmBeforeUninstall: true,
      },
      images: {
        steamgriddbApiKey: "",
      },
    },
  };
}

export const RDR2_DEFAULTS = {
  savesPath:
    "C:\\Users\\Ayman\\Documents\\Rockstar Games\\Red Dead Redemption 2\\Profiles",
  backupPath: "E:\\ayman\\ME\\Macro\\RDR2Backup\\Saves",
  installerPath:
    "C:\\Users\\Ayman\\Downloads\\Application\\Rockstar-Games-Launcher.exe",
  uninstallCommand: "",
  confirmBeforeUninstall: true,
} as const;

export const IMAGE_DEFAULTS = {
  steamgriddbApiKey: "",
} as const;

interface AppStore {
  config: AppConfig;
  isLoading: boolean;
  isGamingMode: boolean;
  keyboardConnected: boolean;

  setActiveProfile: (profileId: string) => void;
  updateButton: (profileId: string, button: MacroButton) => void;
  createProfile: (name: string) => void;
  deleteProfile: (profileId: string) => void;
  renameProfile: (profileId: string, name: string) => void;
  duplicateProfile: (profileId: string, newName: string) => void;
  updateRdr2Settings: (patch: Partial<AppConfig["settings"]["rdr2"]>) => void;
  setImageSearchApiKey: (key: string) => void;
  setSteamProfilesFolder: (folder: string) => void;
  addSteamAccount: (account: string, shortcutPath: string) => void;
  renameSteamAccount: (oldName: string, newName: string) => void;
  removeSteamAccount: (account: string) => void;
  setGamingMode: (on: boolean) => void;
  setKeyboardConnected: (connected: boolean) => void;
  loadConfig: (config: AppConfig) => void;
  saveConfig: () => Promise<void>;
}

const CP1252_HIGH: Record<number, number> = {
  0x20ac: 0x80, 0x201a: 0x82, 0x0192: 0x83, 0x201e: 0x84, 0x2026: 0x85,
  0x2020: 0x86, 0x2021: 0x87, 0x02c6: 0x88, 0x2030: 0x89, 0x0160: 0x8a,
  0x2039: 0x8b, 0x0152: 0x8c, 0x017d: 0x8e, 0x2018: 0x91, 0x2019: 0x92,
  0x201c: 0x93, 0x201d: 0x94, 0x2022: 0x95, 0x2013: 0x96, 0x2014: 0x97,
  0x02dc: 0x98, 0x2122: 0x99, 0x0161: 0x9a, 0x203a: 0x9b, 0x0153: 0x9c,
  0x017e: 0x9e, 0x0178: 0x9f,
};

function cp1252Bytes(input: string): Uint8Array {
  const bytes = new Uint8Array(input.length);
  for (let i = 0; i < input.length; i++) {
    const code = input.charCodeAt(i);
    bytes[i] =
      code < 0x80 ? code : CP1252_HIGH[code] ?? (code <= 0xff ? code : 0x3f);
  }
  return bytes;
}

const strictUtf8 = new TextDecoder("utf-8", { fatal: true });

/**
 * Un-scrambles double-encoded UTF-8 mojibake (e.g. an emoji that was saved
 * through a non-UTF-8 editor and shows up as "Ã°Å¸Å½Â®" instead of "🎮").
 * Leaves strings that are already valid text untouched.
 */
function repairMojibake(input: string): string {
  if (!input) return input;
  let s = input;
  const isMojibake = (str: string) => /[\u00c0-\u00ff]/.test(str);
  for (let i = 0; i < 4; i++) {
    if (!isMojibake(s)) break;
    let next: string;
    try {
      next = strictUtf8.decode(cp1252Bytes(s));
    } catch {
      break;
    }
    if (next === s || next.includes("\uFFFD")) break;
    s = next;
  }
  return s;
}

export function normalizeProfile(profile: Profile): Profile {
  const cleaned: Profile & { oldKeyboardButtons?: never } = {
    ...profile,
    buttons: Array.isArray(profile.buttons)
      ? profile.buttons.map((b) => ({
          ...b,
          icon: repairMojibake(b.icon),
          description: repairMojibake(b.description),
        }))
      : Array.from({ length: 11 }, (_, i) => createDefaultButton(i + 1)),
  };
  delete cleaned.oldKeyboardButtons;
  return cleaned;
}

export const useAppStore = create<AppStore>((set, get) => ({
  config: createDefaultConfig(),
  isLoading: true,
  isGamingMode: false,
  keyboardConnected: false,

  setActiveProfile: (profileId) =>
    set((state) => ({
      config: { ...state.config, activeProfileId: profileId },
    })),

  updateButton: (profileId, button) =>
    set((state) => ({
      config: {
        ...state.config,
        profiles: state.config.profiles.map((p) =>
          p.id === profileId
            ? {
                ...p,
                buttons: p.buttons.map((b) =>
                  b.id === button.id ? button : b,
                ),
                updatedAt: Date.now(),
              }
            : p,
        ),
      },
    })),

  createProfile: (name) => {
    const profile = createDefaultProfile(name);
    set((state) => ({
      config: {
        ...state.config,
        profiles: [...state.config.profiles, profile],
      },
    }));
  },

  deleteProfile: (profileId) =>
    set((state) => {
      const remaining = state.config.profiles.filter(
        (p) => p.id !== profileId,
      );
      const newActive =
        state.config.activeProfileId === profileId
          ? remaining[0]?.id ?? ""
          : state.config.activeProfileId;
      return {
        config: {
          ...state.config,
          activeProfileId: newActive,
          profiles: remaining,
        },
      };
    }),

  renameProfile: (profileId, name) =>
    set((state) => ({
      config: {
        ...state.config,
        profiles: state.config.profiles.map((p) =>
          p.id === profileId ? { ...p, name, updatedAt: Date.now() } : p,
        ),
      },
    })),

  duplicateProfile: (profileId, newName) =>
    set((state) => {
      const original = state.config.profiles.find(
        (p) => p.id === profileId,
      );
      if (!original) return state;
      const duplicate: Profile = {
        ...structuredClone(original),
        id: generateId(),
        name: newName,
        createdAt: Date.now(),
        updatedAt: Date.now(),
      };
      return {
        config: {
          ...state.config,
          profiles: [...state.config.profiles, duplicate],
        },
      };
    }),

  updateRdr2Settings: (patch) =>
    set((state) => ({
      config: {
        ...state.config,
        settings: {
          ...state.config.settings,
          rdr2: { ...RDR2_DEFAULTS, ...state.config.settings.rdr2, ...patch },
        },
      },
    })),

  setImageSearchApiKey: (key) =>
    set((state) => ({
      config: {
        ...state.config,
        settings: {
          ...state.config.settings,
          images: {
            ...IMAGE_DEFAULTS,
            ...state.config.settings.images,
            steamgriddbApiKey: key,
          },
        },
      },
    })),

  setSteamProfilesFolder: (folder) =>
    set((state) => ({
      config: {
        ...state.config,
        settings: {
          ...state.config.settings,
          steam: { ...state.config.settings.steam, profileShortcutsFolder: folder },
        },
      },
    })),

  addSteamAccount: (account, shortcutPath) =>
    set((state) => {
      const trimmed = account.trim();
      const path = shortcutPath.trim();
      if (!trimmed || !path) return state;
      const steam = state.config.settings.steam;
      if (steam.profileShortcuts[trimmed] === path) return state;
      return {
        config: {
          ...state.config,
          settings: {
            ...state.config.settings,
            steam: {
              ...steam,
              profileShortcuts: { ...steam.profileShortcuts, [trimmed]: path },
            },
          },
        },
      };
    }),

  renameSteamAccount: (oldName, newName) =>
    set((state) => {
      const trimmed = newName.trim();
      const steam = state.config.settings.steam;
      const shortcuts = steam.profileShortcuts;
      if (!trimmed || !(oldName in shortcuts) || trimmed === oldName)
        return state;
      if (trimmed in shortcuts) return state;
      const next = { ...shortcuts };
      next[trimmed] = next[oldName];
      delete next[oldName];
      return {
        config: {
          ...state.config,
          settings: {
            ...state.config.settings,
            steam: { ...steam, profileShortcuts: next },
          },
        },
      };
    }),

  removeSteamAccount: (account) =>
    set((state) => {
      const steam = state.config.settings.steam;
      if (!(account in steam.profileShortcuts)) return state;
      const next = { ...steam.profileShortcuts };
      delete next[account];
      return {
        config: {
          ...state.config,
          settings: {
            ...state.config.settings,
            steam: { ...steam, profileShortcuts: next },
          },
        },
      };
    }),

  setGamingMode: (on) => set({ isGamingMode: on }),
  setKeyboardConnected: (connected) => set({ keyboardConnected: connected }),
  loadConfig: (config) =>
    set({
      config: {
        ...config,
        profiles: config.profiles.map(normalizeProfile),
        settings: {
          ...config.settings,
          steam: {
            ...config.settings.steam,
            profileShortcutsFolder:
              config.settings.steam?.profileShortcutsFolder ??
              "E:\\ayman\\ME\\Macro\\script\\SteamProfiles",
          },
          rdr2: { ...RDR2_DEFAULTS, ...config.settings?.rdr2 },
          images: { ...IMAGE_DEFAULTS, ...config.settings?.images },
        },
      },
      isLoading: false,
    }),
  saveConfig: () => {
    const { config } = get();
    const json = JSON.stringify(config);
    return invoke<void>("save_config", { config: json }).catch(() => {
      localStorage.setItem("macrodeck-config", json);
    });
  },
}));
