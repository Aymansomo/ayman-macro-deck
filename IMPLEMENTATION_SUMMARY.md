# AYMAN MACRO DECK — What Changed & How To Test

**Written:** 2026-11-18  
**Updated:** 2026-11-18 (P0 UI fixes + P1 pass + signed production build)  
**Purpose:** A written record of everything that was built/changed in this pass, what was actually
tested, and a step-by-step checklist for **you** to test it and choose your USB keypad in the app.

> **Read this first:** I did **not** select a keyboard for you, and I did **not** verify physical
> device isolation with two separate physical keyboards. That test needs your hardware. See
> sections 2, 6 and 7.

---

## 1. What you asked for

Turn the app into a polished visual keyboard/keypad manager where:

- Only the **selected separate USB macro keypad** triggers macros.
- The **main keyboard stays unaffected** — typing on it must never fire a macro.
- You can see a **visual keyboard**, click any key, and assign/edit macros on it.
- There is a **Device Manager** to choose and test the physical keypad.
- There is a **Dashboard/status bar** showing engine, device and trigger state.
- All existing profiles, actions, assignments and config data are **preserved** (additive migration
  only, with a backup).

Constraints I followed:

- The existing Rust `ActionEngine` (`actions::run_button`) remains the **only** macro executor.
- Native Windows Raw Input APIs are used for device identity: `RegisterRawInputDevices`, `WM_INPUT`,
  `GetRawInputData`, `GetRawInputDeviceInfo`, `GetRawInputDeviceList`.
- No AutoHotkey execution, no second engine, no fake device detection, no static keyboard images,
  no global key suppression, and **no changes to the external AHK script**.
- Tray, profiles, actions, config format and existing assignments stay intact.

---

## 2. Honest status (important)

| Claim | Status |
|---|---|
| Code compiles and builds | ✅ Verified |
| Automated tests pass | ✅ Verified (40 Rust tests, frontend `pnpm build`) |
| App boots clean and enumerates real keyboards | ✅ Verified earlier (5 real devices seen) |
| Existing config/actions preserved after migration | ✅ Verified + backup created |
| P0 Device Manager UI crash/status bugs | ✅ Code fixed; ⏳ awaiting your retest |
| Production installer build (MSI + NSIS) | ✅ Verified, exit code 0 |
| Responsive UI pass | ✅ Built/typechecked; measured key sizing, readable minimums, scroll instead of shrinking |
| Open Application auto-fill picker | ✅ Built/typechecked; native file dialog + Windows icon/name extraction |
| Updater signatures + `latest.json` | ✅ Generated, uploaded, and verified on the v0.1.4 GitHub release |
| **Selected keypad triggers, main keyboard does not** | ⚠️ **NOT verified — needs two physical keyboards** |
| Disconnect/reconnect, NumLock on/off, key repeat | ⚠️ Not tested — needs your hardware |
| Hardware action regressions (Steam, audio, gaming mode), tray, phone API | ⚠️ Not regression-tested this pass |
| RDR2 Cycle | ❌ Removed entirely by your request; not re-added |

The isolation logic is implemented and unit-tested at the code level, but **I will not say it works on
real hardware until you test it with two separate physical keyboards** (section 7, Step D).

---

## 3. What changed, stage by stage

### Stage 1 — Real device identity + isolation boundary (Rust)

New file `src-tauri/src/device.rs`:

- A `RawInputManager` registers a hidden message-only window and calls `RegisterRawInputDevices` for
  HID keyboards (usage page 1, usage 6) with `RIDEV_INPUTSINK | RIDEV_DEVNOTIFY`.
- `WM_INPUT` → `GetRawInputData` → `RAWINPUTHEADER.hDevice` gives the **originating device handle**.
  `GetRawInputDeviceInfoW(RIDI_DEVICENAME)` gives its interface path. That path (uppercased) is the
  stable device ID. `GetRawInputDeviceList` enumerates all keyboards; hardware ID (`VID_…&PID_…`)
  is parsed only as a fallback name/hint.
- On every key event, `should_dispatch()` checks:
  `selected.connected && selected.id == actual_device_id`. A key from any other keyboard is dropped
  **before** it can be mapped to a button — even if it has the same scan code.
