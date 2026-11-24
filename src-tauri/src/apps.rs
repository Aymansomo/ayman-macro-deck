//! Apps a profile starts, and where their window goes.
//!
//! A profile can launch applications the moment it becomes active and move each
//! app's main window to a chosen monitor — full screen, or one half of a split
//! screen (left/right, top/bottom). Windows are matched by executable name, so
//! an app that is already running is repositioned instead of started twice.
//!
//! Everything runs on a detached thread: switching profiles must never wait for
//! a game or a driver panel to open.

use std::collections::HashSet;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use serde::Serialize;
use windows::core::BOOL;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFO, MONITORINFOEXW,
};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Module32FirstW, Process32FirstW, Process32NextW, MODULEENTRY32W,
    PROCESSENTRY32W, TH32CS_SNAPMODULE, TH32CS_SNAPMODULE32, TH32CS_SNAPPROCESS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW,
    GetWindowThreadProcessId, IsWindowVisible, SetWindowPos, ShowWindow, GW_OWNER, HWND_TOP,
    SWP_SHOWWINDOW, SW_RESTORE,
};

use crate::models::ProfileApp;

/// How long to wait for a freshly started app to create its main window.
const WINDOW_WAIT_STEPS: u32 = 60;
const WINDOW_WAIT_STEP: Duration = Duration::from_millis(200);
/// How often a placement is repeated so an app that restores its own size
/// (browsers, Electron apps) still ends up where the profile wants it.
const PLACE_TRIES: u32 = 24;
const PLACE_RETRY_DELAY: Duration = Duration::from_millis(120);
/// How many checks in a row must match before the placement is trusted; apps
/// that restore their saved bounds do it well after the first few attempts.
const PLACE_STEADY: u32 = 8;
/// Windows adds invisible borders, and apps round sizes, so an exact match is
/// never expected.
const PLACE_TOLERANCE: i32 = 24;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Monitor {
    /// Zero-based position in this list; what a profile app stores.
    pub index: u32,
    /// Device name without the `\\.\` prefix, e.g. `DISPLAY2`.
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub primary: bool,
}

unsafe extern "system" fn collect_monitor(
    monitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let list = &mut *(data.0 as *mut Vec<MONITORINFOEXW>);
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if GetMonitorInfoW(monitor, &mut info.monitorInfo as *mut MONITORINFO).as_bool() {
        list.push(info);
    }
    BOOL(1)
}

/// Connected monitors ordered left to right, top to bottom (work areas).
pub fn monitors() -> Vec<Monitor> {
    let mut raw: Vec<MONITORINFOEXW> = Vec::new();

    unsafe {
        let _ = EnumDisplayMonitors(
            None,
            None,
            Some(collect_monitor),
            LPARAM(&mut raw as *mut _ as isize),
        );
    }

    raw.sort_by_key(|info| (info.monitorInfo.rcMonitor.left, info.monitorInfo.rcMonitor.top));

    raw.iter()
        .enumerate()
        .map(|(index, info)| {
            let work = info.monitorInfo.rcWork;
            let end = info
                .szDevice
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(info.szDevice.len());
            let name = String::from_utf16_lossy(&info.szDevice[..end]);
            Monitor {
                index: index as u32,
                name: name.trim_start_matches("\\\\.\\").to_string(),
                x: work.left,
                y: work.top,
                width: work.right - work.left,
                height: work.bottom - work.top,
                // MONITORINFOF_PRIMARY is the only valid flag: bit 0.
                primary: info.monitorInfo.dwFlags & 1 != 0,
            }
        })
        .collect()
}

/// Rectangle for one placement inside a monitor's work area.
fn placement_rect(monitor: &Monitor, placement: &str) -> Option<(i32, i32, i32, i32)> {
    let half_width = monitor.width / 2;
    let half_height = monitor.height / 2;
    match placement {
        "full" => Some((monitor.x, monitor.y, monitor.width, monitor.height)),
        "left" => Some((monitor.x, monitor.y, half_width, monitor.height)),
        "right" => Some((
            monitor.x + half_width,
            monitor.y,
            monitor.width - half_width,
            monitor.height,
        )),
        "top" => Some((monitor.x, monitor.y, monitor.width, half_height)),
        "bottom" => Some((
            monitor.x,
            monitor.y + half_height,
            monitor.width,
            monitor.height - half_height,
        )),
        _ => None,
    }
}

