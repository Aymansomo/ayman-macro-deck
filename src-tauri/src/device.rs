//! Windows Raw Input device manager.
//!
//! Raw Input is the only keyboard API that reports *which physical device*
//! generated an event. Unlike `WH_KEYBOARD_LL`, it can therefore enforce the
//! "main keyboard never fires macros" rule.
//!
//! Design decision: we deliberately do **not** use `RIDEV_NOLEGACY`.
//! `RAWINPUTDEVICE` has no per-device handle field — registration is per HID
//! top-level collection, and every keyboard shares usage page 1 / usage 6.
//! Setting `RIDEV_NOLEGACY` would therefore suppress legacy input for the main
//! keyboard too. Raw Input is used for identification and triggering only;
//! legacy typing is left untouched.

use std::collections::HashSet;
use std::ffi::c_void;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HANDLE, HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::{
    GetRawInputData, GetRawInputDeviceInfoW, GetRawInputDeviceList, RegisterRawInputDevices,
    HRAWINPUT, RAWINPUT, RAWINPUTDEVICE, RAWINPUTDEVICELIST, RAWINPUTHEADER, RAWKEYBOARD,
    RIDI_DEVICENAME, RID_INPUT, RIDEV_DEVNOTIFY, RIDEV_INPUTSINK, RIDEV_REMOVE, RIM_TYPEKEYBOARD,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, PostQuitMessage,
    PostThreadMessageW, RegisterClassW, TranslateMessage, UnregisterClassW, WNDCLASSW,
    RI_KEY_BREAK, RI_KEY_E0, RI_KEY_E1, GIDC_ARRIVAL, GIDC_REMOVAL, HWND_MESSAGE, WM_DESTROY,
    WM_INPUT, WM_INPUT_DEVICE_CHANGE, WM_QUIT,
};

use crate::keyid::KeyId;

const HID_USAGE_PAGE_GENERIC: u16 = 0x01;
const HID_USAGE_GENERIC_KEYBOARD: u16 = 0x06;

/// A physical keyboard as reported by Windows Raw Input.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardDevice {
    /// Stable identity: the normalized HID device interface path.
    pub id: String,
    /// Human-readable identity built from real hardware identifiers.
    pub name: String,
    /// Raw device interface path as returned by `RIDI_DEVICENAME`.
    pub path: String,
    /// e.g. `VID_046D&PID_C52B&MI_00`, parsed from the path.
    pub hardware_id: String,
    pub connected: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub running: bool,
    pub error: Option<String>,
    pub devices: Vec<KeyboardDevice>,
    pub selected_id: Option<String>,
    pub selected_connected: bool,
    pub testing: bool,
    /// Always false. Documented here because per-device suppression is not a
    /// Windows Raw Input capability (see module docs).
    pub suppress_legacy: bool,
    /// When true, every connected HID collection sharing the selected keypad's
    /// hardware ID is accepted as the same physical device.
    pub include_collections: bool,
}

