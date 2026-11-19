# AYMAN MACRO DECK — Roadmap

**Updated:** 2026-11-18
**Related:** `IMPLEMENTATION_SUMMARY.md` (what changed / what was tested)

---

## P0 — Device Manager bugs (fixed, user retest pending)

1. ✅ **Test Keyboard click → "something went wrong: cannot read properties of undefined"**  
   **Root cause:** `keyboard-engine-status-changed` emits a partial `DeviceStatus`; `App.tsx` cast it to full `MacroEngineStatus`, so `engineStatus.keyboard` became `undefined` and the ErrorBoundary crashed.  
   **Fix:** the handler now re-polls full engine status instead of casting the partial payload.
2. ✅ **After Confirm/Select the panel still says "click button to choose the keyboard" until app reload**  
   **Fix:** Device Manager now updates `testing` and the selected device from status events and shows the current keypad while idle.
3. ⏳ **User retest required** — Test Keyboard → press a key on the intended USB keypad → Confirm → reload the app and verify the selection stays visible without a second reload.

---

## P0 — Hardware verification (user, still pending)

1. ⏳ **Choose the keypad in the app** — Device Manager → Test Keyboard → press a key → Confirm.
2. ⏳ **Two-keyboard isolation test** — keypad fires the macro; main keyboard fires nothing.
3. ⏳ **Disconnect/reconnect test** — unplug → input pauses; plug back → works (restored by hardware ID when unambiguous).
4. ⏳ **NumLock on/off, key repeat, arrow/navigation keys** on the real keypad.
5. ⏳ **Action regressions** — Steam, audio, gaming mode, tray, phone remote.

> Physical isolation is **not claimed as verified** until the two-separate-keyboard test is done.

---

## P1 — Code gaps

1. ✅/❌ **`rdr2_cycle`** — **REMOVED BY USER REQUEST.** Action, panel, settings, registry, store fields, Rust whitelist, and executor branch are gone. **Do not re-add.**
2. ✅ **`waitUntilOpen`** — native Rust implementation added: checks `Steam.exe` via ToolHelp and polls `loginusers.vdf` for `MostRecent` up to the configured timeout. Settings now pass `steamExecutable` to the action.
3. ✅ **Custom `trigger` editing** — editable/validated `trigger` field added to `ButtonEditor`; Device Manager can calibrate an unknown detected key and assign it to the selected button via `setButtonPhysicalKey`.
4. ✅ **Multiple HID collections** — opt-in checkbox added. Default is exact selected HID collection only; when enabled, sibling collections with the same hardware ID are also accepted.
5. ✅ **Dead settings** — unused `auto_start_engine` / `engine_mode` removed from Rust and TypeScript models/defaults.
6. ✅ **Log access in UI** — "Open logs folder" added to Profile Manager Tools.
7. ✅ **Config tools in UI** — "Export config", "Import config", and restore-from-backup selector added to Profile Manager Tools.
8. ✅ **Production installer build** — `pnpm tauri build` completes with exit code 0. MSI + NSIS installers and both updater `.sig` files are produced.
9. ✅ **Auto-updater release** — v0.1.4 source pushed to `main`, release published, and all signed assets uploaded: https://github.com/Aymansomo/ayman-macro-deck/releases/tag/v0.1.4 . GitHub still has v0.1.3 with the old signing key; it was not overwritten.

---

## P2 — Cleanup / docs / tests

1. 🟡 Update stale docs: `AGENTS.md` (AHK no longer executor), `PROJECT_STATE.md` (Phase 3), `README.md` (template). This roadmap and `IMPLEMENTATION_SUMMARY.md` are current.
2. ⏳ Remove dead components `ButtonGrid.tsx` / `MacroButton.tsx` (no longer used by `App.tsx`).
3. ✅/⏳ Tests: added Rust tests for VDF parsing, Steam executable enrichment, and opt-in multi-collection dispatch. Device worker and live-config migration tests remain a good future addition.
4. ✅ **Responsive UI pass** — measured keyboard sizing with readable minimums, scroll instead of shrunken keys, grouped/scrollable full layout, responsive dashboard/header/dialogs/side panels. Built and typechecked locally; not yet in a published installer.
5. ✅ **Open Application picker** — adding the action opens a native file dialog; choosing an `.exe`/`.lnk` caches its Windows icon and auto-fills the button name, image and description. A "Choose app…" button allows re-picking.
6. ⏳ Record hardware test results in `IMPLEMENTATION_SUMMARY.md` after the user runs Step D.

---

## Build / release status

- `pnpm build` — passes (including the post-v0.1.4 responsive UI + application-picker changes).
- `cargo test --manifest-path "src-tauri\Cargo.toml"` — **40 passed, 0 failed**.
- Post-v0.1.4 UI changes are local-only; the published v0.1.4 installer does not include them yet.
- `pnpm tauri build` — passes with updater signing env vars set at user level.
- Signed artifacts (v0.1.4):
  - `src-tauri\target\release\bundle\msi\AYMAN MACRO DECK_0.1.4_x64_en-US.msi`
  - `src-tauri\target\release\bundle\msi\AYMAN MACRO DECK_0.1.4_x64_en-US.msi.sig`
  - `src-tauri\target\release\bundle\nsis\AYMAN MACRO DECK_0.1.4_x64-setup.exe`
  - `src-tauri\target\release\bundle\nsis\AYMAN MACRO DECK_0.1.4_x64-setup.exe.sig`
  - `src-tauri\target\release\bundle\latest.json`
- Published release: https://github.com/Aymansomo/ayman-macro-deck/releases/tag/v0.1.4
- Pushed source commit: `6d2e720` on `main`
- Private signing key: `C:\Users\Ayman\.tauri\ayman-macro-deck.key` (keep secret).
- Public key is embedded in `src-tauri/tauri.conf.json`.
- `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PATH`, and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` are set at Windows user scope so future builds can sign updater artifacts.
- **Signing-key migration:** v0.1.3 on GitHub was signed with the old key whose private key was unavailable. Because v0.1.4 embeds a new public key, existing v0.1.3 installs cannot auto-verify the v0.1.4 update; they must install the v0.1.4 setup manually once.

---

## Order of remaining work

1. User retests the two P0 Device Manager bugs.
2. User runs the P0 hardware tests, especially the two-keyboard isolation test.
3. ✅ Published the v0.1.4 GitHub release with `latest.json` + installers + `.sig` files. Existing v0.1.3 users still need one manual reinstall.
4. Record hardware results in `IMPLEMENTATION_SUMMARY.md`; then do P2 cleanup.
