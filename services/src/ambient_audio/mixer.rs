use super::assets::{Library, RATE};
use crate::feedback::{self, Cue};
use emotional_core::{
    audio::{self, Context, Mode, Preferences},
    scene::Conditions,
};
use std::sync::Arc;
pub const BLOCK: usize = 512;
pub const FADE_FRAMES: u64 = RATE as u64 * 120 / 1000;
struct Voice {
    samples: Arc<[i16]>,
    at: usize,
    weight: f32,
}
impl Voice {
    fn next(&mut self) -> f32 {
        let v = self.samples.get(self.at).copied().unwrap_or(0);
        self.at += 1;
        f32::from(v) * self.weight
    }
    fn done(&self) -> bool {
        self.at >= self.samples.len()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Foreground,
    FadingOut,
    Released,
}
pub struct Mixer {
    library: Library,
    pub preferences: Preferences,
    pub context: Context,
    pub stage: Stage,
    pub cursor: u64,
    pub phrases_started: u64,
    pub context_changes: u64,
    next_music: u64,
    next_ambient: u64,
    music_index: usize,
    music: Option<Voice>,
    ambient: Option<Voice>,
    cue: Option<Voice>,
    gains: [f32; 3],
    master: f32,
    lead_override: Option<u64>,
}
impl Mixer {
    pub fn new(library: Library, preferences: Preferences, lead_override: Option<u64>) -> Self {
        let next = lead_override.unwrap_or(audio::lead_seconds(preferences.mode) * u64::from(RATE));
        Self {
            library,
            preferences,
            context: Context::default(),
            stage: Stage::Foreground,
            cursor: 0,
            phrases_started: 0,
            context_changes: 0,
            next_music: next,
            next_ambient: u64::from(RATE) * 10,
            music_index: 0,
            music: None,
            ambient: None,
            cue: None,
            gains: [0.0; 3],
            master: 0.0,
            lead_override,
        }
    }
    pub fn preferences(&mut self, p: Preferences) {
        if self.preferences.mode != p.mode && self.music.is_none() {
            self.next_music = self.cursor
                + self
                    .lead_override
                    .unwrap_or(audio::lead_seconds(p.mode) * u64::from(RATE));
        }
        self.preferences = p;
    }
    pub fn context(&mut self, c: Context) {
        if self.context != c {
            self.context_changes += 1;
            self.context = c;
        }
    }
    pub fn cue(&mut self, cue: Cue) {
        if self.stage == Stage::Foreground && self.preferences.ui_gain() > 0.0 {
            let pcm = feedback::pcm(cue);
            self.cue = Some(Voice {
                samples: pcm
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|b| i16::from_le_bytes([b[0], b[1]]))
                    .collect::<Vec<_>>()
                    .into(),
                at: 0,
                weight: 1.0,
            });
        }
    }
    pub fn suspend(&mut self) {
        self.stage = if self.music.is_none() && self.ambient.is_none() && self.cue.is_none() {
            Stage::Released
        } else {
            Stage::FadingOut
        };
    }
    pub fn resume(&mut self, p: Preferences) {
        self.music = None;
        self.ambient = None;
        self.cue = None;
        self.master = 0.0;
        self.stage = Stage::Foreground;
        self.preferences = p;
        self.next_music = self.cursor
            + self
                .lead_override
                .unwrap_or(audio::lead_seconds(self.preferences.mode) * u64::from(RATE));
        self.next_ambient = self.cursor + u64::from(RATE) * 10;
    }
    fn ambient_id(&self) -> Option<&'static str> {
        match self.context.weather {
            Conditions::Rain => Some("rain"),
            Conditions::Cloud if self.preferences.mode == Mode::Ambient => Some("air"),
            _ => None,
        }
    }
    pub fn until_event(&self) -> u64 {
        if self.stage != Stage::Foreground
            || self.music.is_some()
            || self.ambient.is_some()
            || self.cue.is_some()
        {
            return 0;
        }
        let mut next = u64::MAX;
        if self.preferences.music_gain() > 0.0
            && ["umbral", "brasa", "rastro"]
                .iter()
                .any(|n| self.library.clips.contains_key(*n))
        {
            next = self.next_music.saturating_sub(self.cursor);
        }
        if self.preferences.ambient_gain() > 0.0
            && self
                .ambient_id()
                .is_some_and(|id| self.library.clips.contains_key(id))
        {
            next = next.min(self.next_ambient.saturating_sub(self.cursor));
        }
        next
    }
    pub fn skip_silence(&mut self, frames: u64) {
        self.cursor = self.cursor.saturating_add(frames);
    }
    pub fn render(&mut self, out: &mut [i16]) {
        if self.stage == Stage::Released {
            out.fill(0);
            return;
        }
        let music_allowed = self.preferences.music_gain() > 0.0;
        if self.stage == Stage::Foreground
            && self.music.is_none()
            && music_allowed
            && self.cursor >= self.next_music
        {
            let ids = ["umbral", "brasa", "rastro"];
            let id = ids[self.music_index % 3];
            self.music_index += 1;
            if let Some(samples) = self.library.clips.get(id) {
                self.music = Some(Voice {
                    samples: samples.clone(),
                    at: 0,
                    weight: audio::phase_gain(self.context.phase),
                });
                self.phrases_started += 1;
            }
            self.next_music = self.cursor
                + audio::silence_seconds(
                    self.preferences.mode,
                    self.context.phase,
                    self.music_index,
                ) * u64::from(RATE);
        }
        if self.stage == Stage::Foreground
            && self.ambient.is_none()
            && self.cursor >= self.next_ambient
            && self.preferences.ambient_gain() > 0.0
        {
            if let Some(samples) = self.ambient_id().and_then(|id| self.library.clips.get(id)) {
                self.ambient = Some(Voice {
                    samples: samples.clone(),
                    at: 0,
                    weight: 1.0,
                });
            }
            self.next_ambient = self.cursor
                + u64::from(RATE)
                    * if self.preferences.mode == Mode::Ambient {
                        55
                    } else {
                        90
                    };
        }
        let ambient_target = if self.ambient_id().is_some() {
            self.preferences.ambient_gain()
        } else {
            0.0
        };
        let targets = [
            self.preferences.music_gain(),
            ambient_target,
            self.preferences.ui_gain(),
        ];
        for sample in out {
            let master_target = if self.stage == Stage::Foreground {
                1.0
            } else {
                0.0
            };
            self.master += (master_target - self.master)
                .clamp(-1.0 / FADE_FRAMES as f32, 1.0 / FADE_FRAMES as f32);
            for (gain, target) in self.gains.iter_mut().zip(targets) {
                *gain +=
                    (target - *gain).clamp(-1.0 / FADE_FRAMES as f32, 1.0 / FADE_FRAMES as f32);
            }
            let mut value = 0.0;
            for (voice, gain) in [&mut self.music, &mut self.ambient, &mut self.cue]
                .into_iter()
                .zip(self.gains)
            {
                if let Some(v) = voice {
                    value += v.next() * gain;
                }
            }
            *sample = (value * self.master).clamp(-30000.0, 30000.0) as i16;
            self.cursor += 1;
        }
        if self.music.as_ref().is_some_and(Voice::done) || self.gains[0] <= 0.0 {
            self.music = None;
        }
        if self.ambient.as_ref().is_some_and(Voice::done) || self.gains[1] <= 0.0 {
            self.ambient = None;
        }
        if self.cue.as_ref().is_some_and(Voice::done) || self.gains[2] <= 0.0 {
            self.cue = None;
        }
        if self.stage == Stage::FadingOut && self.master <= 0.0 {
            self.music = None;
            self.ambient = None;
            self.cue = None;
            self.stage = Stage::Released;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn mixer() -> Mixer {
        Mixer::new(Library::load(None), Preferences::default(), Some(0))
    }
    #[test]
    fn starts_softly_then_fades_before_release() {
        let mut m = mixer();
        let mut b = [0; BLOCK];
        m.render(&mut b);
        assert!(b.iter().any(|v| *v != 0));
        m.suspend();
        for _ in 0..7 {
            m.render(&mut b);
        }
        assert_eq!(m.stage, Stage::Released);
        assert_eq!(b, [0; BLOCK]);
    }
    #[test]
    fn context_does_not_restart_a_phrase() {
        let mut m = mixer();
        m.render(&mut [0; BLOCK]);
        let pos = m.music.as_ref().unwrap().at;
        let starts = m.phrases_started;
        m.context(Context {
            phase: emotional_core::scene::Phase::Late,
            weather: Conditions::Rain,
        });
        assert_eq!(m.music.as_ref().unwrap().at, pos);
        assert_eq!(m.phrases_started, starts);
    }
    #[test]
    fn silent_and_zero_volume_never_start() {
        for mut p in [
            Preferences {
                mode: Mode::Silence,
                ..Default::default()
            },
            Preferences {
                master: 0,
                ..Default::default()
            },
        ] {
            p.interface_enabled = false;
            let mut m = Mixer::new(Library::load(None), p, Some(0));
            let mut b = [1; BLOCK];
            for _ in 0..10 {
                m.render(&mut b);
                assert_eq!(b, [0; BLOCK]);
            }
            assert_eq!(m.phrases_started, 0);
        }
    }
    #[test]
    fn suspended_cues_and_changes_cannot_leak_into_game() {
        let mut m = mixer();
        m.suspend();
        let p = Preferences {
            interface_enabled: true,
            ..Default::default()
        };
        m.preferences(p);
        m.cue(Cue::Confirm);
        let mut b = [1; BLOCK];
        m.render(&mut b);
        assert_eq!(b, [0; BLOCK]);
        assert!(m.cue.is_none());
    }
    #[test]
    fn return_checks_latest_silence_preferences() {
        let mut m = mixer();
        m.suspend();
        m.resume(Preferences {
            mode: Mode::Silence,
            ..Default::default()
        });
        let mut b = [1; BLOCK];
        m.render(&mut b);
        assert_eq!(b, [0; BLOCK]);
    }
    #[test]
    fn missing_library_keeps_frontend_silent() {
        let mut m = Mixer::new(Library::default(), Preferences::default(), Some(0));
        assert_eq!(m.until_event(), u64::MAX);
        let mut b = [1; BLOCK];
        m.render(&mut b);
        assert_eq!(b, [0; BLOCK]);
    }
}
