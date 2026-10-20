//! Local artistic/audio policy. No sound device, UI, recommendation or network dependency.
use crate::scene::{Conditions, Phase};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Minimal,
    Ambient,
    Silence,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Minimal => "Minimalista",
            Self::Ambient => "Ambiental",
            Self::Silence => "Silencio",
        }
    }
    pub fn step(self, forward: bool) -> Self {
        let n = match self {
            Self::Minimal => 0,
            Self::Ambient => 1,
            Self::Silence => 2,
        };
        [Self::Minimal, Self::Ambient, Self::Silence][(n + if forward { 1 } else { 2 }) % 3]
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Preferences {
    pub mode: Mode,
    pub master: u8,
    pub music: u8,
    pub ambience: u8,
    pub interface: u8,
    pub interface_enabled: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            mode: Mode::Minimal,
            master: 40,
            music: 35,
            ambience: 14,
            interface: 22,
            interface_enabled: false,
        }
    }
}
impl Preferences {
    pub fn validate(&self) -> Result<(), String> {
        if [self.master, self.music, self.ambience, self.interface]
            .into_iter()
            .any(|v| v > 100)
        {
            Err("Audio volume must be 0..100".into())
        } else {
            Ok(())
        }
    }
    pub fn music_gain(&self) -> f32 {
        if self.mode == Mode::Silence {
            0.0
        } else {
            f32::from(self.master) * f32::from(self.music) / 10000.0
        }
    }
    pub fn ambient_gain(&self) -> f32 {
        if self.mode == Mode::Silence {
            0.0
        } else {
            f32::from(self.master) * f32::from(self.ambience) / 10000.0
        }
    }
    pub fn ui_gain(&self) -> f32 {
        if self.interface_enabled {
            f32::from(self.master) * f32::from(self.interface) / 10000.0
        } else {
            0.0
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Context {
    pub phase: Phase,
    pub weather: Conditions,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            phase: Phase::Noon,
            weather: Conditions::Unknown,
        }
    }
}
pub fn lead_seconds(mode: Mode) -> u64 {
    match mode {
        Mode::Minimal => 18,
        Mode::Ambient => 5,
        Mode::Silence => 3600,
    }
}
pub fn silence_seconds(mode: Mode, phase: Phase, phrase: usize) -> u64 {
    let base = [42, 44, 50, 58, 76, 96][phase.index() as usize];
    let gap = if mode == Mode::Ambient {
        base / 3
    } else {
        base
    };
    gap + [0, 11, 5, 17][phrase % 4]
}
pub fn phase_gain(phase: Phase) -> f32 {
    [0.95, 1.0, 0.9, 0.8, 0.6, 0.45][phase.index() as usize]
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn silence_respects_independent_interface_opt_in() {
        let mut p = Preferences {
            mode: Mode::Silence,
            ..Default::default()
        };
        assert_eq!(p.music_gain(), 0.0);
        assert_eq!(p.ambient_gain(), 0.0);
        assert_eq!(p.ui_gain(), 0.0);
        p.interface_enabled = true;
        assert!(p.ui_gain() > 0.0);
        p.master = 0;
        assert_eq!(p.ui_gain(), 0.0);
    }
    #[test]
    fn invalid_volume_rejected() {
        let p = Preferences {
            music: 101,
            ..Default::default()
        };
        assert!(p.validate().is_err());
    }
    #[test]
    fn quiet_spacing_and_family_are_bounded() {
        for phase in [
            Phase::Morning,
            Phase::Noon,
            Phase::Afternoon,
            Phase::Dusk,
            Phase::Night,
            Phase::Late,
        ] {
            assert!(silence_seconds(Mode::Minimal, phase, 0) >= 42);
            assert!(silence_seconds(Mode::Ambient, phase, 0) >= 14);
            assert!(phase_gain(phase) > 0.0);
        }
    }
}
