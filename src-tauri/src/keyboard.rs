//! KeyboardManager — native keyboard engine.
//!
//! Isolation model (Stage 1):
//! - The `WH_KEYBOARD_LL` hook is now a **pure observer**. It never consumes a
//!   key and can never dispatch a macro, so the main keyboard cannot trigger
//!   anything.
//! - Macros are dispatched only from `DeviceKeyEvent`s produced by the Raw
//!   Input manager, which verifies the originating `HANDLE` belongs to the
//!   selected USB keypad.
//! - Actions still run through the one existing `actions::run_button` engine.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager};
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, PostThreadMessageW, SetWindowsHookExW,
    TranslateMessage, UnhookWindowsHookEx, HC_ACTION, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL,
    WM_KEYDOWN, WM_KEYUP, WM_QUIT, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

use crate::device::DeviceKeyEvent;
use crate::keyid::KeyId;
use crate::shortcut::Shortcut;
use crate::state::AppCore;

const LLKHF_INJECTED: u32 = 0x10;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutInfo {
    pub shortcut: String,
    pub button_id: String,
    pub button_name: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardStatus {
    pub running: bool,
    pub paused: bool,
    pub error: Option<String>,
    pub shortcut_count: usize,
    pub shortcuts: Vec<ShortcutInfo>,
    /// Events seen by the low-level observer (diagnostics only).
    pub events_seen: u64,
    /// Macros dispatched from the selected Raw Input device.
    pub triggers: u64,
}

#[derive(Clone, Debug)]
struct Binding {
    button_id: String,
    button_name: String,
}

#[derive(Clone, Copy, Debug)]
pub struct KeyInput {
    pub scan: u16,
    pub extended: bool,
    pub up: bool,
    pub injected: bool,
}

/// Diagnostic state for the LL hook. It deliberately has no dispatch channel,
/// no key-down set and no bindings lookup used for execution.
struct HookContext {
    /// Bindings are kept only so the UI can display configured shortcuts.
    bindings: Mutex<HashMap<Shortcut, Binding>>,
    events: AtomicU64,
    physical_events: AtomicU64,
    paused: AtomicBool,
}

impl HookContext {
    fn new() -> Self {
        Self {
            bindings: Mutex::new(HashMap::new()),
            events: AtomicU64::new(0),
            physical_events: AtomicU64::new(0),
            paused: AtomicBool::new(false),
        }
    }

    /// Observes an event for diagnostics. Always returns; never blocks input.
    fn observe(&self, input: KeyInput) {
        let seen = self.events.fetch_add(1, Ordering::Relaxed);
        if seen < 4 {
            crate::logging::info(format!(
                "observed key event #{} scan={} extended={} up={} injected={}",
                seen + 1,
                input.scan,
                input.extended,
                input.up,
                input.injected
            ));
        }
        if !input.injected {
            self.physical_events.fetch_add(1, Ordering::Relaxed);
        }
        let _ = self.paused.load(Ordering::SeqCst);
    }
}

static CONTEXT: OnceLock<Arc<HookContext>> = OnceLock::new();

unsafe extern "system" fn keyboard_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code != HC_ACTION as i32 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let Some(context) = CONTEXT.get() else {
        return CallNextHookEx(None, code, wparam, lparam);
    };

    let info = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
    let message = wparam.0 as u32;
    let up = message == WM_KEYUP || message == WM_SYSKEYUP;
    let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;

    if !up && !down {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let flags = info.flags.0;
    context.observe(KeyInput {
        scan: info.scanCode as u16,
        extended: flags & 0x01 != 0,
        up,
        injected: flags & LLKHF_INJECTED != 0,
    });

    // Never consume the key. Never dispatch a macro.
    CallNextHookEx(None, code, wparam, lparam)
}

fn build_bindings(core: &AppCore) -> (HashMap<Shortcut, Binding>, Vec<String>) {
    let config = core.config();
    let mut bindings: HashMap<Shortcut, Binding> = HashMap::new();
    let mut problems: Vec<String> = Vec::new();

    let Some(profile) = config.active_profile(core.gaming()) else {
        return (bindings, problems);
    };

    for button in &profile.buttons {
        if !button.enabled {
            continue;
        }

        match button.effective_trigger() {
            Ok(Some(shortcut)) => {
                if bindings.contains_key(&shortcut) {
                    problems.push(format!(
                        "{} is bound twice in profile \"{}\"",
                        shortcut, profile.name
                    ));
                    continue;
                }
                bindings.insert(
                    shortcut,
                    Binding {
                        button_id: button.id.clone(),
                        button_name: button.name.clone(),
                    },
                );
            }
            Ok(None) => {}
            Err(error) => problems.push(format!("button \"{}\": {}", button.name, error)),
        }
    }

    (bindings, problems)
}

pub struct KeyboardManager {
    core: Arc<AppCore>,
    context: Arc<HookContext>,
    thread: Mutex<Option<JoinHandle<()>>>,
    thread_id: AtomicU32,
    running: AtomicBool,
    error: Mutex<Option<String>>,
    device_receiver: Mutex<Option<Receiver<DeviceKeyEvent>>>,
    device_worker: Mutex<Option<JoinHandle<()>>>,
    device_triggers: Arc<AtomicU64>,
    pressed: Arc<Mutex<HashSet<(String, String)>>>,
    shutdown: Arc<AtomicBool>,
}

impl KeyboardManager {
    pub fn new(core: Arc<AppCore>, device_receiver: Receiver<DeviceKeyEvent>) -> Arc<Self> {
        Arc::new(Self {
            core,
            context: Arc::new(HookContext::new()),
            thread: Mutex::new(None),
            thread_id: AtomicU32::new(0),
            running: AtomicBool::new(false),
            error: Mutex::new(None),
            device_receiver: Mutex::new(Some(device_receiver)),
            device_worker: Mutex::new(None),
            device_triggers: Arc::new(AtomicU64::new(0)),
            pressed: Arc::new(Mutex::new(HashSet::new())),
            shutdown: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn start(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        // The device worker is the only macro dispatch path.
        if let Some(receiver) = self.device_receiver.lock().unwrap().take() {
            let manager = self.core.clone();
            let triggers = self.device_triggers.clone();
            let pressed = self.pressed.clone();
            let shutdown = self.shutdown.clone();
            let handle = thread::Builder::new()
                .name("deck-device-actions".to_string())
                .spawn(move || device_worker_loop(manager, receiver, triggers, pressed, shutdown))
                .map_err(|e| format!("cannot start device action worker: {}", e))?;
            *self.device_worker.lock().unwrap() = Some(handle);
        }

        let _ = CONTEXT.set(self.context.clone());

        let (ready_tx, ready_rx) = channel::<Result<u32, String>>();
        let handle = thread::Builder::new()
            .name("deck-keyboard-observer".to_string())
            .spawn(move || unsafe {
                match SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook), None, 0) {
                    Ok(hook) => {
                        let thread_id = GetCurrentThreadId();
                        crate::logging::info(format!(
                            "keyboard observer thread {} installed, pumping messages",
                            thread_id
                        ));
                        let _ = ready_tx.send(Ok(thread_id));
                        let mut message = MSG::default();
                        loop {
                            let result = GetMessageW(&mut message, None, 0, 0);
                            if result.0 <= 0 {
                                crate::logging::warn(format!(
                                    "observer message loop ended (GetMessage returned {})",
                                    result.0
                                ));
                                break;
                            }
                            let _ = TranslateMessage(&message);
                            DispatchMessageW(&message);
                        }
                        let _ = UnhookWindowsHookEx(hook);
                        crate::logging::info("keyboard observer uninstalled");
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "cannot install keyboard observer: {}",
                            error
                        )));
                    }
                }
            })
            .map_err(|e| format!("cannot start keyboard observer thread: {}", e))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(thread_id)) => {
                self.thread_id.store(thread_id, Ordering::SeqCst);
                *self.thread.lock().expect("thread lock poisoned") = Some(handle);
                self.running.store(true, Ordering::SeqCst);
                *self.error.lock().expect("error lock poisoned") = None;
                let count = self.reload()?;
                crate::logging::info(format!("keyboard observer running; {} configured shortcut(s)", count));
                Ok(())
            }
            Ok(Err(error)) => {
                *self.error.lock().expect("error lock poisoned") = Some(error.clone());
                crate::logging::error(&error);
                Err(error)
            }
            Err(_) => {
                let error = "keyboard observer did not start in time".to_string();
                *self.error.lock().expect("error lock poisoned") = Some(error.clone());
                crate::logging::error(&error);
                Err(error)
            }
        }
    }

    pub fn stop(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if self.running.swap(false, Ordering::SeqCst) {
            let thread_id = self.thread_id.swap(0, Ordering::SeqCst);
            if thread_id != 0 {
                unsafe {
                    let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
                }
            }
            if let Some(handle) = self.thread.lock().expect("thread lock poisoned").take() {
                let _ = handle.join();
            }
            crate::logging::info("keyboard observer stopped");
        }

        // The device worker observes the shutdown flag on its next event; the
        // Raw Input listener is stopped by RawInputManager::stop() as well.
        self.pressed.lock().unwrap().clear();
    }

    /// Rebuilds the informational key -> button map from the active profile.
    pub fn reload(&self) -> Result<usize, String> {
        let (bindings, problems) = build_bindings(&self.core);
        for problem in &problems {
            crate::logging::warn(format!("shortcut: {}", problem));
        }
        let count = bindings.len();
        *self
            .context
            .bindings
            .lock()
            .expect("bindings lock poisoned") = bindings;
        Ok(count)
    }

    pub fn set_paused(&self, paused: bool) {
        self.context.paused.store(paused, Ordering::SeqCst);
        if !paused {
            self.pressed.lock().unwrap().clear();
        }
        crate::logging::info(if paused {
            "macro engine paused"
        } else {
            "macro engine resumed"
        });
    }

    pub fn paused(&self) -> bool {
        self.context.paused.load(Ordering::SeqCst)
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn status(&self) -> KeyboardStatus {
        let bindings = self.context.bindings.lock().expect("bindings lock poisoned");
        let mut shortcuts: Vec<ShortcutInfo> = bindings
            .iter()
            .map(|(shortcut, binding)| ShortcutInfo {
                shortcut: shortcut.to_string(),
                button_id: binding.button_id.clone(),
                button_name: binding.button_name.clone(),
            })
            .collect();
        shortcuts.sort_by(|a, b| a.shortcut.cmp(&b.shortcut));

        KeyboardStatus {
            running: self.running.load(Ordering::SeqCst),
            paused: self.paused(),
            error: self.error.lock().expect("error lock poisoned").clone(),
            shortcut_count: shortcuts.len(),
            shortcuts,
            events_seen: self.context.events.load(Ordering::Relaxed),
            triggers: self.device_triggers.load(Ordering::SeqCst),
        }
    }

    /// Runs a button exactly like a physical key press. Used by the phone
    /// remote and tests; every entry point shares the one ActionEngine.
    pub fn dispatch_button(&self, button_id: &str) -> Vec<String> {
        let Some(button) = self.core.find_button(button_id) else {
            return vec![format!("button {} not found", button_id)];
        };
        if !button.enabled {
            return vec![format!("button {} is disabled", button.name)];
        }

        let results = crate::actions::run_button(&self.core, &button);
        let mode_action = crate::actions::is_mode_action(&button);
        if self.core.gaming() && !mode_action {
            self.core.set_gaming(false);
        }
        let _ = self.reload();
        results
    }
}