fn process_ids(exe_name: &str) -> Vec<u32> {
    let mut ids = Vec::new();

    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return ids;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let end = entry
                    .szExeFile
                    .iter()
                    .position(|unit| *unit == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
                if name.eq_ignore_ascii_case(exe_name) {
                    ids.push(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }

        let _ = CloseHandle(snapshot);
    }

    ids
}

struct WindowSearch {
    process_ids: Vec<u32>,
    found: Option<HWND>,
    /// When false, only hidden windows are accepted (tray apps).
    want_visible: bool,
}

/// A window that belongs to a dialog/modal (it has an owner) is skipped: the
/// main window is the one without. `GetWindow(GW_OWNER)` returns NULL for a
/// normal window, and NULL is reported as an error, so that case means "no
/// owner" — not "unknown".
fn has_owner(hwnd: HWND) -> bool {
    match unsafe { GetWindow(hwnd, GW_OWNER) } {
        Ok(owner) => !owner.0.is_null(),
        Err(_) => false,
    }
}

unsafe extern "system" fn collect_window(hwnd: HWND, data: LPARAM) -> BOOL {
    let search = &mut *(data.0 as *mut WindowSearch);
    let owned = has_owner(hwnd);
    let visible = IsWindowVisible(hwnd).as_bool();
    if visible != search.want_visible || owned || GetWindowTextLengthW(hwnd) <= 0 {
        return BOOL(1);
    }

    let mut process_id = 0u32;
    GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));
    if search.process_ids.contains(&process_id) {
        search.found = Some(hwnd);
        return BOOL(0);
    }

    BOOL(1)
}

/// Top-level window of any of these processes; `want_visible` picks either the
/// normal window or one an app hid into the tray.
fn find_window(process_ids: &[u32], want_visible: bool) -> Option<HWND> {
    if process_ids.is_empty() {
        return None;
    }
    let mut search = WindowSearch {
        process_ids: process_ids.to_vec(),
        found: None,
        want_visible,
    };
    unsafe {
        let _ = EnumWindows(Some(collect_window), LPARAM(&mut search as *mut _ as isize));
    }
    search.found
}

fn main_window(process_ids: &[u32]) -> Option<HWND> {
    find_window(process_ids, true)
}

/// Window an already running app hid into the tray, so it can be brought back.
fn hidden_window(process_ids: &[u32]) -> Option<HWND> {
    find_window(process_ids, false)
}

/// An app that currently owns a visible, titled window.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunningApp {
    pub pid: u32,
    pub title: String,
    /// Full executable path, ready to store on a profile.
    pub path: String,
    pub name: String,
}

fn window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let length = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if length <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..length as usize])
}

fn module_path(process_id: u32) -> Option<String> {
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(
            TH32CS_SNAPMODULE | TH32CS_SNAPMODULE32,
            process_id,
        )
        .ok()?;
        let mut entry = MODULEENTRY32W {
            dwSize: std::mem::size_of::<MODULEENTRY32W>() as u32,
            ..Default::default()
        };
        let path = if Module32FirstW(snapshot, &mut entry).is_ok() {
            let end = entry
                .szExePath
                .iter()
                .position(|unit| *unit == 0)
                .unwrap_or(entry.szExePath.len());
            let text = String::from_utf16_lossy(&entry.szExePath[..end]);
            Some(text).filter(|text| !text.trim().is_empty())
        } else {
            None
        };
        let _ = CloseHandle(snapshot);
        path
    }
}

