//! Temporary LAN receiver lives outside the player's read-only ROM namespace.
//! Starting/stopping it never blocks the Slint input thread.
use qrcodegen::{QrCode, QrCodeEcc};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{atomic::{AtomicBool, Ordering}, mpsc, Arc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub enum Event { Ready(String), Failed, Closed }
pub struct Session { cancel: Arc<AtomicBool>, pub events: mpsc::Receiver<Event> }
impl Session {
    pub fn start(directory: PathBuf, profile: PathBuf, notify: impl Fn() + Send + 'static) -> Self {
        let (tx, events) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let stopping = cancel.clone();
        thread::spawn(move || {
            let send = |event| { let _ = tx.send(event); notify(); };
            if run(&directory, &profile, &stopping, &send).is_err() { send(Event::Failed); }
            send(Event::Closed);
        });
        Self { cancel, events }
    }
    pub fn stop(&self) { self.cancel.store(true, Ordering::Relaxed); }
}
impl Drop for Session { fn drop(&mut self) { self.stop(); } }

fn functional_session(raw: &str) -> Option<&str> {
    let mut matches = raw.lines().filter_map(|line| line.strip_prefix("0::"))
        .filter_map(|path| Path::new(path).file_name()?.to_str())
        .filter(|unit| unit.strip_prefix("emotional-ui-functional-")
            .and_then(|v| v.strip_suffix(".service"))
            .is_some_and(|v| v.len() == 32 && v.bytes().all(|b| b.is_ascii_hexdigit())));
    let unit = matches.next()?;
    if matches.next().is_some() { None } else { Some(unit) }
}

fn run(directory: &Path, profile: &Path, stopping: &AtomicBool, send: &impl Fn(Event)) -> Result<(), ()> {
    if !cfg!(target_os = "linux") { return Err(()); }
    let cgroup = fs::read_to_string("/proc/self/cgroup").map_err(|_| ())?;
    let session = functional_session(&cgroup).ok_or(())?;
    let uid = Command::new("/usr/bin/id").arg("-u").output().map_err(|_| ())?;
    let uid = std::str::from_utf8(&uid.stdout).map_err(|_| ())?.trim().parse::<u32>().map_err(|_| ())?;
    if uid == 0 { return Err(()); }
    let executable = std::env::current_exe().map_err(|_| ())?;
    let helper = executable.parent().ok_or(())?.join("rom_drop.py");
    let info = fs::symlink_metadata(&helper).map_err(|_| ())?;
    if !info.is_file() || info.file_type().is_symlink() || !profile.is_absolute() || !directory.is_absolute() { return Err(()); }
    let stat = fs::read_to_string("/proc/self/stat").map_err(|_| ())?;
    let start = stat.rsplit_once(')').ok_or(())?.1.split_whitespace().nth(19).ok_or(())?;
    if !start.bytes().all(|b| b.is_ascii_digit()) { return Err(()); }
    let nonce = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| ())?.as_nanos();
    let status = directory.join(format!("drop-{nonce}.json"));
    let stop = status.with_extension("stop");
    let home = std::env::var("HOME").map_err(|_| ())?;
    if home.contains(['\n', '\r', '\0']) { return Err(()); }
    let mut child = Command::new("/usr/bin/sudo")
        .args(["-n", "/usr/bin/systemd-run", "--quiet", "--wait", "--collect", "--service-type=exec"])
        .arg(format!("--unit=emotional-ui-drop-{}-{nonce}", std::process::id()))
        .arg(format!("--uid={uid}"))
        .arg(format!("--property=BindsTo={session}"))
        .arg(format!("--property=PartOf={session}"))
        .arg("--property=RuntimeMaxSec=1h")
        .arg(format!("--property=Environment=HOME={home}"))
        .arg("/usr/bin/python3").arg("-B").arg(helper)
        .arg("--status").arg(&status).arg("--profile").arg(profile)
        .arg("--parent-pid").arg(std::process::id().to_string()).arg("--parent-start").arg(start)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|_| ())?;
    let started = Instant::now();
    let mut ready = false;
    let mut marker = false;
    let mut failed = false;
    loop {
        if stopping.load(Ordering::Relaxed) && !marker {
            use std::os::unix::fs::OpenOptionsExt;
            let _ = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&stop);
            marker = true;
        }
        if !ready && !marker {
            if let Ok(meta) = fs::symlink_metadata(&status) {
                if meta.is_file() && !meta.file_type().is_symlink() && meta.len() <= 1024 {
                    if let Ok(data) = fs::read(&status) {
                        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&data) {
                            if let Some(url) = json.get("url").and_then(|v| v.as_str()).filter(|s| s.starts_with("http://") && s.len() < 256) {
                                send(Event::Ready(url.into())); ready = true;
                            }
                        }
                    }
                }
            }
            if !ready && started.elapsed() > Duration::from_secs(20) { failed = true; }
        }
        if failed && !marker {
            use std::os::unix::fs::OpenOptionsExt;
            let _ = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(&stop);
            marker = true;
        }
        match child.try_wait() {
            Ok(Some(status_code)) => {
                let _ = fs::remove_file(&status); let _ = fs::remove_file(&stop);
                return if failed || (!ready && !marker) || (!status_code.success() && !stopping.load(Ordering::Relaxed)) { Err(()) } else { Ok(()) };
            }
            Ok(None) => thread::sleep(Duration::from_millis(100)),
            Err(_) => return Err(()),
        }
    }
}

pub fn qr_image(value: &str) -> Result<Image, ()> {
    let qr = QrCode::encode_text(value, QrCodeEcc::Medium).map_err(|_| ())?;
    let count = qr.size() as u32;
    // Integral pixels and a four-module quiet zone make the native display scannable.
    let scale = (192 / (count + 8)).max(1);
    let size = 192;
    let margin = (size - (count + 8) * scale) / 2;
    let mut pixels = SharedPixelBuffer::<Rgba8Pixel>::new(size, size);
    for y in 0..size { for x in 0..size {
        let xx = if x < margin { -1 } else { ((x - margin) / scale) as i32 - 4 };
        let yy = if y < margin { -1 } else { ((y - margin) / scale) as i32 - 4 };
        let dark = xx >= 0 && yy >= 0 && xx < qr.size() && yy < qr.size() && qr.get_module(xx, yy);
        let color = if dark { 0 } else { 255 };
        pixels.make_mut_slice()[(y*size+x) as usize] = Rgba8Pixel { r: color, g: color, b: color, a: 255 };
    }}
    Ok(Image::from_rgba8(pixels))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_verified_functional_cgroup_can_start_a_receiver() {
        let unit = "emotional-ui-functional-0123456789abcdef0123456789abcdef.service";
        assert_eq!(functional_session(&format!("0::/system.slice/{unit}\n")), Some(unit));
        assert_eq!(functional_session("0::/user.slice/other.service\n"), None);
        assert_eq!(functional_session(&format!("0::/{unit}\n0::/{unit}\n")), None);
    }
    #[test]
    fn qr_has_integer_sized_quiet_zone_and_is_bounded() {
        let image = qr_image("http://192.168.1.2:12345/#abcdefgh1234567890").unwrap();
        let size = image.size();
        assert_eq!(size.width, size.height);
        assert_eq!(size.width, 192);
    }
}