fn device_worker_loop(
    core: Arc<AppCore>,
    receiver: Receiver<DeviceKeyEvent>,
    triggers: Arc<AtomicU64>,
    pressed: Arc<Mutex<HashSet<(String, String)>>>,
    shutdown: Arc<AtomicBool>,
) {
    for event in receiver {
        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        if core.app().state::<Arc<KeyboardManager>>().paused() {
            continue;
        }

        if event.down {
            if !pressed
                .lock()
                .unwrap()
                .insert((event.device_id.clone(), event.key_id.clone()))
            {
                continue;
            }
        } else {
            pressed
                .lock()
                .unwrap()
                .remove(&(event.device_id.clone(), event.key_id.clone()));
            continue; // Actions fire on key-down only.
        }

        let Some(key) = KeyId::parse(&event.key_id) else {
            crate::logging::warn(format!("unsupported physical key id: {}", event.key_id));
            continue;
        };

        let config = core.config();
        let Some(profile) = config.active_profile(core.gaming()) else {
            continue;
        };
        let legacy = key.legacy_key();

        let button = profile
            .buttons
            .iter()
            .find(|button| {
                if !button.enabled {
                    return false;
                }
                // Stable physical identity wins: labels and NumLock can change,
                // but scan code + extended flag cannot.
                if let Some(id) = button
                    .physical_key_id
                    .as_deref()
                    .and_then(KeyId::parse)
                {
                    if id == key {
                        return true;
                    }
                }
                let trigger_match = button
                    .effective_trigger()
                    .ok()
                    .flatten()
                    .map(|shortcut| key.matches_shortcut(&shortcut))
                    .unwrap_or(false);
                trigger_match || legacy.is_some() && button.key == legacy
            })
            .cloned();

        let Some(button) = button else {
            crate::logging::info(format!(
                "no assignment for {} on {}",
                event.key_id, profile.name
            ));
            continue;
        };

        crate::logging::info(format!(
            "{} on selected keypad -> {} ({})",
            event.key_id, button.name, button.id
        ));
        let _ = core.app().emit(
            "macro-action-started",
            serde_json::json!({
                "deviceId": event.device_id,
                "keyId": event.key_id,
                "buttonId": button.id,
                "buttonName": button.name,
            }),
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::actions::run_button(&core, &button)
        }));
        let results = match result {
            Ok(results) => results,
            Err(_) => vec!["internal error: action engine panicked".to_string()],
        };

        triggers.fetch_add(1, Ordering::SeqCst);
        let mode_action = crate::actions::is_mode_action(&button);
        if core.gaming() && !mode_action {
            core.set_gaming(false);
        }

        let (bindings, problems) = build_bindings(&core);
        for problem in problems {
            crate::logging::warn(format!("shortcut: {}", problem));
        }
        *core
            .app()
            .state::<Arc<KeyboardManager>>()
            .context
            .bindings
            .lock()
            .unwrap() = bindings;

        let event_name = if results_have_error(&results) {
            "macro-action-error"
        } else {
            "macro-action-completed"
        };
        let _ = core.app().emit(
            event_name,
            serde_json::json!({
                "deviceId": event.device_id,
                "keyId": event.key_id,
                "buttonId": button.id,
                "buttonName": button.name,
                "results": results,
                "gamingMode": core.gaming(),
            }),
        );
        let _ = core.app().emit(
            "deck:triggered",
            serde_json::json!({
                "source": "device",
                "deviceId": event.device_id,
                "key": event.key_id,
                "buttonId": button.id,
                "buttonName": button.name,
                "results": results,
                "gamingMode": core.gaming(),
            }),
        );
        core.emit_state(&core.app().state::<Arc<KeyboardManager>>().status());
    }
}