unsafe extern "system" fn collect_running_app(hwnd: HWND, data: LPARAM) -> BOOL {
    let found = &mut *(data.0 as *mut Vec<(u32, String)>);
    if IsWindowVisible(hwnd).as_bool() && !has_owner(hwnd) {
        let title = window_title(hwnd);
        if !title.is_empty() {
            let mut process_id = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id as *mut u32));
            if process_id != 0 {
                found.push((process_id, title));
            }
        }
    }
    BOOL(1)
}

/// Apps with a real window right now, so one can be added without knowing its
/// install path (tray apps like AMD Software included).
pub fn running_apps() -> Vec<RunningApp> {
    let mut windows: Vec<(u32, String)> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(collect_running_app),
            LPARAM(&mut windows as *mut _ as isize),
        );
    }

    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for (process_id, title) in windows {
        let Some(path) = module_path(process_id) else {
            continue;
        };
        if !seen.insert(path.to_ascii_lowercase()) {
            continue;
        }
        let name = Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        apps.push(RunningApp {
            pid: process_id,
            title,
            path,
            name,
        });
    }

    apps.sort_by_key(|app| app.name.to_ascii_lowercase());
    apps
}

/// How many normal windows are on screen right now, for failure logs.
fn on_screen_windows() -> usize {
    let mut windows: Vec<(u32, String)> = Vec::new();
    unsafe {
        let _ = EnumWindows(
            Some(collect_running_app),
            LPARAM(&mut windows as *mut _ as isize),
        );
    }
    windows.len()
}

fn all_process_ids() -> HashSet<u32> {
    let mut ids = HashSet::new();
    unsafe {
        let Ok(snapshot) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else {
            return ids;
        };
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                ids.insert(entry.th32ProcessID);
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = CloseHandle(snapshot);
    }
    ids
}

fn window_rect(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
    Some((
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    ))
}

/// Where a window ended up, and where the profile asked it to go.
struct Placed {
    /// True when the window settled on the requested rectangle.
    held: bool,
    rect: (i32, i32, i32, i32),
    target: (i32, i32, i32, i32),
}

impl Placed {
    fn rect_text(&self) -> String {
        size_text(self.rect)
    }

    fn target_text(&self) -> String {
        size_text(self.target)
    }
}

fn size_text((x, y, width, height): (i32, i32, i32, i32)) -> String {
    format!("{}×{} at ({}, {})", width, height, x, y)
}

/// Moves a window and keeps it there for a couple of seconds, so a browser or
/// Electron app that re-applies its own saved window bounds cannot undo the
/// placement. Reports the rectangle the window really ended up with.
fn place_window(hwnd: HWND, monitor: &Monitor, placement: &str) -> Option<Placed> {
    let (x, y, width, height) = placement_rect(monitor, placement)?;
    let target = (x, y, width, height);
    let mut rect = target;
    let mut steady = 0u32;

    for _ in 0..PLACE_TRIES {
        unsafe {
            // A maximized window ignores SetWindowPos, so always un-maximize and
            // bring the window to the front first.
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetWindowPos(hwnd, Some(HWND_TOP), x, y, width, height, SWP_SHOWWINDOW);
        }
        thread::sleep(PLACE_RETRY_DELAY);
        let Some(current) = window_rect(hwnd) else {
            return Some(Placed {
                held: true,
                rect,
                target,
            });
        };
        rect = current;
        let (rx, ry, rw, rh) = current;
        let matched = (rx - x).abs() <= PLACE_TOLERANCE
            && (ry - y).abs() <= PLACE_TOLERANCE
            && (rw - width).abs() <= PLACE_TOLERANCE
            && (rh - height).abs() <= PLACE_TOLERANCE;
        if !matched {
            steady = 0;
            continue;
        }
        steady += 1;
        if steady >= PLACE_STEADY {
            return Some(Placed {
                held: true,
                rect,
                target,
            });
        }
    }

    Some(Placed {
        held: false,
        rect,
        target,
    })
}

