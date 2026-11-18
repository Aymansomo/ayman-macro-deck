//! Minimal file logger (no extra crates): writes to
//! `%APPDATA%\com.ayman.macrodeck\logs\app.log` and mirrors to the console in
//! debug builds.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 2 * 1024 * 1024;

static SINK: OnceLock<Mutex<Option<File>>> = OnceLock::new();

pub fn init(log_dir: PathBuf) {
    let _ = fs::create_dir_all(&log_dir);
    let path = log_dir.join("app.log");

    let file = OpenOptions::new().create(true).append(true).open(&path).ok();
    if let Some(file) = &file {
        if file.metadata().map(|m| m.len()).unwrap_or(0) > MAX_LOG_BYTES {
            let _ = file.set_len(0);
        }
    }

    let _ = SINK.set(Mutex::new(file));
    info(format!("--- log started ({}) ---", path.display()));
}

pub fn write(level: &str, message: impl AsRef<str>) {
    let line = format!("[{}] {:<5} {}\n", timestamp(), level, message.as_ref());

    if cfg!(debug_assertions) {
        print!("{}", line);
    }

    if let Some(sink) = SINK.get() {
        if let Ok(mut guard) = sink.lock() {
            if let Some(file) = guard.as_mut() {
                let _ = file.write_all(line.as_bytes());
                let _ = file.flush();
            }
        }
    }
}

pub fn info(message: impl AsRef<str>) {
    write("INFO", message);
}

pub fn warn(message: impl AsRef<str>) {
    write("WARN", message);
}

pub fn error(message: impl AsRef<str>) {
    write("ERROR", message);
}

fn timestamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Compact UTC "YYYY-MM-DD HH:MM:SS" from unix seconds (no chrono needed).
    let days = (secs / 86_400) as i64;
    let seconds_of_day = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year,
        month,
        day,
        seconds_of_day / 3600,
        (seconds_of_day % 3600) / 60,
        seconds_of_day % 60
    )
}

/// Howard Hinnant's days-to-civil conversion.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::civil_from_days;

    #[test]
    fn converts_known_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
    }
}