fn results_have_error(results: &[String]) -> bool {
    results.iter().any(|message| {
        let lower = message.to_ascii_lowercase();
        lower.contains("failed")
            || lower.contains("missing")
            || lower.contains("unknown action")
            || lower.contains("not found")
            || lower.contains("not a valid")
            || lower.contains("skipped")
            || lower.contains("panicked")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AppConfig, ButtonAction, MacroButton, Profile};

    fn context_with(shortcut: &str, button_id: &str) -> Arc<HookContext> {
        let context = Arc::new(HookContext::new());
        let parsed = Shortcut::parse(shortcut).unwrap();
        context.bindings.lock().unwrap().insert(
            parsed,
            Binding {
                button_id: button_id.to_string(),
                button_name: "Test".to_string(),
            },
        );
        context
    }

    fn input(shortcut: &str, up: bool) -> KeyInput {
        let parsed = Shortcut::parse(shortcut).unwrap();
        let (scan, extended) = parsed.key.scan().unwrap();
        KeyInput {
            scan,
            extended,
            up,
            injected: false,
        }
    }

    #[test]
    fn observer_never_dispatches_or_consumes() {
        let context = context_with("Numpad1", "b1");
        let before = context.bindings.lock().unwrap().len();
        context.observe(input("Numpad1", false));
        context.observe(input("Numpad1", true));
        assert_eq!(context.events.load(Ordering::Relaxed), 2);
        assert_eq!(context.physical_events.load(Ordering::Relaxed), 2);
        assert_eq!(context.bindings.lock().unwrap().len(), before);
    }

    #[test]
    fn injected_events_are_not_counted_as_physical() {
        let context = context_with("Numpad1", "b1");
        let mut injected = input("Numpad1", false);
        injected.injected = true;
        context.observe(injected);
        assert_eq!(context.events.load(Ordering::Relaxed), 1);
        assert_eq!(context.physical_events.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn action_errors_are_detected_for_ui_events() {
        assert!(results_have_error(&["Action x failed: boom".into()]));
        assert!(!results_have_error(&["Opened URL: https://example.com".into()]));
    }

    #[test]
    fn bindings_come_from_the_active_profile_only() {
        let config = AppConfig {
            active_profile_id: "n".into(),
            profiles: vec![
                Profile {
                    id: "g".into(),
                    name: "Gaming".into(),
                    buttons: vec![MacroButton {
                        id: "gb".into(),
                        key: Some(1),
                        name: "Gaming 1".into(),
                        actions: vec![ButtonAction {
                            action_type: "volume_up".into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                Profile {
                    id: "n".into(),
                    name: "Normal".into(),
                    buttons: vec![MacroButton {
                        id: "nb".into(),
                        key: Some(2),
                        name: "Normal 2".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        assert_eq!(config.active_profile(false).unwrap().id, "n");
        assert_eq!(config.active_profile(true).unwrap().id, "g");
    }
}