/// Processes that appeared after an app was started, so a launcher's real app
/// can be found without knowing its executable name.
fn fresh_ids(before: &HashSet<u32>) -> Vec<u32> {
    all_process_ids()
        .into_iter()
        .filter(|id| !before.contains(id))
        .collect()
}

/// Chrome/Brave/Edge app shortcuts start `<browser>_proxy.exe`, which hands the
/// request to the real browser and exits: the window belongs to another process.
fn is_launcher_name(exe_name: &str) -> bool {
    let lower = exe_name.to_ascii_lowercase();
    lower.ends_with("_proxy.exe") || lower == "explorer.exe"
}

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// True when only a file name was stored, e.g. `Discord.exe`.
fn is_bare_name(path: &Path) -> bool {
    path.parent()
        .map(|parent| parent.as_os_str().is_empty())
        .unwrap_or(true)
}

/// Full path of a running process with this executable name.
fn running_path(name: &str) -> Option<PathBuf> {
    process_ids(name)
        .into_iter()
        .find_map(module_path)
        .map(PathBuf::from)
}

/// The few shallow places a stored file name may live, checked directly
/// without walking the disk: `%LOCALAPPDATA%\Programs\<name>`, then the root of
/// each known folder.
fn shallow_candidates(name: &str) -> Vec<PathBuf> {
    const ROOTS: [&str; 4] = [
        "LOCALAPPDATA",
        "ProgramFiles",
        "ProgramFiles(x86)",
        "APPDATA",
    ];

    let mut candidates = Vec::new();
    for root in ROOTS {
        let Some(dir) = std::env::var_os(root).map(PathBuf::from) else {
            continue;
        };
        if root == "LOCALAPPDATA" {
            candidates.push(dir.join("Programs").join(name));
        }
        candidates.push(dir.join(name));
    }
    candidates
}

/// Finds an executable stored by file name only, cheaply: a running process with
/// that name, then a couple of shallow known folders. Never scans the disk.
fn locate_exe(name: &str) -> Option<PathBuf> {
    if let Some(found) = running_path(name) {
        return Some(found);
    }
    shallow_candidates(name)
        .into_iter()
        .find(|path| path.is_file())
}

/// Executable a `.lnk` shortcut points at, asked from the Windows shell.
fn shortcut_target(path: &Path) -> Option<String> {
    let escaped = path.display().to_string().replace('\'', "''");
    let script = format!(
        "(New-Object -ComObject WScript.Shell).CreateShortcut('{}').TargetPath",
        escaped
    );
    let output = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then_some(text)
}

/// How an app is started and which executable identifies its window.
struct LaunchTarget {
    /// What actually gets started (the stored path, or a resolved full path).
    path: PathBuf,
    /// Executable file name used to find the app's window.
    exe_name: String,
    /// True when the real process is unknown, so only newly started processes
    /// can be searched (batch files, documents, unresolved shortcuts).
    unknown_target: bool,
    /// True when only a file name was stored and it could not be found quickly.
    bare_unresolved: bool,
}

fn resolve_target(app: &ProfileApp) -> LaunchTarget {
    let stored = PathBuf::from(app.path.trim());
    let mut path = stored.clone();
    let mut exe_name = stored
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut unknown_target = false;
    let mut bare_unresolved = false;

    let is_shortcut = stored
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("lnk"))
        .unwrap_or(false);

    if is_shortcut {
        // A shortcut hides its executable: ask the shell, so an already running
        // app's window can be found and placed instead of guessing.
        match shortcut_target(&stored) {
            Some(target) => {
                if let Some(name) = Path::new(&target).file_name() {
                    exe_name = name.to_string_lossy().to_string();
                }
            }
            None => unknown_target = true,
        }
    } else if !exe_name.to_ascii_lowercase().ends_with(".exe") {
        unknown_target = true;
    } else if is_bare_name(&stored) {
        match locate_exe(&exe_name) {
            Some(full) => path = full,
            None => bare_unresolved = true,
        }
    }

    if is_launcher_name(&exe_name) {
        unknown_target = true;
    }

    LaunchTarget {
        path,
        exe_name,
        unknown_target,
        bare_unresolved,
    }
}

