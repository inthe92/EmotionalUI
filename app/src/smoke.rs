//! Optional, exclusive-create smoke journal. No claim that LCD presentation occurred.
use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex, OnceLock,
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};
static JOURNAL: OnceLock<Mutex<(File, Instant)>> = OnceLock::new();
static FAILED: AtomicBool = AtomicBool::new(false);
#[derive(Serialize)]
struct Event<'a> {
    event: &'a str,
    unix_ms: u64,
    elapsed_ms: u64,
    pid: u32,
    target_os: &'a str,
    target_arch: &'a str,
    width: Option<u32>,
    height: Option<u32>,
    detail: Option<&'a str>,
}
pub fn enable(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = OpenOptions::new().write(true).create_new(true).open(path)?;
    JOURNAL
        .set(Mutex::new((file, Instant::now())))
        .map_err(|_| "Smoke journal already open")?;
    emit(
        "process_start",
        None,
        None,
        Some("LCD presentation unverified"),
    );
    Ok(())
}
pub fn emit(event: &str, width: Option<u32>, height: Option<u32>, detail: Option<&str>) {
    if let Some(journal) = JOURNAL.get() {
        match journal.lock() {
            Ok(mut guard) => {
                let row = Event {
                    event,
                    unix_ms: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                    elapsed_ms: guard.1.elapsed().as_millis() as u64,
                    pid: std::process::id(),
                    target_os: std::env::consts::OS,
                    target_arch: std::env::consts::ARCH,
                    width,
                    height,
                    detail,
                };
                let result = toml::to_string(&row)
                    .map_err(std::io::Error::other)
                    .and_then(|s| writeln!(guard.0, "[[events]]\n{s}"));
                if let Err(error) = result {
                    FAILED.store(true, Ordering::Relaxed);
                    eprintln!("Smoke journal failed: {error}");
                }
            }
            Err(_) => {
                FAILED.store(true, Ordering::Relaxed);
            }
        }
    }
}
pub fn failed() -> bool {
    FAILED.load(Ordering::Relaxed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_journal_is_parseable() {
        let row = Event {
            event: "failure",
            unix_ms: 1,
            elapsed_ms: 2,
            pid: 3,
            target_os: "linux",
            target_arch: "aarch64",
            width: Some(640),
            height: Some(480),
            detail: Some("quoted \"error\"\nline"),
        };
        let text = format!("[[events]]\n{}", toml::to_string(&row).unwrap());
        let parsed = text.parse::<toml::Value>().unwrap();
        assert_eq!(parsed["events"][0]["width"].as_integer(), Some(640));
        assert_eq!(parsed["events"][0]["detail"].as_str(), row.detail);
    }
}
