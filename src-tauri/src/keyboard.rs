//! KeyboardManager — native keyboard engine.
//!
//! Isolation model:
//! - Macros are dispatched from `DeviceKeyEvent`s produced by the Raw Input
//!   manager, which verifies the originating `HANDLE` belongs to the selected
//!   USB keypad. The main keyboard can never trigger a macro.
//! - The `WH_KEYBOARD_LL` hook hides a bound deck key so it never types. It sets
//!   no state of its own: it attributes each candidate key to a device through
//!   the shared Raw Input record (see `deck_owns_key`), so the same scan code
//!   pressed on the main keyboard is passed through untouched.
//! - Actions still run through the one existing `actions::run_button` engine.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
use crate::models::{AppConfig, MacroButton};
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

/// Device name used for macros dispatched from the low-level hook. The hook
/// cannot report a device, so these events carry their own source tag.
const HOOK_SOURCE: &str = "hook";

/// Bound deck keys are swallowed by the low-level hook so they never type.
/// Windows cannot tell keyboards apart at this layer, so the hook asks the Raw
/// Input records which device produced the key before hiding it.
static SUPPRESS_BOUND_KEYS: AtomicBool = AtomicBool::new(true);

/// A physical key event seen by the Raw Input listener, used to attribute a
/// `WH_KEYBOARD_LL` callback (which carries no device) to a real keyboard.
#[derive(Clone, Copy, Debug)]
struct RawKeyRecord {
    /// True when the record came from the selected macro keypad.
    deck: bool,
    scan: u16,
    extended: bool,
    down: bool,
    at: Instant,
}

/// Raw Input and the low-level hook race for the same key press: either may be
/// reported first. Records are kept this long and the hook waits briefly for the
/// matching one instead of guessing (guessing would eat main-keyboard keys).
const RAW_KEY_TTL: Duration = Duration::from_millis(150);
const HOOK_RAW_WAIT: Duration = Duration::from_millis(6);

static RAW_KEYS: Mutex<VecDeque<RawKeyRecord>> = Mutex::new(VecDeque::new());

/// Called by the Raw Input listener for every physical key event it receives.
/// `deck` is the same isolation verdict used to dispatch macros, so a key is
/// only hidden when the selected keypad really produced it.
pub fn record_raw_key(deck: bool, scan: u16, extended: bool, down: bool) {
    let now = Instant::now();
    let mut keys = RAW_KEYS.lock().unwrap();
    while keys
        .front()
        .is_some_and(|key| now.duration_since(key.at) >= RAW_KEY_TTL)
    {
        keys.pop_front();
    }
    keys.push_back(RawKeyRecord {
        deck,
        scan,
        extended,
        down,
        at: now,
    });
}

/// Removes the newest unconsumed record for this exact key event and reports
/// whether it came from the macro keypad. `None` means Raw Input has not
/// reported the event yet.
fn take_raw_key(scan: u16, extended: bool, down: bool) -> Option<bool> {
    let now = Instant::now();
    let mut keys = RAW_KEYS.lock().unwrap();
    while keys
        .front()
        .is_some_and(|key| now.duration_since(key.at) >= RAW_KEY_TTL)
    {
        keys.pop_front();
    }
    let index = keys
        .iter()
        .rposition(|key| key.scan == scan && key.extended == extended && key.down == down)?;
    keys.remove(index).map(|key| key.deck)
}

/// True only when the selected macro keypad produced this key. The main
/// keyboard's identical key is never hidden or turned into a macro.
fn deck_owns_key(scan: u16, extended: bool, down: bool) -> bool {
    if let Some(deck) = take_raw_key(scan, extended, down) {
        return deck;
    }
    let deadline = Instant::now() + HOOK_RAW_WAIT;
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
        if let Some(deck) = take_raw_key(scan, extended, down) {
            return deck;
        }
    }
    // Unknown source: keep typing unaffected rather than eat a main-keyboard key.
    false
}

pub fn suppress_bound_keys() -> bool {
    SUPPRESS_BOUND_KEYS.load(Ordering::SeqCst)
}

pub fn set_suppress_bound_keys(suppress: bool) {
    SUPPRESS_BOUND_KEYS.store(suppress, Ordering::SeqCst);
    if let Some(context) = CONTEXT.get() {
        context.swallowed_down.lock().unwrap().clear();
    }
    crate::logging::info(if suppress {
        "bound deck keys are captured: they run their macro and never type"
    } else {
        "bound deck keys type normally again"
    });
}