#[derive(Clone, Debug)]
pub struct DeviceKeyEvent {
    /// Identity of the device that actually produced the event.
    pub device_id: String,
    pub key_id: String,
    pub down: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceTestResult {
    pub device_id: String,
    pub device_name: String,
    pub key_id: String,
    pub key_label: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
struct DeviceSettings {
    selected_id: Option<String>,
    selected_hardware_id: Option<String>,
    selected_name: Option<String>,
    #[serde(default)]
    include_collections: bool,
}

static MANAGER: OnceLock<Arc<RawInputManager>> = OnceLock::new();

pub struct RawInputManager {
    app: AppHandle,
    tx: Sender<DeviceKeyEvent>,
    devices: RwLock<Vec<KeyboardDevice>>,
    selected: RwLock<Option<KeyboardDevice>>,
    /// Key-down state per (device, key). Auto-repeat is ignored until key-up.
    pressed: Mutex<HashSet<(String, String)>>,
    test_candidate: RwLock<Option<KeyboardDevice>>,
    include_collections: AtomicBool,
    settings_path: Mutex<PathBuf>,
    testing: AtomicBool,
    running: AtomicBool,
    error: Mutex<Option<String>>,
    thread_id: AtomicU32,
    hwnd: Mutex<Option<isize>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl RawInputManager {
    /// Installs the manager used by the Raw Input window procedure. Must be
    /// called before `start()`.
    pub fn install(manager: Arc<Self>) {
        if MANAGER.set(manager).is_err() {
            crate::logging::warn("Raw Input manager was already installed");
        }
    }

    pub fn new(app: AppHandle, tx: Sender<DeviceKeyEvent>) -> Arc<Self> {
        let settings_path = crate::config_path(&app)
            .parent()
            .map(|dir| dir.join("device.json"))
            .unwrap_or_else(|| PathBuf::from("device.json"));

        let manager = Arc::new(Self {
            app,
            tx,
            devices: RwLock::new(Vec::new()),
            selected: RwLock::new(None),
            pressed: Mutex::new(HashSet::new()),
            test_candidate: RwLock::new(None),
            include_collections: AtomicBool::new(false),
            settings_path: Mutex::new(settings_path),
            testing: AtomicBool::new(false),
            running: AtomicBool::new(false),
            error: Mutex::new(None),
            thread_id: AtomicU32::new(0),
            hwnd: Mutex::new(None),
            thread: Mutex::new(None),
        });

        // Enumerate first so a saved selection can be matched against real
        // devices. The second refresh re-tags the newly restored selection.
        manager.refresh();
        manager.load_settings();
        manager.refresh();
        manager
    }

    fn load_settings(&self) {
        let path = self.settings_path.lock().unwrap().clone();
        let Ok(text) = fs::read_to_string(path) else { return };
        let Ok(saved) = serde_json::from_str::<DeviceSettings>(&text) else { return };
        self.include_collections
            .store(saved.include_collections, Ordering::SeqCst);

        let devices = self.devices.read().unwrap();
        let identity = saved.selected_id.as_deref().unwrap_or("");
        let hardware = saved.selected_hardware_id.as_deref().unwrap_or("");

        // Prefer the exact interface path. If the device reappeared with a new
        // instance path, adopt it only when the hardware identity is unique.
        let matched = devices
            .iter()
            .find(|d| !identity.is_empty() && d.id == identity)
            .or_else(|| {
                if hardware.is_empty() {
                    return None;
                }
                let mut candidates = devices.iter().filter(|d| d.hardware_id == hardware);
                let first = candidates.next()?;
                if candidates.next().is_some() {
                    return None; // ambiguous — never guess
                }
                crate::logging::warn(format!(
                    "selected keypad reattached with a new path; adopting {}",
                    first.id
                ));
                Some(first)
            })
            .cloned();

        if let Some(device) = matched {
            *self.selected.write().unwrap() = Some(device);
        } else if !identity.is_empty() {
            crate::logging::warn("selected keypad is not connected; keeping the saved identity");
            *self.selected.write().unwrap() = Some(KeyboardDevice {
                id: identity.to_string(),
                name: saved.selected_name.clone().unwrap_or_else(|| "Keypad".into()),
                path: identity.to_string(),
                hardware_id: hardware.to_string(),
                connected: false,
                selected: true,
            });
        }
    }

    fn save_settings(&self) {
        let selected = self.selected.read().unwrap().clone();
        let settings = DeviceSettings {
            selected_id: selected.as_ref().map(|d| d.id.clone()),
            selected_hardware_id: selected.as_ref().map(|d| d.hardware_id.clone()),
            selected_name: selected.as_ref().map(|d| d.name.clone()),
            include_collections: self.include_collections.load(Ordering::SeqCst),
        };
        if let Ok(json) = serde_json::to_string_pretty(&settings) {
            let path = self.settings_path.lock().unwrap().clone();
            if let Some(dir) = path.parent() {
                let _ = fs::create_dir_all(dir);
            }
            let _ = fs::write(path, json);
        }
    }

    /// Re-enumerates keyboards from `GetRawInputDeviceList`.
    pub fn refresh(&self) -> Vec<KeyboardDevice> {
        let mut found = Vec::new();
        unsafe {
            let mut count = 0u32;
            let size = std::mem::size_of::<RAWINPUTDEVICELIST>() as u32;
            GetRawInputDeviceList(None, &mut count, size);
            if count > 0 {
                let mut list = vec![RAWINPUTDEVICELIST::default(); count as usize];
                let got = GetRawInputDeviceList(Some(list.as_mut_ptr()), &mut count, size);
                for item in list.iter().take(got as usize) {
                    if item.dwType != RIM_TYPEKEYBOARD {
                        continue;
                    }
                    if let Some(path) = device_path(item.hDevice) {
                        found.push(make_device(path));
                    }
                }
            }
        }

        let selected_id = self.selected.read().unwrap().as_ref().map(|d| d.id.clone());
        for device in &mut found {
            device.selected = selected_id.as_deref() == Some(device.id.as_str());
        }

        let previous: HashSet<String> = self
            .devices
            .read()
            .unwrap()
            .iter()
            .map(|d| d.id.clone())
            .collect();
        let current: HashSet<String> = found.iter().map(|d| d.id.clone()).collect();

        {
            let mut selected = self.selected.write().unwrap();
            if let Some(id) = &selected_id {
                if let Some(live) = found.iter().find(|d| d.id == *id) {
                    *selected = Some(live.clone());
                } else if let Some(old) = selected.as_mut() {
                    old.connected = false;
                }
            }
        }
        *self.devices.write().unwrap() = found.clone();

        for id in current.difference(&previous) {
            crate::logging::info(format!("keyboard device discovered: {}", id));
            let _ = self.app.emit("keyboard-device-added", json!({ "deviceId": id }));
        }
        for id in previous.difference(&current) {
            crate::logging::info(format!("keyboard device removed: {}", id));
            let _ = self.app.emit("keyboard-device-removed", json!({ "deviceId": id }));
            self.clear_pressed_for_device(id);
            if self.selected.read().unwrap().as_ref().map(|d| d.id.as_str()) == Some(id.as_str()) {
                crate::logging::warn("selected keypad disconnected; macro input paused");
            }
        }

        found
    }

    pub fn devices(&self) -> Vec<KeyboardDevice> {
        self.devices.read().unwrap().clone()
    }

    pub fn selected(&self) -> Option<KeyboardDevice> {
        self.selected.read().unwrap().clone()
    }

    pub fn select(&self, device_id: &str) -> Result<KeyboardDevice, String> {
        let device = self
            .devices
            .read()
            .unwrap()
            .iter()
            .find(|d| d.id == device_id)
            .cloned()
            .ok_or_else(|| format!("Keyboard device {} is not connected", device_id))?;

        crate::logging::info(format!(
            "macro keypad selected: {} ({})",
            device.name, device.id
        ));
        *self.selected.write().unwrap() = Some(device.clone());
        self.pressed.lock().unwrap().clear();
        self.testing.store(false, Ordering::SeqCst);
        *self.test_candidate.write().unwrap() = None;
        self.save_settings();
        let _ = self.app.emit(
            "keyboard-device-selected",
            json!({ "deviceId": device.id, "deviceName": device.name }),
        );
        self.emit_status();
        Ok(device)
    }

    pub fn clear_selection(&self) {
        crate::logging::info("macro keypad selection cleared");
        *self.selected.write().unwrap() = None;
        self.pressed.lock().unwrap().clear();
        self.save_settings();
        self.emit_status();
    }

    /// Accepts every connected HID collection that shares the selected keypad's
    /// hardware ID. Explicit opt-in: a second identical keyboard is a different
    /// interface path and is only accepted when this is enabled.
    pub fn set_include_collections(&self, include: bool) -> DeviceStatus {
        self.include_collections.store(include, Ordering::SeqCst);
        self.save_settings();
        crate::logging::info(format!(
            "multi-collection keypad support {}",
            if include { "enabled" } else { "disabled" }
        ));
        self.emit_status();
        self.status()
    }

    pub fn include_collections(&self) -> bool {
        self.include_collections.load(Ordering::SeqCst)
    }

    pub fn start_test(&self) {
        *self.test_candidate.write().unwrap() = None;
        self.testing.store(true, Ordering::SeqCst);
        crate::logging::info("device test started: waiting for a physical key");
        self.emit_status();
    }

    pub fn stop_test(&self) {
        self.testing.store(false, Ordering::SeqCst);
        self.emit_status();
    }

    pub fn test_candidate(&self) -> Option<KeyboardDevice> {
        self.test_candidate.read().unwrap().clone()
    }

    pub fn is_testing(&self) -> bool {
        self.testing.load(Ordering::SeqCst)
    }

    pub fn status(&self) -> DeviceStatus {
        let selected = self.selected.read().unwrap().clone();
        let selected_id = selected.as_ref().map(|d| d.id.clone());
        let connected: HashSet<String> = self.devices.read().unwrap().iter().map(|d| d.id.clone()).collect();
        let selected_connected = selected_id.as_ref().map(|id| connected.contains(id)).unwrap_or(false);

        let mut devices = self.devices.read().unwrap().clone();
        for device in &mut devices {
            device.selected = selected_id.as_deref() == Some(device.id.as_str());
            device.connected = true;
        }
        if let Some(s) = selected {
            if !devices.iter().any(|d| d.id == s.id) {
                let mut disconnected = s;
                disconnected.connected = false;
                devices.push(disconnected);
            }
        }

        DeviceStatus {
            running: self.running.load(Ordering::SeqCst),
            error: self.error.lock().unwrap().clone(),
            devices,
            selected_id,
            selected_connected,
            testing: self.is_testing(),
            suppress_legacy: false,
            include_collections: self.include_collections(),
        }
    }

    pub fn emit_status(&self) {
        let _ = self.app.emit("keyboard-engine-status-changed", self.status());
    }

    pub fn start(&self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<u32, String>>();
        let handle = thread::Builder::new()
            .name("deck-raw-input".to_string())
            .spawn(move || raw_input_thread(ready_tx))
            .map_err(|e| format!("cannot start Raw Input thread: {}", e))?;

        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(thread_id)) => {
                self.thread_id.store(thread_id, Ordering::SeqCst);
                *self.thread.lock().unwrap() = Some(handle);
                self.running.store(true, Ordering::SeqCst);
                *self.error.lock().unwrap() = None;
                self.refresh();
                crate::logging::info("Raw Input keyboard listener started");
                self.emit_status();
                Ok(())
            }
            Ok(Err(error)) => {
                let _ = handle.join();
                *self.error.lock().unwrap() = Some(error.clone());
                crate::logging::error(&error);
                Err(error)
            }
            Err(_) => {
                let error = "Raw Input listener did not start in time".to_string();
                *self.error.lock().unwrap() = Some(error.clone());
                crate::logging::error(&error);
                Err(error)
            }
        }
    }

    pub fn stop(&self) {
        if !self.running.swap(false, Ordering::SeqCst) {
            return;
        }
        let thread_id = self.thread_id.load(Ordering::SeqCst);
        if thread_id != 0 {
            unsafe {
                let _ = PostThreadMessageW(thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
            }
        }
        if let Some(handle) = self.thread.lock().unwrap().take() {
            let _ = handle.join();
        }
        self.pressed.lock().unwrap().clear();
        crate::logging::info("Raw Input keyboard listener stopped");
        self.emit_status();
    }

    fn clear_pressed_for_device(&self, device_id: &str) {
        self.pressed
            .lock()
            .unwrap()
            .retain(|(id, _)| id != device_id);
    }

    fn on_input(&self, device_handle: HANDLE, keyboard: &RAWKEYBOARD) {
        // The originating device comes from RAWINPUTHEADER.hDevice. This is the
        // isolation boundary: a key from any other keyboard is never mapped to
        // a selected keypad slot, even if it has the same scan code.
        let Some(path) = (unsafe { device_path(device_handle) }) else { return };
        let actual_id = normalize_id(&path);
        let extended = keyboard.Flags & ((RI_KEY_E0 | RI_KEY_E1) as u16) != 0;
        let down = keyboard.Flags & RI_KEY_BREAK as u16 == 0;
        let key = KeyId::from_raw(keyboard.MakeCode, extended, keyboard.VKey);
        let key_id = key.id();

        if self.is_testing() {
            let candidate = self
                .devices
                .read()
                .unwrap()
                .iter()
                .find(|d| d.id == actual_id)
                .cloned()
                .unwrap_or_else(|| make_device(path.clone()));
            *self.test_candidate.write().unwrap() = Some(candidate.clone());
            let result = DeviceTestResult {
                device_id: candidate.id,
                device_name: candidate.name,
                key_id: key_id.clone(),
                key_label: key.label(),
            };
            crate::logging::info(format!(
                "device test key detected: {} on {}",
                result.key_id, result.device_name
            ));
            let _ = self.app.emit("device-test-result", result);
            return;
        }

        let Some(selected) = self.selected.read().unwrap().clone() else { return };
        let (connected, actual_hardware) = {
            let devices = self.devices.read().unwrap();
            let ids = devices.iter().map(|d| d.id.clone()).collect::<HashSet<_>>();
            let hardware = devices
                .iter()
                .find(|d| d.id == actual_id)
                .map(|d| d.hardware_id.clone());
            (ids, hardware)
        };
        if !should_dispatch(
            &actual_id,
            &selected,
            &connected,
            self.include_collections(),
            actual_hardware.as_deref(),
        ) {
            // Never fall back to another keyboard, even if it reports the same
            // physical key.
            return;
        }

        if down {
            if !self
                .pressed
                .lock()
                .unwrap()
                .insert((actual_id.clone(), key_id.clone()))
            {
                return; // auto-repeat
            }
            let _ = self.app.emit(
                "macro-key-pressed",
                json!({ "deviceId": actual_id, "keyId": key_id, "keyLabel": key.label() }),
            );
        } else {
            self.pressed
                .lock()
                .unwrap()
                .remove(&(actual_id.clone(), key_id.clone()));
            let _ = self.app.emit(
                "macro-key-released",
                json!({ "deviceId": actual_id, "keyId": key_id, "keyLabel": key.label() }),
            );
        }

        let _ = self.tx.send(DeviceKeyEvent {
            device_id: actual_id,
            key_id,
            down,
        });
    }
}

/// Pure isolation check used by the Raw Input handler and unit tests.
fn should_dispatch(
    actual_id: &str,
    selected: &KeyboardDevice,
    connected: &HashSet<String>,
    include_collections: bool,
    actual_hardware_id: Option<&str>,
) -> bool {
    if !selected.connected || !connected.contains(actual_id) {
        return false;
    }
    if actual_id == selected.id {
        return true;
    }
    // Opt-in only: a sibling HID collection is the same physical keypad, but a
    // second keyboard with an identical VID/PID must still be rejected unless
    // the user explicitly enabled this.
    include_collections
        && !selected.hardware_id.is_empty()
        && actual_hardware_id == Some(selected.hardware_id.as_str())
}

fn normalize_id(path: &str) -> String {
    path.to_ascii_uppercase()
}

fn make_device(path: String) -> KeyboardDevice {
    let hardware_id = parse_hardware_id(&path);
    let id = normalize_id(&path);
    let name = if hardware_id.is_empty() {
        path.clone()
    } else {
        hardware_id.clone()
    };
    KeyboardDevice {
        id,
        name,
        path,
        hardware_id,
        connected: true,
        selected: false,
    }
}

fn parse_hardware_id(path: &str) -> String {
    let upper = path.to_ascii_uppercase();
    let start = upper.find("VID_");
    let Some(start) = start else { return String::new() };
    let end = upper[start..]
        .find('#')
        .map(|i| start + i)
        .unwrap_or(upper.len());
    upper[start..end].trim_end_matches('\\').to_string()
}

unsafe fn device_path(handle: HANDLE) -> Option<String> {
    let mut size = 0u32;
    GetRawInputDeviceInfoW(Some(handle), RIDI_DEVICENAME, None, &mut size);
    if size == 0 {
        return None;
    }
    let mut buffer = vec![0u16; size as usize + 1];
    let written = GetRawInputDeviceInfoW(
        Some(handle),
        RIDI_DEVICENAME,
        Some(buffer.as_mut_ptr() as *mut c_void),
        &mut size,
    );
    if written == u32::MAX {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer[..written as usize]);
    Some(text.trim_end_matches('\0').to_string())
}

unsafe extern "system" fn raw_window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(manager) = MANAGER.get() else {
        return DefWindowProcW(hwnd, message, wparam, lparam);
    };

    match message {
        WM_INPUT => {
            // RIM_INPUT == 0. Ignore RIM_INPUTSINK (1) echoes.
            if (wparam.0 as u32 & 0xFF) == 0 {
                let mut size = 0u32;
                let header_size = std::mem::size_of::<RAWINPUTHEADER>() as u32;
                GetRawInputData(
                    HRAWINPUT(lparam.0 as *mut c_void),
                    RID_INPUT,
                    None,
                    &mut size,
                    header_size,
                );
                let needed =
                    (std::mem::size_of::<RAWINPUTHEADER>() + std::mem::size_of::<RAWKEYBOARD>()) as u32;
                if size >= needed && size < 4096 {
                    let mut buffer = vec![0u8; size as usize];
                    let written = GetRawInputData(
                        HRAWINPUT(lparam.0 as *mut c_void),
                        RID_INPUT,
                        Some(buffer.as_mut_ptr() as *mut c_void),
                        &mut size,
                        header_size,
                    );
                    if written == size {
                        let mut input = RAWINPUT::default();
                        let copy_len = (size as usize).min(std::mem::size_of::<RAWINPUT>());
                        std::ptr::copy_nonoverlapping(
                            buffer.as_ptr(),
                            &mut input as *mut RAWINPUT as *mut u8,
                            copy_len,
                        );
                        if input.header.dwType == RIM_TYPEKEYBOARD.0 {
                            let device = input.header.hDevice;
                            let keyboard = input.data.keyboard;
                            manager.on_input(device, &keyboard);
                        }
                    }
                }
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        WM_INPUT_DEVICE_CHANGE => {
            match wparam.0 as u32 {
                GIDC_ARRIVAL | GIDC_REMOVAL => {
                    manager.refresh();
                    manager.emit_status();
                }
                _ => {}
            }
            DefWindowProcW(hwnd, message, wparam, lparam)
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

fn raw_input_thread(ready: Sender<Result<u32, String>>) {
    unsafe {
        let class_name = w!("AYMAN_MACRO_DECK_RAW_INPUT");
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let class = WNDCLASSW {
            style: Default::default(),
            lpfnWndProc: Some(raw_window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: HINSTANCE(instance.0),
            hIcon: Default::default(),
            hCursor: Default::default(),
            hbrBackground: std::mem::zeroed(),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name,
        };
        let atom = RegisterClassW(&class);
        if atom == 0 {
            let _ = ready.send(Err("RegisterClassW failed for Raw Input window".into()));
            return;
        }

        let Ok(hwnd) = CreateWindowExW(
            Default::default(),
            class_name,
            w!("AYMAN MACRO DECK Raw Input"),
            Default::default(),
            0,
            0,
            0,
            0,
            Some(HWND_MESSAGE),
            None,
            None,
            None,
        ) else {
            let _ = UnregisterClassW(class_name, None);
            let _ = ready.send(Err("cannot create Raw Input message window".into()));
            return;
        };

        let device = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            // INPUTSINK receives keys even while the app is in the tray.
            // DEVNOTIFY reports arrival/removal. NOLEGACY is intentionally
            // absent: it would suppress the main keyboard too.
            dwFlags: RIDEV_INPUTSINK | RIDEV_DEVNOTIFY,
            hwndTarget: hwnd,
        };
        if RegisterRawInputDevices(&[device], std::mem::size_of::<RAWINPUTDEVICE>() as u32).is_err() {
            let _ = DestroyWindow(hwnd);
            let _ = UnregisterClassW(class_name, None);
            let _ = ready.send(Err("RegisterRawInputDevices failed".into()));
            return;
        }

        if let Some(manager) = MANAGER.get() {
            *manager.hwnd.lock().unwrap() = Some(hwnd.0 as isize);
        }
        let _ = ready.send(Ok(current_thread_id()));

        let mut message = std::mem::zeroed();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0);
            if result.0 <= 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }

        let remove = RAWINPUTDEVICE {
            usUsagePage: HID_USAGE_PAGE_GENERIC,
            usUsage: HID_USAGE_GENERIC_KEYBOARD,
            dwFlags: RIDEV_REMOVE,
            hwndTarget: HWND(std::ptr::null_mut()),
        };
        let _ = RegisterRawInputDevices(&[remove], std::mem::size_of::<RAWINPUTDEVICE>() as u32);
        let _ = DestroyWindow(hwnd);
        let _ = UnregisterClassW(class_name, None);
        crate::logging::info("Raw Input window destroyed");
    }
}

fn current_thread_id() -> u32 {
    unsafe { windows::Win32::System::Threading::GetCurrentThreadId() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(id: &str, connected: bool) -> KeyboardDevice {
        KeyboardDevice {
            id: id.to_string(),
            name: id.to_string(),
            path: id.to_string(),
            hardware_id: "VID_1234&PID_5678".into(),
            connected,
            selected: true,
        }
    }

    #[test]
    fn hardware_ids_are_parsed_from_real_device_paths() {
        let path = r"\\?\HID#VID_046D&PID_C52B&MI_00#7&1A2B3C4D&0&0000#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}";
        let device = make_device(path.to_string());
        assert_eq!(device.hardware_id, "VID_046D&PID_C52B&MI_00");
        assert_eq!(device.id, path.to_ascii_uppercase());
        assert!(!device.selected);
    }

    #[test]
    fn device_without_vid_pid_keeps_empty_hardware_id() {
        let device = make_device(r"\\?\ACPI#PNP0C0D#2&DA1AFF34&0#{884b96c3-56ef-11d1-bc8c-00a0c91405dd}".into());
        assert_eq!(device.hardware_id, "");
        assert!(!device.name.is_empty());
    }

    #[test]
    fn keys_from_other_keyboards_are_dropped() {
        let connected: HashSet<String> = ["KEYPAD", "MAIN"].iter().map(|s| s.to_string()).collect();
        let selected = device("KEYPAD", true);
        assert!(should_dispatch("KEYPAD", &selected, &connected, false, None));
        assert!(!should_dispatch("MAIN", &selected, &connected, false, None));
        assert!(!should_dispatch("UNKNOWN", &selected, &connected, false, None));
    }

    #[test]
    fn disconnected_selected_device_drops_all_keys() {
        let connected: HashSet<String> = ["MAIN"].iter().map(|s| s.to_string()).collect();
        let selected = device("KEYPAD", false);
        assert!(!should_dispatch("KEYPAD", &selected, &connected, false, None));
    }

    #[test]
    fn sibling_collections_are_opt_in_only() {
        let connected: HashSet<String> = ["KEYPAD", "KEYPAD-2", "MAIN"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let selected = device("KEYPAD", true);
        let same_hardware = Some("VID_1234&PID_5678");

        // Exact path always works.
        assert!(should_dispatch("KEYPAD", &selected, &connected, false, None));
        // Sibling collection is rejected by default, accepted when opted in.
        assert!(!should_dispatch(
            "KEYPAD-2",
            &selected,
            &connected,
            false,
            same_hardware
        ));
        assert!(should_dispatch(
            "KEYPAD-2",
            &selected,
            &connected,
            true,
            same_hardware
        ));
        // Different hardware is never accepted, even when opted in.
        assert!(!should_dispatch(
            "MAIN",
            &selected,
            &connected,
            true,
            Some("VID_9999&PID_0000")
        ));
    }
}