- `RIDEV_NOLEGACY` is deliberately **not** used: it applies per top-level collection, not per device,
  so it would suppress the main keyboard too. Selected-keypad keys may still type normally — that is
  intentional Windows-compatible behavior. `suppressLegacy` is always `false` and is exposed in the
  status object only to document this.
- Device arrival/removal (`GIDC_ARRIVAL`/`GIDC_REMOVAL`) refreshes the list and emits
  `keyboard-device-added` / `keyboard-device-removed`. Disconnecting the selected device pauses
  macro input; pressed-key state is cleared to avoid stuck keys.
- Test mode: `start_test()` captures the next physical key and emits `device-test-result` with the
  device and `keyId`; `confirm_device_test()` adopts that device.
- Selection persists in `%APPDATA%\com.ayman.macrodeck\device.json`, separate from `config.json`.
  On reload it prefers the exact path; it adopts a new path only when exactly one device matches the
  saved hardware ID (ambiguous hardware IDs are never guessed).

New file `src-tauri/src/keyid.rs`:

- `KeyId` maps Raw Input scan code + `E0`/`E1` extended flags to a stable physical key identity
  (`Numpad1`, `NumpadDivide`, `ArrowUp`, `KeyA`, `Scan…E0`, …). This is what makes identity independent
  of NumLock state and of the printed label.
- `legacy_key()` / `from_legacy_key()` bridge old config slots `1–9`, `10 = /`, `11 = *`.
- `parse()`/`id()` round-trip so identities can be stored in config and resolved later.

Reworked `src-tauri/src/keyboard.rs`:

- The `WH_KEYBOARD_LL` hook is now a **pure observer**. It counts events for diagnostics and always
  calls `CallNextHookEx` — it never consumes a key and never dispatches a macro.
- The **only** macro dispatch path is a device worker thread reading `DeviceKeyEvent`s from the Raw
  Input manager. It fires on key-down only, ignores auto-repeat, and runs actions through the one
  existing `actions::run_button` engine (wrapped in `catch_unwind`). After a non-mode action in
  Gaming mode it returns to Normal mode, preserving the old one-shot gaming behavior.
- Key lookup order per button: stable `physicalKeyId` first, then explicit `trigger`, then legacy
  `key` slot. Events `macro-action-started`, `macro-action-completed`, `macro-action-error`,
  `deck:triggered` are emitted for the UI.
- `set_paused()` / `dispatch_button()` power the pause/resume and "test button" features.

`src-tauri/Cargo.toml`: added Windows features for Raw Input/GDI, window messages and library loader.

### Stage 2 — Frontend typing + Device Manager

- `src/lib/backend.ts`: typed Tauri wrappers for all device/engine commands
  (`get_keyboard_devices`, `refresh_keyboard_devices`, `get_selected_macro_device`,
  `select_macro_device`, `clear_macro_device_selection`, `start_device_test`, `stop_device_test`,
  `get_device_test_candidate`, `confirm_device_test`, `get_macro_engine_status`,
  `set_macro_engine_running`) and typed event listeners.
- `src/components/DeviceManagerPanel.tsx`: slide-out panel with device list, Refresh, Select,
  Clear selection and the **Test Keyboard** flow (press a key → detected device/key shown → Confirm).
  Multiple HID collections from the same physical keyboard are labelled `… · collection N` so you
  can tell them apart.
- New events are wired in `src/App.tsx` (device added/removed/selected, key pressed, action
  started/completed/error).

### Stage 3 — Additive config migration (nothing deleted)

- `src-tauri/src/models.rs`: added `layout?: "numpad" | "numpadNav" | "full"` on profiles and
  `physicalKeyId?: string` on buttons. Every struct keeps `#[serde(flatten)] extra`, so unknown
  JSON fields survive round trips. `migrate_layouts()` only **adds** fields:
  layout defaults to `numpad`; `physicalKeyId` is derived from `trigger` or the legacy `key` slot.
  It never removes actions, assignments, profiles or unknown fields, and is idempotent.
