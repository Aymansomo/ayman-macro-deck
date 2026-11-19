//! One running copy per signed-in user.
//!
//! Two copies would install two low-level keyboard hooks and two Raw Input
//! listeners, so a bound key could fire its macro twice (or be swallowed by one
//! copy and dispatched by the other). A named mutex in the `Local\` namespace
//! is per logon session, so separate Windows users on the same PC do not block
//! each other.

use windows::core::PCWSTR;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, MB_ICONINFORMATION, MB_OK, MB_SETFOREGROUND,
};

const MUTEX_NAME: &str = "Local\\com.ayman.macrodeck.instance";

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Returns true when this process is the first copy. The mutex handle is
/// intentionally leaked: it must live until the process exits.
pub fn acquire() -> bool {
    let name = wide(MUTEX_NAME);
    unsafe {
        match CreateMutexW(None, true, PCWSTR::from_raw(name.as_ptr())) {
            Ok(handle) => {
                // HANDLE has no destructor, so keeping the value here keeps the
                // mutex owned for the whole process lifetime.
                let _ = handle;
                GetLastError() != ERROR_ALREADY_EXISTS
            }
            // Never block startup just because the guard failed.
            Err(_) => true,
        }
    }
}

/// Tells the user where the running copy is, so the silent exit is explained.
pub fn warn_already_running() {
    let text = wide(
        "AYMAN MACRO DECK is already running.\n\nLook for its icon in the system tray, near the clock.",
    );
    let caption = wide("AYMAN MACRO DECK");
    unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(text.as_ptr()),
            PCWSTR::from_raw(caption.as_ptr()),
            MB_OK | MB_ICONINFORMATION | MB_SETFOREGROUND,
        );
    }
}
