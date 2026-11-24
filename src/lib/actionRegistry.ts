import type { ActionDefinition } from "../types";
import { invoke } from "@tauri-apps/api/core";
import { useAppStore } from "../store";

export const ACTION_REGISTRY: ActionDefinition[] = [
  {
    type: "open_application",
    name: "Open Application",
    description: "Launch an application or open a shortcut",
    icon: "📂",
    paramSchema: [
      { key: "path", label: "Application Path", type: "path", placeholder: "C:\\...\\app.exe or .lnk" },
    ],
  },
  {
    type: "open_url",
    name: "Open URL",
    description: "Open a URL in the default browser",
    icon: "🌐",
    paramSchema: [
      { key: "url", label: "URL", type: "text", placeholder: "https://..." },
    ],
  },
  {
    type: "switch_steam_account",
    name: "Switch Steam Account",
    description: "Switch Steam account via shortcut",
    icon: "🎮",
    paramSchema: [
      {
        key: "account",
        label: "Account Name",
        type: "select",
        placeholder: "Select an account...",
      },
      {
        key: "waitUntilOpen",
        label: "Wait until account is open",
        type: "boolean",
        defaultValue: true,
      },
    ],
    async loadOptions(paramKey) {
      if (paramKey === "account") {
        return Object.keys(
          useAppStore.getState().config.settings.steam.profileShortcuts,
        );
      }
      return [];
    },
  },
  {
    type: "launch_steam_game",
    name: "Launch Steam Game",
    description: "Launch a Steam game by App ID",
    icon: "🎮",
    paramSchema: [
      { key: "appId", label: "Steam App ID", type: "text", placeholder: "1817070" },
      { key: "name", label: "Game Name", type: "text", placeholder: "Spider-Man" },
    ],
  },
  {
    type: "launch_epic_game",
    name: "Launch Epic Game",
    description: "Launch a game via Epic Games Store URI",
    icon: "🔫",
    paramSchema: [
      { key: "uri", label: "Epic URI", type: "text", placeholder: "com.epicgames.launcher://..." },
    ],
  },
  {
    type: "change_audio_device",
    name: "Change Audio Device",
    description: "Switch default audio output device",
    icon: "🎧",
    paramSchema: [
      { key: "device", label: "Device", type: "select", dynamic: "audio_devices" },
    ],
    async loadOptions(paramKey) {
      if (paramKey === "device" || paramKey === "audio_devices") {
        const svv = useAppStore.getState().config.settings.audio.soundVolumeViewPath;
        try {
          const devices = await invoke<string[]>("list_audio_devices", {
            soundVolumeViewPath: svv,
          });
          return devices;
        } catch {
          return [];
        }
      }
      return [];
    },
  },
  {
    type: "run_command",
    name: "Run Command",
    description: "Execute a shell command",
    icon: "💻",
    paramSchema: [
      { key: "command", label: "Command", type: "text", placeholder: "cmd.exe" },
      { key: "args", label: "Arguments (comma-separated)", type: "text", placeholder: "/c, echo, hello" },
    ],
  },
  {
    type: "run_ahk_script",
    name: "Run AHK Script",
    description: "Execute an AutoHotkey script",
    icon: "📜",
    paramSchema: [
      { key: "scriptPath", label: "Script Path", type: "path", placeholder: "E:\\...\\script.ahk" },
    ],
  },
  {
    type: "open_folder",
    name: "Open Folder",
    description: "Open a folder in File Explorer",
    icon: "📁",
    paramSchema: [
      { key: "path", label: "Folder Path", type: "path", placeholder: "C:\\Users\\..." },
    ],
  },
  {
    type: "toggle_gaming_mode",
    name: "Toggle Gaming Mode",
    description: "Toggle ScrollLock-based gaming mode",
    icon: "🎯",
    paramSchema: [],
  },
  {
    type: "keyboard_shortcut",
    name: "Keyboard Shortcut",
    description: "Click Record and press a key combination, e.g. Ctrl+A",
    icon: "⌨️",
    paramSchema: [
      { key: "keys", label: "Keys", type: "keycapture", placeholder: "Ctrl+Shift+M" },
    ],
  },
  {
    type: "play_sound",
    name: "Play Sound",
    description: "Play an audio file such as a WAV or MP3",
    icon: "🎵",
    paramSchema: [
      {
        key: "path",
        label: "Sound file",
        type: "audiofile",
        placeholder: "C:\\Sounds\\click.wav",
      },
    ],
  },
  {
    type: "media_play_pause",
    name: "Play / Pause Music",
    description: "Pause or resume whatever music is playing",
    icon: "⏯️",
    paramSchema: [],
  },
  {
    type: "media_next",
    name: "Skip Track",
    description: "Skip to the next song (Spotify, browser, any player)",
    icon: "⏭️",
    paramSchema: [],
  },
  {
    type: "media_previous",
    name: "Previous Track",
    description: "Go back to the previous song",
    icon: "⏮️",
    paramSchema: [],
  },
  {
    type: "media_stop",
    name: "Stop Music",
    description: "Stop the current music playback",
    icon: "⏹️",
    paramSchema: [],
  },
  {
    type: "volume_up",
    name: "Volume Up",
    description: "Increase system volume",
    icon: "🔊",
    paramSchema: [],
  },
  {
    type: "volume_down",
    name: "Volume Down",
    description: "Decrease system volume",
    icon: "🔉",
    paramSchema: [],
  },
  {
    type: "mute",
    name: "Mute/Unmute",
    description: "Toggle system mute",
    icon: "🔇",
    paramSchema: [],
  },
  {
    type: "open_page",
    name: "Open Page (Folder)",
    description: "Jump to a deck page, or back to the main page",
    icon: "📁",
    paramSchema: [
      {
        key: "page",
        label: "Page",
        type: "select",
        placeholder: "Main page (back)",
      },
    ],
    async loadOptions() {
      const state = useAppStore.getState();
      const profile = state.config.profiles.find(
        (candidate) => candidate.id === state.config.activeProfileId,
      );
      return ["Main", ...(profile?.pages ?? []).map((page) => page.name)];
    },
  },
  {
    type: "stop_all_audio",
    name: "Stop All Sound",
    description: "Stop every sound this deck is playing and the current music",
    icon: "🔕",
    paramSchema: [],
  },
  {
    type: "lock_workstation",
    name: "Lock PC",
    description: "Lock Windows immediately",
    icon: "🔒",
    paramSchema: [],
  },
  {
    type: "sleep_pc",
    name: "Sleep",
    description: "Put the PC to sleep",
    icon: "🌙",
    paramSchema: [],
  },
  {
    type: "sign_out",
    name: "Sign Out",
    description: "Sign out of the current Windows user",
    icon: "🚪",
    paramSchema: [],
  },
  {
    type: "restart_pc",
    name: "Restart PC",
    description:
      "Restart Windows. Needs a second press within 5 seconds (or a hold) to confirm",
    icon: "🔄",
    paramSchema: [],
  },
  {
    type: "shutdown_pc",
    name: "Shut Down PC",
    description:
      "Shut Windows down. Needs a second press within 5 seconds (or a hold) to confirm",
    icon: "⏻",
    paramSchema: [],
  },
  {
    type: "show_desktop",
    name: "Show Desktop",
    description: "Minimise everything (Win+D)",
    icon: "🖥️",
    paramSchema: [],
  },
  {
    type: "task_manager",
    name: "Task Manager",
    description: "Open Windows Task Manager",
    icon: "📊",
    paramSchema: [],
  },
  {
    type: "screenshot",
    name: "Screenshot",
    description: "Save a screenshot of every monitor to Pictures",
    icon: "📸",
    paramSchema: [],
  },
  {
    type: "empty_recycle_bin",
    name: "Empty Recycle Bin",
    description: "Delete everything in the Recycle Bin",
    icon: "🗑️",
    paramSchema: [],
  },
  {
    type: "switch_profile",
    name: "Switch Profile",
    description: "Jump to any profile when this button's key is pressed",
    icon: "🔄",
    paramSchema: [
      { key: "profile", label: "Switch to profile", type: "select", placeholder: "Choose a profile" },
    ],
    async loadOptions() {
      const profiles = useAppStore.getState().config.profiles;
      return profiles.map((p) => p.name);
    },
  },
];

export function getActionDef(type: string): ActionDefinition | undefined {
  return ACTION_REGISTRY.find((a) => a.type === type);
}

export function createAction(type: string): import("../types").ButtonAction {
  const def = getActionDef(type);
  const params: Record<string, string | number | boolean> = {};
  if (def) {
    for (const field of def.paramSchema) {
      if (field.defaultValue !== undefined) {
        params[field.key] = field.defaultValue;
      } else {
        params[field.key] = "";
      }
    }
  }
  return {
    id: crypto.randomUUID().slice(0, 8),
    type,
    name: def?.name ?? type,
    params,
  };
}