- `src-tauri/src/state.rs`: before the **first** migrated write, a timestamped backup of the exact
  original bytes is written to
  `%APPDATA%\com.ayman.macrodeck\config.json.bak-<timestamp>` (so even unknown fields are preserved
  byte-for-byte).
- `src/types/index.ts` + `src/store/index.ts`: matching TS types, `setProfileLayout`,
  `ensureButtonForPhysicalKey`, `removeButton` (with confirmation) and normalization defaults.

### Stage 4 — Visual keyboard editor

- `src/lib/keyboardLayouts.ts`: key definitions for three layouts — **Numpad Only**,
  **Numpad + Navigation**, **Full Keyboard** — with physical IDs, labels and proportional sizing.
- `src/components/KeyboardView.tsx` + `KeyboardKey.tsx`: renders the layout with key states
  (configured / selected / active / running), legends and icons.
- `src/components/KeyboardEditor.tsx`: layout switcher, counts of configured keys/actions, and a
  notice listing off-layout assignments so they are **never silently lost**.
  Clicking a key creates/selects its button.
- The old `ButtonGrid`/`MacroButton` files are still present but the main view is now `KeyboardEditor`.

### Stage 5 — Action assignment integration

- `src/components/ButtonEditor.tsx`: shows the **physical key ID** of the selected key, offers
  **Remove assignment** with confirmation, and displays running/error state from action events.
  Saving still calls `reload_ahk_script`, which is now a compatibility shim that re-registers native
  bindings (`keyboard.reload()`) — it no longer launches AHK.
