//! Read-only Linux evdev worker. Never grabs a device or writes input events.
use emotional_core::controls::RawEvent;
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    os::{
        fd::AsRawFd,
        unix::fs::{FileTypeExt, OpenOptionsExt},
    },
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, TryRecvError},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

pub struct Gamepad {
    rx: Receiver<RawEvent>,
    stop: Arc<AtomicBool>,
    fault: Arc<Mutex<Option<String>>>,
    worker: Option<JoinHandle<()>>,
    pub path: PathBuf,
}
fn device_name(file: &File) -> io::Result<String> {
    let mut name = [0u8; 256];
    // SAFETY: EVIOCGNAME writes at most the requested 256 bytes to this buffer.
    if unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            (2u64 << 30) | (256 << 16) | (u64::from(b'E') << 8) | 6,
            name.as_mut_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    Ok(String::from_utf8_lossy(name.split(|b| *b == 0).next().unwrap_or_default()).into())
}
fn open(path: &Path, name: &str) -> io::Result<File> {
    let path = path.canonicalize()?;
    if path.parent() != Some(Path::new("/dev/input"))
        || !path
            .file_name()
            .is_some_and(|s| s.to_string_lossy().starts_with("event"))
    {
        return Err(io::Error::other("Expected an evdev event device"));
    }
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(&path)?;
    if !f.metadata()?.file_type().is_char_device() || device_name(&f)? != name {
        return Err(io::Error::other(
            "Controller identity does not match profile",
        ));
    }
    Ok(f)
}
pub fn decode(bytes: &[u8]) -> io::Result<Vec<RawEvent>> {
    let offset = std::mem::size_of::<libc::timeval>();
    let size = offset + 8;
    if !bytes.len().is_multiple_of(size) {
        return Err(io::Error::other("Partial input_event record"));
    }
    Ok(bytes
        .chunks_exact(size)
        .map(|b| RawEvent {
            kind: u16::from_ne_bytes(b[offset..offset + 2].try_into().unwrap()),
            code: u16::from_ne_bytes(b[offset + 2..offset + 4].try_into().unwrap()),
            value: i32::from_ne_bytes(b[offset + 4..offset + 8].try_into().unwrap()),
        })
        .collect())
}
fn validate_mapping(file: &File, mapping: &emotional_core::controls::Mapping) -> io::Result<()> {
    mapping.validate().map_err(io::Error::other)?;
    let mut bits = [0u8; 96];
    // SAFETY: EVIOCGBIT(EV_KEY) writes at most 96 bytes to the supplied buffer.
    if unsafe {
        libc::ioctl(
            file.as_raw_fd(),
            (2u64 << 30) | (96 << 16) | (u64::from(b'E') << 8) | 0x21,
            bits.as_mut_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    for button in &mapping.buttons {
        if bits[usize::from(button.code) / 8] & (1 << (button.code % 8)) == 0 {
            return Err(io::Error::other(
                "Mapped button is absent on this controller",
            ));
        }
    }
    for axis in &mapping.axes {
        let mut info = [0i32; 6];
        // SAFETY: EVIOCGABS writes one initialized six-i32 input_absinfo structure.
        if unsafe {
            libc::ioctl(
                file.as_raw_fd(),
                (2u64 << 30) | (24 << 16) | (u64::from(b'E') << 8) | (0x40 + u64::from(axis.code)),
                info.as_mut_ptr(),
            )
        } < 0
        {
            return Err(io::Error::last_os_error());
        }
        if info[1] != axis.min || info[2] != axis.max {
            return Err(io::Error::other(
                "Axis range differs from the configured revision",
            ));
        }
    }
    Ok(())
}
impl Gamepad {
    pub fn open(
        expected_name: &str,
        path: Option<&Path>,
        mapping: &emotional_core::controls::Mapping,
    ) -> io::Result<Self> {
        if expected_name.is_empty() || expected_name.len() > 128 {
            return Err(io::Error::other("Invalid controller name"));
        }
        let (file, path) = if let Some(path) = path {
            (open(path, expected_name)?, path.to_owned())
        } else {
            let mut found = None;
            for i in 0..64 {
                let path = PathBuf::from(format!("/dev/input/event{i}"));
                if let Ok(file) = open(&path, expected_name) {
                    found = Some((file, path));
                    break;
                }
            }
            found.ok_or_else(|| io::Error::other("Controller not found or not readable"))?
        };
        validate_mapping(&file, mapping)?;
        let (tx, rx) = mpsc::sync_channel(128);
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let fault = Arc::new(Mutex::new(None));
        let error = fault.clone();
        let worker = thread::Builder::new()
            .name("emotional-input".into())
            .spawn(move || {
                let result = (|| -> io::Result<()> {
                    let mut file = file;
                    let mut bytes = vec![0u8; (std::mem::size_of::<libc::timeval>() + 8) * 64];
                    while !flag.load(Ordering::Relaxed) {
                        let mut fd = libc::pollfd {
                            fd: file.as_raw_fd(),
                            events: libc::POLLIN,
                            revents: 0,
                        };
                        // SAFETY: fd points to one initialized pollfd, valid for this call.
                        let n = unsafe { libc::poll(&mut fd, 1, 20) };
                        if n < 0 {
                            let e = io::Error::last_os_error();
                            if e.kind() == io::ErrorKind::Interrupted {
                                continue;
                            }
                            return Err(e);
                        }
                        if fd.revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
                            return Err(io::Error::other("Controller disconnected"));
                        }
                        if n == 0 {
                            continue;
                        }
                        match file.read(&mut bytes) {
                            Ok(0) => return Err(io::Error::other("Controller closed")),
                            Ok(n) => {
                                for event in decode(&bytes[..n])? {
                                    tx.try_send(event).map_err(|_| {
                                        io::Error::other(
                                            "Input queue overflow/closed; reset controls",
                                        )
                                    })?;
                                }
                            }
                            Err(e)
                                if matches!(
                                    e.kind(),
                                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                                ) => {}
                            Err(e) => return Err(e),
                        }
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    if let Ok(mut slot) = error.lock() {
                        *slot = Some(e.to_string());
                    }
                }
            })?;
        Ok(Self {
            rx,
            stop,
            fault,
            worker: Some(worker),
            path,
        })
    }
    pub fn drain(&self) -> Result<Vec<RawEvent>, String> {
        if let Some(error) = self
            .fault
            .lock()
            .map_err(|_| "Input worker failed")?
            .clone()
        {
            while self.rx.try_recv().is_ok() {}
            return Err(error);
        }
        let mut events = Vec::new();
        for _ in 0..128 {
            match self.rx.try_recv() {
                Ok(e) => events.push(e),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Err("Controller worker stopped".into()),
            }
        }
        Ok(events)
    }
}
impl Drop for Gamepad {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn event_abi_and_signed_axes() {
        let size = std::mem::size_of::<libc::timeval>();
        let mut bytes = vec![0u8; size + 8];
        bytes[size..size + 2].copy_from_slice(&3u16.to_ne_bytes());
        bytes[size + 4..size + 8].copy_from_slice(&(-1800i32).to_ne_bytes());
        let e = decode(&bytes).unwrap();
        assert_eq!(e[0].kind, 3);
        assert_eq!(e[0].value, -1800);
        assert!(decode(&bytes[..size + 7]).is_err());
    }
    #[test]
    fn refuses_regular_files() {
        assert!(Gamepad::open(
            "retrogame_joypad",
            Some(Path::new("/etc/os-release")),
            &emotional_core::controls::Mapping::default()
        )
        .is_err());
    }
}
