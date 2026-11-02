# AYMAN MACRO DECK - Project State

## Last Updated: Phase 3 Complete

---

## Current Status

**Phase 3 (Button Editor) is COMPLETE and working.**

The app is a **configuration editor** (not a click-to-execute Stream Deck). Physical keyboard + AHK handles execution.

---

## What We Built

### Phase 1: Foundation ✅
- Tauri 2 + React + TypeScript + Vite + Tailwind CSS v4
- Dark gaming-style UI with 3x3 button grid
- Zustand store for state management
- JSON config persistence via Rust backend (`%APPDATA%\com.ayman.macrodeck\config.json`)
- Default "Gaming" profile with 9 pre-configured key mappings
- MSVC Build Tools installed

### Phase 2: Profile Management ✅
- Slide-out profile panel (create, rename, duplicate, delete with confirmation)
- Profile selector dropdown in status bar
- Header with gear icon to open profile manager

### Phase 3: Button Editor ✅
- Click button to select and edit (NOT execute)
- Right-side editor panel with:
  - Icon, name, description fields
  - Enable/disable toggle
  - Key display (Numpad 1-9)
- Action registry with 14 action types
- Macro editor (add/remove/reorder actions, delays)
- All changes auto-save instantly

---

## How It Works Now

```
┌─────────────────────────────────────────────────────────┐
│  AYMAN MACRO DECK                     [Profile] [⚙]     │
├────────────────────────────┬────────────────────────────┤
│                            │  Edit Button               │
│   ┌──────┐ ┌──────┐       │  ┌─────────────────────┐   │
│   │ 🎮   │ │ 🕷️   │       │  │ KEY: 1    [Enabled]  │   │
│   │Steam │ │Spider│  ◄──  │  ├─────────────────────┤   │
│   │ SOMO │ │ Man  │ click │  │ ICON: [🎮]           │   │
│   └──────┘ └──────┘       │  │ NAME: [Steam SOMO]   │   │
│   ┌──────┐ ┌──────┐       │  │ DESC: [Switch Steam]  │   │
│   │ ⚔️   │ │ 💬   │       │  ├─────────────────────┤   │
│   │Khazan│ │Disc. │       │  │ ACTIONS (1)          │   │
│   └──────┘ └──────┘       │  │ ▸ Switch Steam Acct  │   │
│   ...                     │  │   Account: [SOMO]    │   │
│                            │  │ + Add Action         │   │
├────────────────────────────┴────────────────────────────┤
│  [Profile ▼]           ● Normal Mode   ● Keyboard OK    │
└─────────────────────────────────────────────────────────┘
```

**User flow:**
1. Click button card → Opens editor panel on right
2. Edit icon, name, description
3. Add/remove/reorder actions (macros)
4. All changes auto-save to JSON config

---

## Next Steps

### Phase 4: AHK Integration
- Connect editor config to AHK script (`E:\ayman\ME\Macro\script\steam_mode.ahk`)
- Physical keyboard detection (via AHK)
- Gaming mode sync (toggle from UI, sync with AHK)
- Test action from editor (execute single action via Rust → AHK)

### Phase 5: Settings Page
- Steam account paths configuration
- Audio device selection
- Application paths (Playnite, etc.)
- Global settings (startup, hotkeys)

### Phase 6: Visual Macro Editor
- Drag-and-drop action reordering
- Visual delay editor
- Action grouping
- Copy/paste actions between buttons

### Phase 7: System Features
- System tray integration
- Windows startup option
- Import/Export profiles
- Profile backup/restore

### Phase 8: Polish
- Performance optimization (minimal CPU/RAM when idle)
- Error handling and logging
- Keyboard shortcuts for power users
- Final packaging and installer

---

## Key Files

