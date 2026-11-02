# AGENTS.md

Project-wide reference for AI agents. Read this first before making changes.

---

## What This Project Is

**AYMAN MACRO DECK** — a Windows macro deck that turns the computer keyboard's **number pad (Numpad1-9, plus `/` and `*`)** into assignable macro buttons. It is a **Tauri 2 + React + TypeScript + Vite + Tailwind v4** desktop app that *edits a JSON config*; a separate **AutoHotkey (AHK v2) engine** does the actual execution when physical keys are pressed.

Two components work together:
1. **The app (configuration editor)** — user clicks grid buttons to assign actions, saves to `config.json`.
2. **The AHK engine** — reads `config.json`, registers numpad hotkeys, runs the configured actions.

> The app is an **editor, not an executor**. Clicking a button in the UI selects it for editing; it does NOT run the macro.

---

## Project Root

```
E:\ayman\Development\My progicte\ME\macro app\ayman-macro-deck\
```

## Key Externals (not in repo)

| Path | Purpose |
|------|---------|
| `E:\ayman\ME\Macro\script\steam_mode.ahk` | The AHK engine (the real executor) |
| `C:\Program Files\AutoHotkey\v2\AutoHotkey.exe` | AHK v2 interpreter (system-wide) |
| `E:\ayman\ME\Macro\SoundVolumeView\SoundVolumeView.exe` | Audio device switcher |
| `E:\ayman\ME\Macro\script\SteamProfiles\*.lnk` | Steam account switch shortcuts (SOMO, Tesk, spydey, Khazan...) |
| `E:\ayman\ME\Macro\script\Games\*.lnk` | Game shortcuts |
| `%APPDATA%\com.ayman.macrodeck\config.json` | **Live config** — single source of truth for what keys do |

---

## Architecture / Data Flow

```
                ┌────────────────────────────┐
                │         THE APP (React)    │
                │  ButtonGrid  →  ButtonEditor│
                │  store (Zustand)           │
                └────────────┬───────────────┘
                             │ save_config / load_config (Tauri command)
                             ▼
                %APPDATA%\com.ayman.macrodeck\config.json
                             ▲
                             │ reads every 1 second (RefreshDeck timer)
                ┌────────────┴───────────────┐
                │      THE ENGINE (AHK v2)   │
                │  Numpad hotkeys → find     │
                │  button by key in ACTIVE   │
                │  profile → RunAction       │
                └────────────────────────────┘
```

- The engine reloads `config.json` every 1 second (`SetTimer RefreshDeck, 1000`), so app edits take effect automatically — no restart needed.
- When a button is saved in the app, it also fires the Rust command `reload_ahk_script`, which restarts the engine (via `#SingleInstance Force`).
- The app has a **500ms auto-save debounce** (`App.tsx`) — any config state change is written to disk soon after.

---

## The Two "Profiles" and Mode Selection (CRITICAL)

The engine picks which profile to use based on **ScrollLock state** (the "mode"), NOT only the app's `activeProfileId`:

- **ScrollLock ON** → **Gaming** profile → Gaming buttons (Steam account switching, games)
- **ScrollLock OFF** → **Normal** profile → Normal buttons (apps, URLs, audio)

This is `GetActiveProfile()` in the engine. **The active profile from the app is only a fallback** if profiles named exactly "Gaming"/"Normal" don't exist.

> **Common bug:** editing button 9 in the app while the *Gaming* profile is active changes the Gaming profile, but pressing numpad 9 in Normal mode uses the *Normal* profile's button 9. Always check which profile is active **in the UI (bottom-left selector) AND which mode ScrollLock is in** when debugging "why doesn't my button do what I set".

`switch_profile` actions toggle ScrollLock (and thus profile) at runtime:
- `Gaming` → `SetScrollLockState "On"`, beep 1200
- `Normal` (or any other) → `SetScrollLockState "Off"`, beep 800/1000

---

## Numpad Key Mapping (engine ↔ app grid)

| Physical key | Button `key` | App grid label |
|------|------|------|
| Numpad1 / NumpadEnd | 1 | 1 |
| Numpad2 / NumpadDown | 2 | 2 |
| Numpad3 / NumpadPgDn | 3 | 3 |
| Numpad4 / NumpadLeft | 4 | 4 |
| Numpad5 / NumpadClear | 5 | 5 |
| Numpad6 / NumpadRight | 6 | 6 |
| Numpad7 / NumpadHome | 7 | 7 |
| Numpad8 / NumpadUp | 8 | 8 |
| Numpad9 / NumpadPgUp | 9 | 9 |
| NumpadDiv (`/`) | 10 | `/` |
| NumpadMult (`*`) | 11 | `*` |

- Both NumLock states are covered (digit names + navigation names map to the same button).
- The engine uses AHK **hotkeys**, which **consume** the keypress (nothing is typed into the focused app).
- Remote trigger mode: launching the engine with a numeric arg runs that button and exits (`steam_mode.ahk 3`).

---

## Config Shape (`config.json`)