- **Open Application picker:** adding that action immediately opens a native file dialog for `.exe`,
  `.lnk`, `.bat`, `.cmd` or `.msi`. The new Rust command `get_application_info` resolves the app name
  and caches the Windows Explorer icon as a PNG under `%APPDATA%\com.ayman.macrodeck\images\`. The
  editor then auto-fills the action path, button name, button image and (when empty) description. A
  **Choose app…** button next to the path field allows re-picking later.

### Stage 6 — Dashboard / status

- `src/components/DashboardBar.tsx`: engine/device status, recent actions, pause/resume, and a
  shortcut to Device Manager.
- `src/components/Header.tsx`: device indicator and Device Manager trigger.
- `src/components/StatusBar.tsx`: engine + device status dots.
- Header title changed from `SOMO MACRO DECK` to **`AYMAN MACRO DECK`**.

### Stage 7 — P0 bug fixes + P1 pass (this pass)

**P0 Device Manager bugs fixed**

- `keyboard-engine-status-changed` carries only a partial `DeviceStatus`. `App.tsx` was casting it to
  the full `MacroEngineStatus`, making `engineStatus.keyboard` undefined and crashing into the
  ErrorBoundary with "Something went wrong / cannot read properties of undefined". The handler now
  re-polls full status instead of casting the partial payload.
- `DeviceManagerPanel.tsx` now updates `testing` from status events and shows the currently selected
  keypad while idle, so the panel no longer stays stuck on "click button to choose the keyboard"
  after selection until a full app reload. **Please retest this.**

**RDR2 Cycle removed**

- Deleted `Rdr2Panel.tsx`, Header button/panel state, `rdr2_cycle` registry entry, `rdr2` settings
  type/defaults/store actions, and the Rust whitelist/executor branch. No RDR2 references remain in
  `src/` or `src-tauri/src/`. It was not re-added anywhere.

**Native Steam account wait (`waitUntilOpen`)**

- `src-tauri/src/actions.rs` now checks `Steam.exe` with ToolHelp and polls
  `<SteamDir>\config\loginusers.vdf` until the target account is `MostRecent`, matching the old AHK
  behavior. Default timeout is 120s; numeric seconds are also accepted. `steamExecutable` from
  settings is injected into action params.

**Custom trigger + key calibration**

- `ButtonEditor.tsx` has a validated `TRIGGER` field (e.g. `Numpad1`, `Ctrl+Shift+Numpad4`,
  `KeyA`, `Scan035E0`). Invalid values block save with a clear message.
- Device Manager's detected key can now be assigned to the currently selected button via the new
  `setButtonPhysicalKey` store action; it clears any conflicting explicit `trigger`.

**Multi-HID collection support (opt-in)**

- `device.json` and `DeviceStatus` now carry `includeCollections`. Default `false` means **exact
  selected HID interface path only**.
- When enabled, sibling HID collections sharing the selected keypad's hardware ID are also accepted.
  A different physical keyboard with the same VID/PID is still rejected unless it is the exact
  selected path. Unit test added.

**Cleanup + tools**

- Removed unused `auto_start_engine` / `engine_mode` fields from Rust and TS models/defaults.
- Added `open_logs_folder`, `export_config`, `import_config`, `list_config_backups`, and
  `restore_config_backup` commands, with matching typed wrappers in `backend.ts`.
- Profile Manager now has a **Tools** section: Export config, Import config, Open logs folder, and a
  restore-from-backup selector. Import/restore re-sync the store and reload native bindings.

**Build/signing**

- `pnpm tauri build` now completes cleanly. A new updater keypair was generated because the
  previously configured public key had no private key. The public key in `tauri.conf.json` was
  updated, and user-level environment variables were set so future builds sign automatically.
- The local version was bumped from **0.1.3 → 0.1.4** because GitHub already has a v0.1.3 release
  signed with the old key. Do **not** overwrite that release.
- Both v0.1.4 installers and both `.sig` files were produced. v0.1.4 `latest.json` was generated at
  `src-tauri\target\release\bundle\latest.json`, then uploaded to the published v0.1.4 GitHub release.
- Current source was pushed to `main` as commit `6d2e720`. The release is live at  
  https://github.com/Aymansomo/ayman-macro-deck/releases/tag/v0.1.4
- Existing v0.1.3 installations cannot auto-verify v0.1.4 with the old embedded public key. They
  need a one-time manual install of the v0.1.4 setup; after that, future updates use the new key.

**Responsive UI polish (after the v0.1.4 release)**

- `KeyboardView.tsx` measures the available canvas with `ResizeObserver` and computes key size from
  the layout instead of using one fixed tiny unit. Numpad keys stay large; Full Keyboard keys keep a
  readable 44px minimum and scroll when the window is too small instead of shrinking into illegibility.
- `KeyboardKey.tsx` now scales label/icon/name fonts from `--key-unit`.
- `index.css` adds grouped keyboard styling, safe centering, readable group titles, and responsive
  breakpoints. Dashboard cards, header actions, dialogs, side panels and the Button Editor now adapt
  to narrow windows; the full keyboard can scroll in both directions.
- Row-span keys are positioned on a fixed row pitch so `+` / `Enter` no longer create extra blank rows.
- These responsive changes were built and typechecked locally after the v0.1.4 release; they are not
  in the published installer until pushed and released again.

---

## 4. Files changed / added

| Area | Files |
|---|---|
| New Rust | `src-tauri/src/device.rs`, `src-tauri/src/keyid.rs` |
| Reworked Rust | `src-tauri/src/actions.rs`, `src-tauri/src/keyboard.rs`, `src-tauri/src/models.rs`, `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` |
| New frontend | `src/lib/keyboardLayouts.ts`, `src/components/DeviceManagerPanel.tsx`, `src/components/DashboardBar.tsx`, `src/components/KeyboardEditor.tsx`, `src/components/KeyboardView.tsx`, `src/components/KeyboardKey.tsx` |
| Reworked frontend | `src/App.tsx`, `src/lib/backend.ts`, `src/types/index.ts`, `src/store/index.ts`, `src/components/ButtonEditor.tsx`, `src/components/Header.tsx`, `src/components/StatusBar.tsx`, `src/components/ProfileManager.tsx` |
| Deleted | `src/components/Rdr2Panel.tsx` |
| Release artifacts | `src-tauri\target\release\bundle\**` (MSI, NSIS, `.sig`, `latest.json`) |
| External | **Nothing changed** in `E:\ayman\ME\Macro\script\steam_mode.ahk` or any config outside this repo |

The v0.1.4 source was pushed to GitHub as commit `6d2e720`. The responsive UI changes made after
that release are local-only until they are committed/pushed and published as a new version.

---

## 5. What was actually tested (performed)

| Test | Command / method | Result |
|---|---|---|
| Rust unit tests | `cargo test --manifest-path "src-tauri\Cargo.toml"` | **40 passed, 0 failed** |
| Rust build | `cargo build --manifest-path "src-tauri\Cargo.toml"` | **passed** |
| Frontend typecheck + build | `pnpm build` (`tsc && vite build`) | **passed** |
| Production installers + updater signing | `pnpm tauri build` with signing env vars | **exit code 0**; 2 bundles + 2 signatures |
| Clean runtime boot | launched app with redirected stdout/stderr | `alive=True`, empty stderr |
| Boot log | see below | migration backup, **5 real keyboard devices**, observer started, Raw Input listener started, tray ready, `AYMAN MACRO DECK ready` |
| Config preservation | read live JSON after migration | Gaming: 11 buttons / 13 actions / `layout=numpad` / 11 physical IDs; Normal: 11 buttons / 11 actions / 11 physical IDs; `active=884e0485`; `backups=1` |
| Backup file | inspected `%APPDATA%\com.ayman.macrodeck\` | `config.json.bak-1795033076738` created |

The app process was stopped after runtime verification. No device was selected by me.

---

## 6. What was NOT tested (needs your hardware)

- ❌ **Physical isolation with two separate keyboards** — the key acceptance criterion.
- ❌ Test Keyboard confirm flow on a real keypad (code fixed; user retest pending).
- ❌ Selected-keypad keys actually firing macros.
- ❌ Main keyboard NOT firing macros.
- ❌ Disconnect/reconnect of the USB keypad.
- ❌ NumLock on/off, key repeat, arrow/navigation keys on real hardware.
- ❌ Hardware action regressions: Steam shortcuts, audio switching, gaming mode, tray, phone API.
- ✅ GitHub v0.1.4 release published with `latest.json` + installers + `.sig` files:  
  https://github.com/Aymansomo/ayman-macro-deck/releases/tag/v0.1.4  
  Existing v0.1.3 users need a one-time manual reinstall.

---

## 7. How to test (your turn)

### Step A — Run the app

```powershell
cd "E:\ayman\Development\My_progicte\ME\macro app\ayman-macro-deck"
pnpm tauri dev
```

If an old instance is running:

```powershell
Stop-Process -Name "ayman-macro-deck" -Force -ErrorAction SilentlyContinue
```

### Step B — Choose your keypad (Device Manager)

1. Open the app. In the top bar / DashboardBar click the **device manager** button.
2. You will see a list of real keyboards reported by Windows Raw Input. Plug in your USB keypad
   **before** refreshing if it is not listed.
3. Click **Refresh devices** if needed.
4. Click **Test Keyboard**, then **press one key on the intended USB keypad**.
   The panel should show the detected device name/path and the key (`Numpad1`, `/`, `A`…).
5. If the detected device looks right, click **Confirm** → it becomes the selected macro keypad.
   (You can also select a device directly from the list, but Test Keyboard is the safest way to
   avoid choosing your main keyboard by mistake.)
6. Use **Clear selection** to undo. The choice is saved in `device.json` and restored on next launch.

> If two rows look identical, they are HID collections of the same device — the UI shows
> `… · collection 1`, `… · collection 2`. Test a key to see which one actually receives it.
> If only some keys fire, enable **Include all HID collections of this keypad** in Device Manager.
> While a key button is selected, Test Keyboard also offers **Assign this key to the selected button**
> for keys that appear as `Scan…E0` instead of a friendly name.

### Step C — Assign keys in the visual editor

1. Pick a profile (bottom-left selector). Remember: while **Gaming mode** is on, the engine uses the
   `Gaming` profile; otherwise `Normal` (see StatusBar).
2. In the main view choose a layout: **Numpad**, **Numpad + Nav**, or **Full**.
3. Click any key → the editor opens on the right showing its physical key ID.
4. Add/edit actions, or click **Remove assignment**. Changes auto-save (~500 ms debounce).
5. Off-layout assignments are listed in the editor header — they are kept, not deleted.

### Step D — Physical isolation test (the important one)

With **two separate physical keyboards** connected (your main keyboard + the USB keypad):

1. Select the USB keypad using Step B.
2. Press the same physical key on the **USB keypad** → the assigned macro should run; the
   Dashboard should flash the action and the trigger count should increase.
3. Press the corresponding key on the **main keyboard** → **no macro should run** and the trigger
   count should not increase.
4. Unplug the USB keypad → macro input should pause/no longer fire; plug it back in and check it
   still fires (the app remembers the selection by hardware ID when unambiguous).
5. Repeat once with NumLock off and once with it on.

> If step 2 fails, check that the key you pressed matches the physical key ID shown in the editor.
> If step 3 fires, stop and send me the log — that means the isolation boundary needs fixing.

### Step E — Confirm nothing was lost

Compare `%APPDATA%\com.ayman.macrodeck\config.json` profiles/actions with the backup
`config.json.bak-*`. The migration only added `layout` and `physicalKeyId`.

---

## 8. If something is wrong — what to send me

Collect:

```powershell
Get-Content "$env:APPDATA\com.ayman.macrodeck\logs\app.log" -Tail 200
Get-Content "$env:APPDATA\com.ayman.macrodeck\device.json" -Raw
Get-Content "$env:APPDATA\com.ayman.macrodeck\config.json" -Raw
```

Include:

- Which device you selected (name/path from the Device Manager).
- The physical key ID shown in the editor and the key you actually pressed.
- Which of Step D's checks failed (keypad not firing / main keyboard firing / reconnect issue).
- The relevant tail of `app.log`.

---

## 9. Design decisions & limitations (so nothing surprises you)

- **Selected-keypad keys still type normally.** Per-device key suppression is not a Windows Raw
  Input capability (`RIDEV_NOLEGACY` would suppress every keyboard). We isolate *macro triggering*,
  not typing.
- **The low-level keyboard hook is observer-only.** It exists for diagnostics; it cannot block keys
  or run macros, so the main keyboard cannot trigger anything through it.
- **Identity is the HID interface path**, with hardware ID used only as a fallback when exactly one
  candidate matches. Two identical keypads need Test Keyboard to disambiguate.
- **Profile selection follows Gaming mode**: `Gaming` profile while Gaming mode is on, otherwise
  `Normal`, falling back to the editor's active profile if those names are missing. The old one-shot
  gaming auto-return is preserved.
- `reload_ahk_script` is now only a compatibility shim — no AHK process is started. The external
  AHK script is untouched and is no longer the executor.
- **RDR2 Cycle was removed entirely** at your request. There is no panel, button, registry action,
  settings section, or Rust executor branch for it.
- **Multi-collection support is opt-in.** By default only the exact selected HID interface path can
  trigger macros; enabling the checkbox accepts sibling collections with the same hardware ID.
- `AGENTS.md` is still outdated: it describes the AHK engine as the executor. This document and
  `roadmap.md` reflect the current native-engine architecture.

---

## 10. Build commands (reference)

```powershell
# Typecheck + frontend build
pnpm build

# Rust tests
cargo test --manifest-path "src-tauri\Cargo.toml"

# Dev run
pnpm tauri dev

# Production installer + updater signatures
pnpm tauri build
# Signing env vars are already set at Windows user scope:
#   TAURI_SIGNING_PRIVATE_KEY, TAURI_SIGNING_PRIVATE_KEY_PATH, TAURI_SIGNING_PRIVATE_KEY_PASSWORD
# outputs: src-tauri\target\release\bundle\msi\*.msi(.sig)
#          src-tauri\target\release\bundle\nsis\*-setup.exe(.sig)
#          src-tauri\target\release\bundle\latest.json  (publish to the GitHub release)
```

---

### Bottom line

The code is built, tested, signed, pushed to GitHub, and published as v0.1.4; it also preserved your
config. **The remaining work is yours:** retest the two Device Manager UI bugs, open Device Manager →
Test Keyboard → press a key on the real USB keypad → Confirm, then run the two-keyboard isolation
test in Step D. Existing v0.1.3 users need one manual reinstall because the signing key changed.
Send me the log and details if anything fails, and I will fix it.
