//! Optional original PCM cues. Bounded worker; no playback on the UI thread.
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    Focus,
    Confirm,
    Cancel,
    Panel,
}
pub fn pcm(cue: Cue) -> Vec<u8> {
    let (hz, ms) = match cue {
        Cue::Focus => (520., 28),
        Cue::Confirm => (660., 52),
        Cue::Cancel => (330., 40),
        Cue::Panel => (440., 48),
    };
    let count = 22050 * ms / 1000;
    let mut bytes = Vec::with_capacity(count * 2);
    for i in 0..count {
        let t = i as f64 / 22050.;
        let phase = std::f64::consts::TAU * hz * t;
        let envelope = (std::f64::consts::PI * i as f64 / (count - 1) as f64)
            .sin()
            .powi(2);
        let sample = ((phase.sin() + 0.12 * (phase * 2.).sin()) * envelope * 1500.) as i16;
        bytes.extend(sample.to_le_bytes());
    }
    bytes
}
pub struct Feedback {
    tx: Option<SyncSender<Cue>>,
    stop: Arc<AtomicBool>,
    enabled: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}
impl Feedback {
    pub fn new() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let enabled = Arc::new(AtomicBool::new(false));
        let audible = enabled.clone();
        if !cfg!(target_os = "linux") || !Path::new("/usr/bin/aplay").is_file() {
            return Self {
                tx: None,
                stop,
                enabled,
                worker: None,
            };
        }
        let (tx, rx) = mpsc::sync_channel(4);
        let worker = thread::Builder::new()
            .name("emotional-audio".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    let Ok(cue) = rx.recv_timeout(Duration::from_millis(20)) else {
                        continue;
                    };
                    if flag.load(Ordering::Relaxed) {
                        break;
                    }
                    if !audible.load(Ordering::Relaxed) {
                        continue;
                    }
                    let Ok(mut child) = Command::new("/usr/bin/aplay")
                        .args(["-q", "-t", "raw", "-f", "S16_LE", "-r", "22050", "-c", "1"])
                        .stdin(Stdio::piped())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                    else {
                        continue;
                    };
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = stdin.write_all(&pcm(cue));
                    }
                    let start = Instant::now();
                    loop {
                        if child.try_wait().ok().flatten().is_some() {
                            break;
                        }
                        if flag.load(Ordering::Relaxed)
                            || !audible.load(Ordering::Relaxed)
                            || start.elapsed() > Duration::from_millis(250)
                        {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        thread::sleep(Duration::from_millis(5));
                    }
                }
            })
            .ok();
        Self {
            tx: worker.as_ref().map(|_| tx),
            stop,
            enabled,
            worker,
        }
    }
    pub fn available(&self) -> bool {
        self.tx.is_some()
    }
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }
    pub fn play(&self, cue: Cue, enabled: bool) {
        self.set_enabled(enabled);
        if enabled {
            if let Some(tx) = &self.tx {
                let _ = tx.try_send(cue);
            }
        }
    }
}
impl Default for Feedback {
    fn default() -> Self {
        Self::new()
    }
}
impl Drop for Feedback {
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
    fn original_cues_are_short_and_faded() {
        for c in [Cue::Focus, Cue::Confirm, Cue::Cancel, Cue::Panel] {
            let b = pcm(c);
            assert!(b.len() < 2500);
            let samples = b
                .as_chunks::<2>()
                .0
                .iter()
                .map(|x| i16::from_le_bytes([x[0], x[1]]))
                .collect::<Vec<_>>();
            assert_eq!(samples[0], 0);
            assert_eq!(*samples.last().unwrap(), 0);
            assert!(samples.iter().all(|s| s.abs() < 1800));
            assert!(samples.iter().any(|s| *s != 0));
        }
    }
    #[test]
    fn cues_have_distinct_content() {
        assert_ne!(pcm(Cue::Focus), pcm(Cue::Confirm));
        assert_ne!(pcm(Cue::Cancel), pcm(Cue::Panel));
    }
}