### Project Root
`E:\ayman\Development\My progicte\ME\macro app\ayman-macro-deck\`

### Source Files
| File | Purpose |
|------|---------|
| `src\components\MacroButton.tsx` | Edit-mode button card (click to select, not execute) |
| `src\components\ButtonGrid.tsx` | 3x3 grid with selection tracking |
| `src\components\ButtonEditor.tsx` | Right-side editor panel (icon, name, description, actions/macro) |
| `src\components\Header.tsx` | App header with ProfileManager trigger |
| `src\components\ProfileManager.tsx` | Slide-out profile CRUD panel |
| `src\components\StatusBar.tsx` | Bottom bar with profile selector and status indicators |
| `src\components\ConfirmationDialog.tsx` | Reusable confirm/danger dialog |
| `src\lib\actionRegistry.ts` | 14 action types with param schemas, createAction factory |
| `src\lib\actions.ts` | Frontend action executor (invoke Rust execute_action) |
| `src\store\index.ts` | Zustand store (config, profiles, buttons, CRUD, auto-save) |
| `src\types\index.ts` | TypeScript interfaces (Profile, MacroButton, ButtonAction, AppConfig, etc.) |
| `src\index.css` | Tailwind v4 theme + custom animations |
| `src-tauri\src\lib.rs` | Rust backend (load_config, save_config, execute_action) |
| `src-tauri\Cargo.toml` | Dependencies (open, windows crate for ScrollLock) |
| `src-tauri\tauri.conf.json` | Window config (900x700, title "AYMAN MACRO DECK") |

### External Files (AHK Engine)
| File | Purpose |
|------|---------|
| `E:\ayman\ME\Macro\script\steam_mode.ahk` | Main AHK engine (323 lines, untouched) |
| `E:\ayman\ME\Macro\SoundVolumeView\SoundVolumeView.exe` | Audio device switcher |
| `E:\ayman\ME\Macro\script\SteamProfiles\*.lnk` | Steam account shortcuts (SOMO, Tesk, spydey, Khazan) |
| `E:\ayman\ME\Macro\script\Games\*.lnk` | Game shortcuts (Sekiro, Elden Ring, etc.) |

---

## Build Commands

### Development
```bash
cd "E:\ayman\Development\My progicte\ME\macro app\ayman-macro-deck"
pnpm tauri dev
```

### Production Build
```bash
# First, set up MSVC environment
cmd /c "`"C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvarsall.bat`" x64 && pnpm tauri build"
```

### Kill Running App (before rebuild)
```powershell
Stop-Process -Name "ayman-macro-deck" -Force -ErrorAction SilentlyContinue
```

---

## Important Notes

1. **AHK script is untouched** - We preserve the existing AHK engine
2. **App is editor, not executor** - Physical keyboard + AHK does execution
3. **Tailwind CSS v4** uses `@theme` directive (not v3 config file)
4. **Config auto-saves** with 500ms debounce in App.tsx
5. **MSVC Build Tools required** for Rust compilation (installed at `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`)

---

## Action Registry (14 Types)

1. Open Application
2. Open URL
3. Switch Steam Account
4. Launch Steam Game
5. Launch Epic Game
6. Change Audio Device
7. Run Command
8. Run AHK Script
9. Open Folder
10. Toggle Gaming Mode
11. Keyboard Shortcut
12. Volume Up
13. Volume Down
14. Mute/Unmute

---

## AHK Key Mappings (Gaming Profile)

| Button | AHK Key | Action |
|--------|---------|--------|
| 1 | Numpad1 | Switch to SOMO Steam account |
| 2 | Numpad2 | Switch to spydey + launch Spider-Man |
| 3 | Numpad3 | Switch to Khazan + launch The First Berserker |
| 4 | Numpad4 | Discord |
| 5 | Numpad5 | Fortnite (Epic) |
| 6 | Numpad6 | Bluetooth Audio Receiver |
| 7 | Numpad7 | WhatsApp |
| 8 | Numpad8 | Playnite |
| 9 | Numpad9 | Toggle Gaming Mode |

---

## Ready for Phase 4

**Next task:** Connect the editor config to the AHK script via Rust backend commands, so changes in the UI can be tested/executed through AHK.

Say **"next"** to continue with Phase 4 (AHK Integration).