```json
{
  "version": 1,
  "activeProfileId": "fd4b4c8a",
  "profiles": [
    {
      "id": "...",
      "name": "Gaming",
      "buttons": [
        {
          "id": "...",
          "key": 1,
          "name": "Steam SOMO",
          "icon": "🎮",
          "description": "...",
          "enabled": true,
          "actions": [
            { "id": "...", "type": "switch_steam_account", "name": "Switch Steam Account", "params": { "account": "SOMO" }, "delayAfter": 0 }
          ]
        }
      ],
      "createdAt": 0,
      "updatedAt": 0
    }
  ],
  "settings": { "general": {}, "keyboard": {}, "steam": {}, "audio": {}, "appearance": {} }
}
```

- IDs are 8-char hex (first 8 chars of `crypto.randomUUID()`), generated in `src/store/index.ts`.
- Each profile has exactly **11 buttons** (keys 1-11). Keys 10/11 default to `switch_profile` (Gaming/Normal).
- `params` values are strings mostly; `delayAfter` is a number (ms sleep after the action in the engine).

---

## Action Types (registry: `src/lib/actionRegistry.ts`)

Handled by the **engine** (`RunAction` in `steam_mode.ahk`), mirrored in Rust `execute_action`:

| type | param | Notes |
|------|-------|-------|
| `open_application` | `path` | Open exe/shortcut |
| `open_url` | `url` | Default browser |
| `switch_steam_account` | `account`, optional `waitUntilOpen` | Uses `settings.steam.profileShortcuts`; when `waitUntilOpen=true` the engine polls `loginusers.vdf` (via `WaitForSteamAccount`/`SteamAccountActive`) until Steam is running with that account marked most-recent before continuing |
| `launch_steam_game` | `appId`, `name` | `steam://rungameid/<appId>` |
| `launch_epic_game` | `uri` | Epic launcher URI |
| `change_audio_device` | `device` | Via SoundVolumeView |
| `run_command` | `command`, `args` | |
| `run_ahk_script` | `scriptPath` | Uses `settings.keyboard.ahkExecutablePath` |
| `open_folder` | `path` | |
| `toggle_gaming_mode` | — | Toggles ScrollLock + beep/tooltip |
| `switch_profile` | `profile` | Name of target profile (Gaming/Normal) |
| `rdr2_cycle` | — | Backup saves → uninstall Rockstar Launcher → silent fresh install (engine-only; uses `settings.rdr2` paths; uninstaller runs with `RunWait` before the silent install) |
| `keyboard_shortcut` | `keys` | SendInput (implemented in Rust; engine TODO) |
| `volume_up` / `volume_down` / `mute` | — | System media keys |

**IMPORTANT engine quirk:** after running a button while in Gaming mode, unless the button is a mode-toggle (`toggle_gaming_mode` or `switch_profile`), the engine auto-returns to Normal mode (`RunButton` one-shot logic). This is intentional "one-shot gaming".

---

## Source Layout (app)

| File | Purpose |
|------|---------|
| `src/App.tsx` | Layout: Header + (ButtonGrid \| ButtonEditor) + StatusBar; config load/save, mode poll every 2s |
| `src/components/ButtonGrid.tsx` | Numpad-style layout: keys 1-9 (3×3) on left, `/` and `*` in a side column |
| `src/components/MacroButton.tsx` | Grid card (click = select for editing) |
| `src/components/ButtonEditor.tsx` | Right panel: key, icon, name, desc, enable toggle, action list (add/remove/reorder/delay) |
| `src/components/Header.tsx` | Title + profile manage gear |
| `src/components/ProfileManager.tsx` | Slide-out profile CRUD |
| `src/components/ProfileSelector.tsx` | Bottom-left active-profile dropdown |
| `src/components/Rdr2Panel.tsx` | Slide-out RDR2 Cycle panel (🐎 button in Header): edits `settings.rdr2` saves/backup/installer paths + confirm-before-uninstall toggle |
| `src/components/SteamAccountsPanel.tsx` | Slide-out Steam Accounts panel (🎮 button in Header): edits `settings.steam.profileShortcuts` (add/rename/remove accounts + pick `.lnk`), scans `settings.steam.profileShortcutsFolder` via Rust `list_lnk_files` |
| `src/components/StatusBar.tsx` | Profile selector + Gaming/Normal + keyboard status dots |
| `src/components/ConfirmationDialog.tsx` | Confirm dialog (profile delete, etc.) |
| `src/components/ErrorBoundary.tsx` | Render error catch |
| `src/lib/actionRegistry.ts` | Action definitions + `createAction` factory + dynamic option loaders |
| `src/store/index.ts` | Zustand store: default profiles (11 buttons), CRUD, `normalizeProfile` strips legacy `oldKeyboardButtons`, auto-save |
| `src/types/index.ts` | TS types (AppConfig, Profile, MacroButton, ButtonAction, ...) |
| `src/index.css` | Tailwind v4 `@theme` dark palette + animations |
| `src-tauri/src/lib.rs` | Rust commands: config load/save, reload_ahk_script, execute_action, get_gaming_mode, get_keyboard_connected, list_audio_devices, list_lnk_files, tray |
| `src-tauri/src/main.rs` | Entry point (calls `lib::run`) |
| `src-tauri/capabilities/default.json` | Tauri permission capabilities |
| `src-tauri/tauri.conf.json` | Window 900×700, id `com.ayman.macrodeck`, bundles msi+nsis |