fn launch(path: &Path, args: &str) -> Result<(), String> {
    let mut command = if path
        .extension()
        .map(|ext| ext.eq_ignore_ascii_case("exe"))
        .unwrap_or(false)
    {
        Command::new(path)
    } else {
        // Shortcuts, batch files and documents go through the shell.
        let mut shell = Command::new("cmd");
        shell.arg("/c").arg("start").arg("").arg(path);
        shell
    };

    for arg in args.split_whitespace() {
        command.arg(arg);
    }
    if let Some(folder) = path.parent() {
        command.current_dir(folder);
    }

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("cannot start {}: {}", path.display(), error))
}

fn where_text(monitor: &Monitor, placement: &str) -> String {
    let side = match placement {
        "full" => "full screen",
        "left" => "left half",
        "right" => "right half",
        "top" => "top half",
        "bottom" => "bottom half",
        other => other,
    };
    format!("Monitor {} · {}", monitor.index + 1, side)
}

/// The monitor a profile app asked for, plus a note when that screen is gone.
struct MonitorChoice {
    monitor: Option<Monitor>,
    note: Option<String>,
}

/// Picks the remembered screen by device name first: names survive monitors
/// being rearranged or a different one being plugged in, while an index does
/// not. An index is only a fallback for configs saved before names were stored,
/// and a missing screen falls back to the main monitor with a note.
fn pick_monitor(name: Option<&str>, index: Option<u32>, all: &[Monitor]) -> MonitorChoice {
    if all.is_empty() {
        return MonitorChoice {
            monitor: None,
            note: None,
        };
    }
    let primary = all.iter().find(|item| item.primary).unwrap_or(&all[0]);

    if let Some(name) = name.map(str::trim).filter(|name| !name.is_empty()) {
        if let Some(found) = all.iter().find(|item| item.name.eq_ignore_ascii_case(name)) {
            return MonitorChoice {
                monitor: Some(found.clone()),
                note: None,
            };
        }
        return MonitorChoice {
            note: Some(format!(
                "{} is not connected, used Monitor {}",
                name,
                primary.index + 1
            )),
            monitor: Some(primary.clone()),
        };
    }

    match index {
        Some(index) => match all.iter().find(|item| item.index == index) {
            Some(found) => MonitorChoice {
                monitor: Some(found.clone()),
                note: None,
            },
            None => MonitorChoice {
                note: Some(format!(
                    "Monitor {} is not connected, used Monitor {}",
                    index + 1,
                    primary.index + 1
                )),
                monitor: Some(primary.clone()),
            },
        },
        None => MonitorChoice {
            monitor: None,
            note: None,
        },
    }
}

fn choose_monitor(app: &ProfileApp) -> MonitorChoice {
    let placement = app.placement.as_deref().unwrap_or("keep");
    if !placement_rect_ref(placement) {
        return MonitorChoice {
            monitor: None,
            note: None,
        };
    }
    pick_monitor(app.monitor_name.as_deref(), app.monitor, &monitors())
}

/// Places a window, logs what really happened and returns the editor's line:
/// which monitor and half, plus the window's real size and position.
fn report_placement(
    label: &str,
    verb: &str,
    window: HWND,
    exe_name: &str,
    monitor: &Monitor,
    placement: &str,
    note: Option<&str>,
) -> String {
    let placed = place_window(window, monitor, placement);
    let where_text = where_text(monitor, placement);
    let proof = match &placed {
        Some(placed) if placed.held => format!("window {}", placed.rect_text()),
        Some(placed) => format!(
            "window {} (wanted {}), the app keeps its own size",
            placed.rect_text(),
            placed.target_text()
        ),
        None => "window not moved".to_string(),
    };

    let mut line = format!("{}: {}, {}", label, verb, where_text);
    if let Some(note) = note {
        line.push_str(&format!(" ({})", note));
    }
    line.push_str(&format!(" — {}", proof));

    crate::logging::info(format!(
        "profile app: {} \"{}\" ({}) on {} [{}] — {}",
        verb,
        window_title(window),
        exe_name,
        where_text,
        monitor.name,
        proof
    ));
    line
}