/// State for the LL hook: it hides bound deck keys from every application and
/// hands them to the one action worker.
struct HookContext {
    /// Bindings are kept only so the UI can display configured shortcuts.
    bindings: Mutex<HashMap<Shortcut, Binding>>,
    /// `(scan, extended)` of keys that must not reach the focused window.
    swallow: Mutex<HashSet<(u16, bool)>>,
    /// Bound keys currently held down, so key-up and auto-repeat stay hidden.
    swallowed_down: Mutex<HashSet<(u16, bool)>>,
    dispatch: Sender<DeviceKeyEvent>,
    events: AtomicU64,
    physical_events: AtomicU64,
    paused: AtomicBool,
}

impl HookContext {
    fn new(dispatch: Sender<DeviceKeyEvent>) -> Self {
        Self {
            bindings: Mutex::new(HashMap::new()),
            swallow: Mutex::new(HashSet::new()),
            swallowed_down: Mutex::new(HashSet::new()),
            dispatch,
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
    let scan = info.scanCode as u16;
    let extended = flags & 0x01 != 0;
    let injected = flags & LLKHF_INJECTED != 0;
    context.observe(KeyInput {
        scan,
        extended,
        up,
        injected,
    });

    // Keys produced by our own actions are never swallowed.
    if injected {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let capture = suppress_bound_keys() && !context.paused.load(Ordering::SeqCst);

    let captured = capture && context.swallow.lock().unwrap().contains(&(scan, extended));

    if down && captured && deck_owns_key(scan, extended, true) {
        // Hide the key from every application and run its macro from the
        // action worker. Auto-repeat only counts as one press.
        let first_press = context
            .swallowed_down
            .lock()
            .unwrap()
            .insert((scan, extended));
        if first_press {
            let _ = context.dispatch.send(DeviceKeyEvent {
                device_id: HOOK_SOURCE.to_string(),
                key_id: KeyId::from_raw(scan, extended, info.vkCode as u16).id(),
                down: true,
            });
        }
        return LRESULT(1);
    }

    if up && context.swallowed_down.lock().unwrap().remove(&(scan, extended)) {
        let _ = take_raw_key(scan, extended, false);
        let _ = context.dispatch.send(DeviceKeyEvent {
            device_id: HOOK_SOURCE.to_string(),
            key_id: KeyId::from_raw(scan, extended, info.vkCode as u16).id(),
            down: false,
        });
        return LRESULT(1);
    }

    CallNextHookEx(None, code, wparam, lparam)
}

/// Physical keys the hook must hide: enabled buttons with real actions whose
/// trigger (or pinned physical key) is a bare, unmodified key.
fn capture_keys(core: &AppCore) -> HashSet<(u16, bool)> {
    let config = core.config();
    let Some(profile) = config.active_profile(core.gaming()) else {
        return HashSet::new();
    };

    let mut keys = HashSet::new();
    for button in &profile.buttons {
        if !button.enabled || button.actions.is_empty() {
            continue;
        }
        if let Some(scan) = button
            .physical_key_id
            .as_deref()
            .and_then(KeyId::parse)
            .and_then(|key| key.scan())
        {
            keys.insert(scan);
        }
        let Ok(Some(shortcut)) = button.effective_trigger() else {
            continue;
        };
        if shortcut.ctrl || shortcut.alt || shortcut.shift || shortcut.win {
            // A modified shortcut must never hide the bare key.
            continue;
        }
        if let Some(scan) = shortcut.key.scan() {
            keys.insert(scan);
        }
    }
    keys
}

fn build_bindings(core: &AppCore) -> (HashMap<Shortcut, Binding>, Vec<String>) {
    build_bindings_for(&core.config(), core.gaming())
}

/// The single definition of "this physical key runs a button": physical identity
/// wins over labels and NumLock, then the configured trigger, then the legacy
/// numeric key. Also used by the Raw Input layer to decide whether a keypad key
/// must stay swallowed in capture mode.
pub fn find_button_for_key(config: &AppConfig, gaming: bool, key: &KeyId) -> Option<MacroButton> {
    let profile = config.active_profile(gaming)?;
    let legacy = key.legacy_key();

    profile
        .buttons
        .iter()
        .find(|button| {
            if !button.enabled {
                return false;
            }
            if let Some(id) = button.physical_key_id.as_deref().and_then(KeyId::parse) {
                if id == *key {
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
        .cloned()
}

fn build_bindings_for(
    config: &crate::models::AppConfig,
    gaming: bool,
) -> (HashMap<Shortcut, Binding>, Vec<String>) {
    let mut bindings: HashMap<Shortcut, Binding> = HashMap::new();
    let mut problems: Vec<String> = Vec::new();

    let Some(profile) = config.active_profile(gaming) else {
        return (bindings, problems);
    };
    let active_profile_id = profile.id.clone();

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

    // Profile-switch and mode-toggle buttons stay reachable from every profile
    // so a keypad key can always jump to another profile. `find_button` already
    // looks across all profiles, so these bindings dispatch the original button.
    // The active profile owns its keys; only unused triggers are borrowed.
    for other in &config.profiles {
        if other.id == active_profile_id {
            continue;
        }
        for button in &other.buttons {
            if !button.enabled || !crate::actions::is_mode_action(button) {
                continue;
            }
            let Ok(Some(shortcut)) = button.effective_trigger() else {
                continue;
            };
            if bindings.contains_key(&shortcut) {
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
    pub fn new(
        core: Arc<AppCore>,
        device_receiver: Receiver<DeviceKeyEvent>,
        device_sender: Sender<DeviceKeyEvent>,
    ) -> Arc<Self> {
        Arc::new(Self {
            core,
            context: Arc::new(HookContext::new(device_sender)),
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

    /// Rebuilds the shown shortcuts and the set of keys the hook captures.
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
        *self
            .context
            .swallow
            .lock()
            .expect("capture lock poisoned") = capture_keys(&self.core);
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
    // A swallowed key can also arrive through Raw Input; run its macro once.
    let mut last_fired: HashMap<String, Instant> = HashMap::new();

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

        // Stable physical identity wins: labels and NumLock can change, but
        // scan code + extended flag cannot. Shared with the capture decision in
        // `device.rs` so a swallowed key always has a macro behind it.
        let Some(button) = find_button_for_key(&config, core.gaming(), &key) else {
            crate::logging::info(format!(
                "no assignment for {} on {}",
                event.key_id, profile.name
            ));
            continue;
        };

        let now = Instant::now();
        if last_fired
            .get(&event.key_id)
            .is_some_and(|previous| now.duration_since(*previous) < Duration::from_millis(150))
        {
            continue;
        }
        last_fired.insert(event.key_id.clone(), now);

        crate::logging::info(format!(
            "{} on {} -> {} ({})",
            event.key_id, event.device_id, button.name, button.id
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

        if let Err(error) = core.app().state::<Arc<KeyboardManager>>().reload() {
            crate::logging::warn(format!("cannot refresh shortcuts: {}", error));
        }

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
        let (sender, _receiver) = channel::<DeviceKeyEvent>();
        let context = Arc::new(HookContext::new(sender));
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
    fn only_the_selected_keypad_owns_a_captured_key() {
        record_raw_key(true, 0x50, true, true);
        assert!(deck_owns_key(0x50, true, true), "the deck key is captured");

        // Same scan code from the main keyboard (or an unknown source) must
        // keep typing and never run a macro.
        record_raw_key(false, 0x50, true, true);
        assert!(!deck_owns_key(0x50, true, true));
        assert!(!deck_owns_key(0x48, true, true), "unrecorded key passes through");
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

    #[test]
    fn profile_switch_buttons_stay_bound_from_other_profiles() {
        let config = AppConfig {
            active_profile_id: "work".into(),
            profiles: vec![
                Profile {
                    id: "n".into(),
                    name: "Normal".into(),
                    buttons: vec![MacroButton {
                        id: "to_normal".into(),
                        trigger: Some("Numpad1".into()),
                        name: "Switch to Normal".into(),
                        actions: vec![ButtonAction {
                            action_type: "switch_profile".into(),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }],
                    ..Default::default()
                },
                Profile {
                    id: "work".into(),
                    name: "Work".into(),
                    buttons: vec![MacroButton {
                        id: "work_button".into(),
                        trigger: Some("Numpad2".into()),
                        name: "Work button".into(),
                        ..Default::default()
                    }],
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        let (bindings, _) = build_bindings_for(&config, false);
        assert_eq!(bindings.len(), 2);
        assert!(
            bindings.values().any(|b| b.button_id == "to_normal"),
            "the switch button from Normal must remain reachable while Work is active"
        );
    }
}
