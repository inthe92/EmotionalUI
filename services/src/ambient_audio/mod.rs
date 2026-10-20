//! AmbientAudioEngine: one mixer worker and one replaceable in-process PCM output.
//! UI calls only update a small mailbox; file/device work stays on this worker.
pub mod assets;
pub mod mixer;
pub mod output;
use crate::feedback::Cue;
use emotional_core::audio::{Context, Preferences};
use mixer::{Mixer, Stage, BLOCK};
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[derive(Clone, Debug)]
pub struct Options {
    pub backend: output::Backend,
    pub assets: Option<PathBuf>,
    pub lead_in_frames: Option<u64>,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            backend: output::Backend::Native,
            assets: None,
            lead_in_frames: None,
        }
    }
}
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub output_open: bool,
    pub released_ticket: Option<u64>,
    pub error: Option<String>,
    pub resource_bytes: usize,
    pub missing_assets: usize,
    pub blocks: u64,
    pub written_blocks: u64,
    pub nonzero_samples: u64,
    pub peak: i16,
    pub phrases: u64,
    pub opens: u64,
    pub closes: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Intent {
    Foreground,
    Suspend(u64),
    Stop,
}
struct Mailbox {
    preferences: Preferences,
    context: Context,
    intent: Intent,
    generation: u64,
    revision: u64,
    cues: VecDeque<Cue>,
}
struct Shared {
    mail: Mutex<Mailbox>,
    changed: Condvar,
    status: Mutex<Status>,
}
#[derive(Clone, Copy, Debug)]
pub struct ReleaseTicket(u64);
pub struct AmbientAudioEngine {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}
impl AmbientAudioEngine {
    pub fn start(preferences: Preferences, options: Options) -> Result<Self, String> {
        preferences.validate()?;
        let shared = Arc::new(Shared {
            mail: Mutex::new(Mailbox {
                preferences,
                context: Context::default(),
                intent: Intent::Foreground,
                generation: 0,
                revision: 0,
                cues: VecDeque::new(),
            }),
            changed: Condvar::new(),
            status: Mutex::new(Status::default()),
        });
        let worker = shared.clone();
        let thread = thread::Builder::new()
            .name("emotional-ambient-audio".into())
            .spawn(move || run(worker, options))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
    pub fn preferences(&self, p: Preferences) -> Result<(), String> {
        p.validate()?;
        let mut mail = self.shared.mail.lock().unwrap();
        if mail.preferences != p {
            mail.preferences = p;
            mail.revision += 1;
            self.shared.changed.notify_one();
        }
        Ok(())
    }
    pub fn context(&self, c: Context) {
        let mut mail = self.shared.mail.lock().unwrap();
        if mail.context != c {
            mail.context = c;
            mail.revision += 1;
            self.shared.changed.notify_one();
        }
    }
    pub fn cue(&self, c: Cue) {
        let mut mail = self.shared.mail.lock().unwrap();
        if mail.intent == Intent::Foreground
            && mail.preferences.ui_gain() > 0.0
            && mail.cues.len() < 4
        {
            mail.cues.push_back(c);
            self.shared.changed.notify_one();
        }
    }
    pub fn suspend_for_game(&self) -> ReleaseTicket {
        let mut mail = self.shared.mail.lock().unwrap();
        mail.generation += 1;
        let id = mail.generation;
        mail.intent = Intent::Suspend(id);
        mail.cues.clear();
        self.shared.status.lock().unwrap().released_ticket = None;
        self.shared.changed.notify_one();
        ReleaseTicket(id)
    }
    pub fn released(&self, ticket: ReleaseTicket) -> bool {
        let mail = self.shared.mail.lock().unwrap();
        let status = self.shared.status.lock().unwrap();
        mail.intent == Intent::Suspend(ticket.0)
            && status.released_ticket == Some(ticket.0)
            && !status.output_open
    }
    pub fn return_from_game(&self, latest: Preferences) -> Result<(), String> {
        latest.validate()?;
        let mut mail = self.shared.mail.lock().unwrap();
        mail.preferences = latest;
        mail.intent = Intent::Foreground;
        mail.generation += 1;
        mail.revision += 1;
        self.shared.status.lock().unwrap().released_ticket = None;
        self.shared.changed.notify_one();
        Ok(())
    }
    pub fn status(&self) -> Status {
        self.shared.status.lock().unwrap().clone()
    }
    pub fn finish(&mut self) -> Result<(), String> {
        if let Some(thread) = self.thread.take() {
            {
                let mut mail = self.shared.mail.lock().unwrap();
                mail.intent = Intent::Stop;
                mail.cues.clear();
                self.shared.changed.notify_one();
            }
            thread
                .join()
                .map_err(|_| "Audio worker failed".to_owned())?;
        }
        Ok(())
    }
}
impl Drop for AmbientAudioEngine {
    fn drop(&mut self) {
        let _ = self.finish();
    }
}
fn close_output(output: &mut Option<Box<dyn output::Output>>, shared: &Shared) {
    if let Some(mut sink) = output.take() {
        sink.drain();
        drop(sink);
        let mut s = shared.status.lock().unwrap();
        s.output_open = false;
        s.closes += 1;
    }
}
fn run(shared: Arc<Shared>, options: Options) {
    let library = assets::Library::load(options.assets.as_deref());
    {
        let mut s = shared.status.lock().unwrap();
        s.resource_bytes = library.bytes();
        s.missing_assets = library.missing.len();
    }
    let prefs = shared.mail.lock().unwrap().preferences.clone();
    let mut mixer = Mixer::new(library, prefs, options.lead_in_frames);
    let mut output: Option<Box<dyn output::Output>> = None;
    let mut current_intent = Intent::Foreground;
    let mut last_revision = 0;
    let mut failed = false;
    let mut last_cue = None;
    let mut buffer = [0i16; BLOCK];
    loop {
        let (preferences, context, intent, revision, cues) = {
            let mut m = shared.mail.lock().unwrap();
            (
                m.preferences.clone(),
                m.context,
                m.intent,
                m.revision,
                m.cues.drain(..).collect::<Vec<_>>(),
            )
        };
        if revision != last_revision {
            if failed {
                mixer.resume(preferences.clone());
            }
            failed = false;
            last_revision = revision;
        }
        mixer.preferences(preferences.clone());
        mixer.context(context);
        if intent != current_intent {
            match intent {
                Intent::Foreground => {
                    mixer.resume(preferences);
                    failed = false;
                }
                Intent::Suspend(_) | Intent::Stop => mixer.suspend(),
            }
            current_intent = intent;
        }
        for cue in cues {
            let now = Instant::now();
            if cue != Cue::Focus
                || last_cue
                    .is_none_or(|t: Instant| now.duration_since(t) >= Duration::from_millis(80))
            {
                mixer.cue(cue);
                last_cue = Some(now);
            }
        }
        if mixer.stage == Stage::Released {
            close_output(&mut output, &shared);
            if let Intent::Suspend(id) = intent {
                shared.status.lock().unwrap().released_ticket = Some(id);
            }
            if intent == Intent::Stop {
                break;
            }
            let mail = shared.mail.lock().unwrap();
            if mail.intent == intent && mail.revision == revision && mail.cues.is_empty() {
                drop(shared.changed.wait(mail).unwrap());
            }
            continue;
        }
        if failed && intent == Intent::Foreground {
            close_output(&mut output, &shared);
            let start = Instant::now();
            let mail = shared.mail.lock().unwrap();
            if mail.intent == intent && mail.revision == revision && mail.cues.is_empty() {
                drop(shared.changed.wait(mail).unwrap());
            }
            mixer.skip_silence((start.elapsed().as_secs_f64() * f64::from(assets::RATE)) as u64);
            continue;
        }
        let frames = mixer.until_event();
        if frames > 0 {
            close_output(&mut output, &shared);
            let start = Instant::now();
            let delay = if frames == u64::MAX {
                Duration::from_secs(3600)
            } else {
                Duration::from_secs_f64(frames as f64 / f64::from(assets::RATE))
            };
            let mail = shared.mail.lock().unwrap();
            if mail.intent == intent && mail.revision == revision && mail.cues.is_empty() {
                drop(shared.changed.wait_timeout(mail, delay).unwrap());
            }
            mixer.skip_silence((start.elapsed().as_secs_f64() * f64::from(assets::RATE)) as u64);
            continue;
        }
        mixer.render(&mut buffer);
        let audible = buffer.iter().any(|v| *v != 0);
        if output.is_none() && audible && !failed {
            match output::open(options.backend) {
                Ok(sink) => {
                    output = Some(sink);
                    let mut s = shared.status.lock().unwrap();
                    s.output_open = true;
                    s.opens += 1;
                    s.error = None;
                }
                Err(error) => {
                    shared.status.lock().unwrap().error = Some(error);
                    failed = true;
                }
            }
        }
        if let Some(sink) = &mut output {
            if let Err(error) = sink.write(&buffer) {
                shared.status.lock().unwrap().error = Some(error);
                failed = true;
                close_output(&mut output, &shared);
            } else {
                shared.status.lock().unwrap().written_blocks += 1;
            }
        } else {
            let mail = shared.mail.lock().unwrap();
            if mail.intent == intent && mail.revision == revision && mail.cues.is_empty() {
                drop(
                    shared
                        .changed
                        .wait_timeout(
                            mail,
                            Duration::from_secs_f64(BLOCK as f64 / f64::from(assets::RATE)),
                        )
                        .unwrap(),
                );
            }
        }
        let mut s = shared.status.lock().unwrap();
        s.blocks += 1;
        s.phrases = mixer.phrases_started;
        s.nonzero_samples += buffer.iter().filter(|v| **v != 0).count() as u64;
        s.peak = s
            .peak
            .max(buffer.iter().map(|v| v.saturating_abs()).max().unwrap_or(0));
    }
    close_output(&mut output, &shared);
}
#[cfg(test)]
mod tests {
    use super::*;
    use emotional_core::audio::Mode;
    fn wait(mut ready: impl FnMut() -> bool) {
        let start = Instant::now();
        while !ready() {
            assert!(
                start.elapsed() < Duration::from_secs(2),
                "audio test timeout"
            );
            thread::sleep(Duration::from_millis(4));
        }
    }
    fn engine(backend: output::Backend) -> AmbientAudioEngine {
        AmbientAudioEngine::start(
            Preferences::default(),
            Options {
                backend,
                lead_in_frames: Some(0),
                assets: None,
            },
        )
        .unwrap()
    }
    #[test]
    fn simulated_emulator_has_exclusive_audio_and_return_uses_latest_settings() {
        let e = engine(output::Backend::Probe);
        wait(|| e.status().output_open);
        let t = e.suspend_for_game();
        wait(|| e.released(t));
        assert!(!e.status().output_open);
        let blocks = e.status().blocks;
        thread::sleep(Duration::from_millis(90));
        assert_eq!(e.status().blocks, blocks);
        e.cue(Cue::Confirm);
        e.context(Context {
            phase: emotional_core::scene::Phase::Late,
            weather: emotional_core::scene::Conditions::Rain,
        });
        assert!(e.released(t));
        e.return_from_game(Preferences {
            mode: Mode::Silence,
            ..Default::default()
        })
        .unwrap();
        assert!(!e.released(t));
        thread::sleep(Duration::from_millis(60));
        assert!(!e.status().output_open);
    }
    #[test]
    fn device_failure_is_reported_without_killing_frontend() {
        let e = engine(output::Backend::Unavailable);
        wait(|| e.status().error.is_some());
        assert!(!e.status().output_open);
        let t = e.suspend_for_game();
        wait(|| e.released(t));
    }
    #[test]
    fn device_disconnect_closes_output_and_does_not_spin_retry() {
        let e = engine(output::Backend::WriteFailure);
        wait(|| e.status().error.is_some());
        wait(|| e.status().closes == 1);
        assert!(!e.status().output_open);
        thread::sleep(Duration::from_millis(40));
        let blocks = e.status().blocks;
        thread::sleep(Duration::from_millis(50));
        assert_eq!(e.status().blocks, blocks);
        assert_eq!(e.status().opens, 1);
        let ticket = e.suspend_for_game();
        wait(|| e.released(ticket));
    }
    #[test]
    fn stale_release_ticket_is_never_authorization_to_launch() {
        let e = engine(output::Backend::Probe);
        let a = e.suspend_for_game();
        let b = e.suspend_for_game();
        wait(|| e.released(b));
        assert!(!e.released(a));
        e.return_from_game(Preferences {
            mode: Mode::Silence,
            ..Default::default()
        })
        .unwrap();
        assert!(!e.released(b));
    }
    #[test]
    fn initial_silence_opens_no_output_and_shutdown_is_bounded() {
        let mut e = AmbientAudioEngine::start(
            Preferences {
                mode: Mode::Silence,
                ..Default::default()
            },
            Options {
                backend: output::Backend::Probe,
                ..Default::default()
            },
        )
        .unwrap();
        thread::sleep(Duration::from_millis(30));
        assert_eq!(e.status().opens, 0);
        let start = Instant::now();
        e.finish().unwrap();
        assert!(start.elapsed() < Duration::from_secs(1));
    }
}