/// Starts or repositions one app and returns a short result for the UI.
fn apply_one(app: &ProfileApp) -> String {
    let target = resolve_target(app);
    let label = if target.path.as_os_str().is_empty() {
        app.path.clone()
    } else {
        target
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| app.path.clone())
    };
    if target.bare_unresolved {
        crate::logging::warn(format!(
            "profile app: {} not found; add it again with the full path",
            app.path
        ));
        return format!(
            "{}: not found — use “Pick a running app…” to store the full path",
            label
        );
    }

    let placement = app.placement.as_deref().unwrap_or("keep");
    let choice = choose_monitor(app);
    let monitor = choice.monitor;
    let note = choice.note;

    let ids = process_ids(&target.exe_name);
    if ids.is_empty() {
        let before = all_process_ids();
        if let Err(error) = launch(&target.path, &app.args) {
            crate::logging::warn(format!("profile app: {}", error));
            return format!("{}: {}", label, error);
        }

        for _ in 0..WINDOW_WAIT_STEPS {
            thread::sleep(WINDOW_WAIT_STEP);
            // The window normally belongs to the executable we know, but a
            // shortcut can start a launcher (or an app-mode browser window in
            // an already running browser), so newly started processes count too.
            let named = if target.unknown_target {
                Vec::new()
            } else {
                process_ids(&target.exe_name)
            };
            let Some(window) =
                main_window(&named).or_else(|| main_window(&fresh_ids(&before)))
            else {
                continue;
            };
            let Some(monitor) = &monitor else {
                crate::logging::info(format!("profile app: started {}", app.path));
                return format!("{}: started", label);
            };
            return report_placement(
                &label,
                "started",
                window,
                &target.exe_name,
                monitor,
                placement,
                note.as_deref(),
            );
        }

        crate::logging::warn(format!(
            "profile app: {} did not open a window in time (exe {}, {} process(es), {} window(s) on screen)",
            app.path,
            target.exe_name,
            process_ids(&target.exe_name).len(),
            on_screen_windows()
        ));
        return format!("{}: started, but no window appeared to place", label);
    }

    // Already running: never start a second copy, only move its window. A tray
    // app keeps a hidden window, so restore that one when nothing is visible;
    // an app that was just started elsewhere may still be creating its window.
    let mut window = main_window(&ids).or_else(|| hidden_window(&ids));
    for _ in 0..WINDOW_WAIT_STEPS / 2 {
        if window.is_some() {
            break;
        }
        thread::sleep(WINDOW_WAIT_STEP);
        window = main_window(&ids).or_else(|| hidden_window(&ids));
    }
    let Some(window) = window else {
        crate::logging::warn(format!(
            "profile app: {} is running but has no window to place",
            app.path
        ));
        return format!("{}: running, but no window found to place", label);
    };
    let Some(monitor) = &monitor else {
        return format!("{}: already running", label);
    };

    report_placement(
        &label,
        "moved",
        window,
        &target.exe_name,
        monitor,
        placement,
        note.as_deref(),
    )
}

/// Cheap check that a placement string does something.
fn placement_rect_ref(placement: &str) -> bool {
    matches!(placement, "full" | "left" | "right" | "top" | "bottom")
}

fn enabled(apps: &[ProfileApp]) -> Vec<ProfileApp> {
    apps.iter()
        .filter(|app| !app.path.trim().is_empty())
        .cloned()
        .collect()
}

/// Starts (or repositions) every app configured on a profile in the background,
/// calling `report` with each app's result and whether it was the last one, so
/// the UI can show progress instead of waiting for the whole run.
pub fn apply_reporting(apps: &[ProfileApp], report: impl Fn(String, bool) + Send + 'static) {
    let apps = enabled(apps);
    if apps.is_empty() {
        return;
    }

    thread::spawn(move || {
        let total = apps.len();
        for (index, app) in apps.into_iter().enumerate() {
            let line = apply_one(&app);
            let last = index + 1 == total;
            report(line, last);
            if !last {
                thread::sleep(Duration::from_millis(200));
            }
        }
    });
}