---

## Engine Structure (`E:\ayman\ME\Macro\script\steam_mode.ahk`, 516 lines)

- `#SingleInstance Force` — relaunching replaces the old instance.
- `LoadConfig()` parses `config.json` with a hand-written `JSON` class (parser handles `\uXXXX` escapes).
- `GetActiveProfile()` — mode-based selection (see above).
- `RunAction(action)` — the big `switch` executing each type; respects `delayAfter`.
- `RunButton(button)` → runs actions + one-shot auto-return-to-Normal.
- `RunButtonByKey(keyNum)` — the physical-key entry point.
- Hotkey blocks at file end: `Numpad1::`…`Numpad9::`, `NumpadDiv::` (10), `NumpadMult::` (11).
- Remote trigger: `if A_Args.Length > 0 { LoadConfig(); RunButtonByKey(Integer(A_Args[1])); ExitApp }`.

> **Formatting note:** every stacked hotkey group must be its own label lines followed by a `{ ...; return }` block. Single-line stacking (`NumpadEnd::Numpad1::`) is INVALID in AHK v2; multi-line stacking without braces is INVALID ("missing its opening brace").

---

## Build / Run Commands

| Task | Command |
|------|---------|
| Frontend typecheck + build | `pnpm build` (→ `tsc && vite build`) |
| Dev run | `pnpm tauri dev` |
| **Production build** | `pnpm tauri build` → exe + `bundle\msi\*.msi` + `bundle\nsis\*-setup.exe` |
| Tauri CLI version | `pnpm tauri --version` (tauri-cli 2.11.4), also `npx tauri` |
| Kill running app | `Stop-Process -Name "ayman-macro-deck" -Force` |

Production outputs (`src-tauri\target\release\`):
- `ayman-macro-deck.exe` (direct run)
- `bundle\msi\AYMAN MACRO DECK_0.1.0_x64_en-US.msi`
- `bundle\nsis\AYMAN MACRO DECK_0.1.0_x64-setup.exe`

### Verifying the AHK engine boots clean
```powershell
Remove-Item E:\ayman\ME\Macro\script\b_out.txt,E:\ayman\ME\Macro\script\b_err.txt -ErrorAction SilentlyContinue
$p = Start-Process "C:\Program Files\AutoHotkey\v2\AutoHotkey.exe" -ArgumentList @('/ErrorStdOut','"E:\ayman\ME\Macro\script\steam_mode.ahk"') -RedirectStandardOutput "E:\ayman\ME\Macro\script\b_out.txt" -RedirectStandardError "E:\ayman\ME\Macro\script\b_err.txt" -PassThru
Start-Sleep 3; "$(-not $p.HasExited)"; Get-Content E:\ayman\ME\Macro\script\b_err.txt -Raw
```
`alive=True` + empty stderr = clean boot. Test remote-trigger: `...\AutoHotkey.exe "...\steam_mode.ahk" 3` should exit 0.

---

## Environment Facts

- **OS:** Windows (dev shell is PowerShell 5.1, tool runs `cmd1; if ($?) { cmd2 }` — **no `&&`**).
- **Package manager:** pnpm (workspace root uses pnpm; `pnpm dev`, `pnpm build`, `pnpm tauri ...`).
- **Tailwind CSS v4** uses the `@theme` directive in CSS (no `tailwind.config.js`).
- **MSVC Build Tools** required for Rust compilation (installed at `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools`).
- **Rust:** 1.98.0. Tauri 2.11.x.

---

## Common Pitfalls / Gotchas

1. **App overwrites config edits:** while the app is running, it owns `config.json` (500ms auto-save). Directly editing the file gets overwritten — change buttons from the app UI, or edit the file only when the app is closed.
2. **Profile vs mode mismatch:** engine uses ScrollLock-based mode for profile selection — the button you edited in the app may be the wrong profile for the mode you test in.
3. **`oldKeyboardButtons` is dead:** the old "Old Keyboard identification" feature was fully removed (app + engine). `normalizeProfile` in the store deletes any leftover `oldKeyboardButtons` key. Do NOT re-add old-keyboard/device-gating logic; the deck keys work from ANY keyboard.
4. **Don't reintroduce raw input:** numpad keys must stay AHK hotkeys (they consume the key so it isn't typed into games/apps). Raw input only listened and leaked the digit through.
5. **AHK v2:** no legacy key syntax, `%` interpolation changes, sequential hotkey syntax rules (see engine note above).

---

## Recommended First Steps for a New Chat

1. Read this file (done) + skim `src/store/index.ts`, `src-tauri/src/lib.rs`, `E:\ayman\ME\Macro\script\steam_mode.ahk`.
2. Check the **live config** to see current button mappings:
   ```powershell
   Get-Content "$env:APPDATA\com.ayman.macrodeck\config.json" -Raw | ConvertFrom-Json | ConvertTo-Json -Depth 10
   ```
3. Verify the engine is running / boots clean before assuming code issues.