/// Fire-and-forget run, used when a profile becomes active.
pub fn apply(apps: &[ProfileApp]) {
    apply_reporting(apps, |_, _| {});
}

#[cfg(test)]
mod tests {
    use super::*;

    fn monitor() -> Monitor {
        Monitor {
            index: 0,
            name: "DISPLAY1".into(),
            x: 100,
            y: 50,
            width: 1920,
            height: 1080,
            primary: true,
        }
    }

    #[test]
    fn halves_split_the_monitor_without_gaps() {
        let monitor = monitor();
        assert_eq!(
            placement_rect(&monitor, "left"),
            Some((100, 50, 960, 1080))
        );
        assert_eq!(
            placement_rect(&monitor, "right"),
            Some((1060, 50, 960, 1080))
        );
        assert_eq!(
            placement_rect(&monitor, "top"),
            Some((100, 50, 1920, 540))
        );
        assert_eq!(
            placement_rect(&monitor, "bottom"),
            Some((100, 590, 1920, 540))
        );
        assert_eq!(
            placement_rect(&monitor, "full"),
            Some((100, 50, 1920, 1080))
        );
    }

    #[test]
    fn keep_placement_moves_nothing() {
        assert_eq!(placement_rect(&monitor(), "keep"), None);
        assert!(!placement_rect_ref("keep"));
    }

    #[test]
    fn bare_exe_names_are_recognized() {
        assert!(is_bare_name(Path::new("Discord.exe")));
        assert!(!is_bare_name(Path::new(
            r"C:\Users\me\AppData\Local\Discord\Discord.exe"
        )));
    }

    #[test]
    fn browser_shortcut_launchers_are_unknown_targets() {
        assert!(is_launcher_name("brave_proxy.exe"));
        assert!(is_launcher_name("Chrome_Proxy.exe"));
        assert!(!is_launcher_name("brave.exe"));
        assert!(!is_launcher_name("Discord.exe"));
    }

    fn two_monitors() -> Vec<Monitor> {
        vec![
            Monitor {
                index: 0,
                name: "DISPLAY1".into(),
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
                primary: true,
            },
            Monitor {
                index: 1,
                name: "DISPLAY2".into(),
                x: 1920,
                y: 0,
                width: 1440,
                height: 900,
                primary: false,
            },
        ]
    }

    #[test]
    fn monitor_name_wins_over_a_stale_index() {
        let all = two_monitors();
        // Saved as the second screen, then the screens were rearranged.
        let choice = pick_monitor(Some("DISPLAY2"), Some(0), &all);
        let monitor = choice.monitor.expect("a monitor is chosen");
        assert_eq!(monitor.name, "DISPLAY2");
        assert!(choice.note.is_none(), "no note when the screen exists");
    }

    #[test]
    fn a_missing_monitor_falls_back_to_the_main_one() {
        let all = two_monitors();
        let choice = pick_monitor(Some("DISPLAY9"), Some(2), &all);
        let monitor = choice.monitor.expect("a monitor is chosen");
        assert_eq!(monitor.name, "DISPLAY1");
        assert_eq!(
            choice.note.as_deref(),
            Some("DISPLAY9 is not connected, used Monitor 1")
        );
    }

    #[test]
    fn an_index_still_works_without_a_name() {
        let all = two_monitors();
        let choice = pick_monitor(None, Some(1), &all);
        assert_eq!(choice.monitor.map(|m| m.name), Some("DISPLAY2".into()));
        assert!(choice.note.is_none());

        let gone = pick_monitor(None, Some(5), &all);
        assert_eq!(gone.monitor.map(|m| m.name), Some("DISPLAY1".into()));
        assert!(gone.note.is_some());
    }
}
