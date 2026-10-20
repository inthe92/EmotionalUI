//! Configurable raw-controller translation. No UI, OS, filesystem or Slint types.
use crate::input::RepeatGate;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Duration,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    Left,
    Right,
    Up,
    Down,
    Confirm,
    Back,
    Refresh,
    Library,
    Menu,
    Previous,
    Next,
    ReducedMotion,
    Sound,
    Details,
    PreviewPrevious,
    PreviewNext,
    MomentPrevious,
    MomentNext,
}
impl Action {
    pub fn navigation(self) -> bool {
        matches!(self, Self::Left | Self::Right | Self::Up | Self::Down)
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Up => "up",
            Self::Down => "down",
            Self::Confirm => "confirm",
            Self::Back => "back",
            Self::Refresh => "refresh",
            Self::Library => "library",
            Self::Menu => "menu",
            Self::Previous => "previous",
            Self::Next => "next",
            Self::ReducedMotion => "reduced_motion",
            Self::Sound => "sound",
            Self::Details => "details",
            Self::PreviewPrevious => "preview_previous",
            Self::PreviewNext => "preview_next",
            Self::MomentPrevious => "moment_previous",
            Self::MomentNext => "moment_next",
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Button {
    pub code: u16,
    pub action: Action,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Axis {
    pub code: u16,
    pub min: i32,
    pub max: i32,
    pub negative: Action,
    pub positive: Action,
    #[serde(default = "engage")]
    pub engage_percent: u8,
    #[serde(default = "release")]
    pub release_percent: u8,
}
fn engage() -> u8 {
    35
}
fn release() -> u8 {
    20
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mapping {
    #[serde(default)]
    pub buttons: Vec<Button>,
    #[serde(default)]
    pub axes: Vec<Axis>,
}
impl Mapping {
    pub fn validate(&self) -> Result<(), String> {
        if self.buttons.len() > 64 || self.axes.len() > 8 {
            return Err("Controller mapping is too large".into());
        }
        let mut codes = BTreeSet::new();
        for b in &self.buttons {
            if b.code > 0x2ff || !codes.insert(b.code) {
                return Err("Invalid/duplicate button code".into());
            }
        }
        codes.clear();
        for a in &self.axes {
            if a.code > 0x3f
                || !codes.insert(a.code)
                || a.min >= a.max
                || a.release_percent >= a.engage_percent
                || a.engage_percent > 95
                || !a.negative.navigation()
                || !a.positive.navigation()
                || a.negative == a.positive
            {
                return Err("Invalid/duplicate axis mapping".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct RawEvent {
    pub kind: u16,
    pub code: u16,
    pub value: i32,
}
/// One action can have multiple physical sources. Releasing one preserves the others.
pub struct Controller {
    mapping: Mapping,
    sources: BTreeMap<(u16, u16), Action>,
    active: BTreeSet<Action>,
    gate: RepeatGate,
    dropped: bool,
}
impl Controller {
    pub fn new(mapping: Mapping) -> Result<Self, String> {
        mapping.validate()?;
        Ok(Self {
            mapping,
            sources: BTreeMap::new(),
            active: BTreeSet::new(),
            gate: RepeatGate::default(),
            dropped: false,
        })
    }
    pub fn held(&self) -> bool {
        !self.active.is_empty()
    }
    pub fn disconnect(&mut self) {
        self.sources.clear();
        self.active.clear();
        self.gate.release();
    }
    pub fn event(&mut self, event: RawEvent, now: Duration) -> Vec<Action> {
        if event.kind == 0 && event.code == 3 {
            self.disconnect();
            self.dropped = true;
            return vec![];
        }
        if self.dropped {
            if event.kind == 0 && event.code == 0 {
                self.dropped = false;
            }
            return vec![];
        }
        let source = (event.kind, event.code);
        let next = if event.kind == 1 {
            if !matches!(event.value, 0 | 1) {
                return vec![];
            } // EV_KEY autorepeat is replaced by our bounded repeat.
            let Some(binding) = self.mapping.buttons.iter().find(|b| b.code == event.code) else {
                return vec![];
            };
            if event.value == 1 {
                Some(binding.action)
            } else {
                None
            }
        } else if event.kind == 3 {
            let Some(axis) = self.mapping.axes.iter().find(|a| a.code == event.code) else {
                return vec![];
            };
            if !(axis.min..=axis.max).contains(&event.value) {
                return vec![];
            }
            let span = i64::from(axis.max) - i64::from(axis.min);
            let centered = i64::from(event.value) * 2 - i64::from(axis.max) - i64::from(axis.min);
            let proposed = if centered < 0 {
                axis.negative
            } else {
                axis.positive
            };
            let threshold = if self.sources.get(&source) == Some(&proposed) {
                axis.release_percent
            } else {
                axis.engage_percent
            };
            if centered.abs() * 100 > span * i64::from(threshold) {
                Some(if centered < 0 {
                    axis.negative
                } else {
                    axis.positive
                })
            } else {
                None
            }
        } else {
            return vec![];
        };
        if let Some(action) = next {
            self.sources.insert(source, action);
        } else {
            self.sources.remove(&source);
        }
        let active = self.sources.values().copied().collect::<BTreeSet<_>>();
        for action in self.active.difference(&active) {
            self.gate.release_key(action.id());
        }
        let mut output = Vec::new();
        for action in active.difference(&self.active) {
            if self.gate.press(action.id(), now, action.navigation()) {
                output.push(*action)
            }
        }
        self.active = active;
        output
    }
    pub fn tick(&mut self, now: Duration) -> Vec<Action> {
        let active = &self.active;
        let gate = &mut self.gate;
        active
            .iter()
            .copied()
            .filter(|a| {
                let opposite = match a {
                    Action::Left => Some(Action::Right),
                    Action::Right => Some(Action::Left),
                    Action::Up => Some(Action::Down),
                    Action::Down => Some(Action::Up),
                    _ => None,
                };
                !opposite.is_some_and(|opposite| active.contains(&opposite))
                    && a.navigation()
                    && gate.press(a.id(), now, true)
            })
            .collect()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> Controller {
        Controller::new(Mapping {
            buttons: vec![
                Button {
                    code: 304,
                    action: Action::Confirm,
                },
                Button {
                    code: 305,
                    action: Action::Down,
                },
            ],
            axes: vec![Axis {
                code: 0,
                min: -1800,
                max: 1800,
                negative: Action::Left,
                positive: Action::Right,
                engage_percent: 35,
                release_percent: 20,
            }],
        })
        .unwrap()
    }
    fn key(code: u16, value: i32) -> RawEvent {
        RawEvent {
            kind: 1,
            code,
            value,
        }
    }
    #[test]
    fn duplicate_and_simultaneous_presses() {
        let mut c = setup();
        assert_eq!(c.event(key(304, 1), Duration::ZERO), vec![Action::Confirm]);
        c.event(key(305, 1), Duration::ZERO);
        assert!(c.event(key(304, 1), Duration::from_secs(1)).is_empty());
        assert!(c.event(key(304, 2), Duration::from_secs(2)).is_empty());
        c.event(key(304, 0), Duration::from_secs(2));
        assert_eq!(
            c.event(key(304, 1), Duration::from_secs(2)),
            vec![Action::Confirm]
        );
    }
    #[test]
    fn navigation_repeat_and_release() {
        let mut c = setup();
        c.event(key(305, 1), Duration::ZERO);
        assert!(c.tick(Duration::from_millis(319)).is_empty());
        assert_eq!(c.tick(Duration::from_millis(320)), vec![Action::Down]);
        assert!(c.tick(Duration::from_millis(400)).is_empty());
        c.event(key(305, 0), Duration::from_millis(401));
        assert!(!c.held());
        assert!(c.tick(Duration::from_secs(9)).is_empty());
    }
    #[test]
    fn axis_hysteresis_and_rapid_reversal() {
        let mut c = setup();
        let axis = |v| RawEvent {
            kind: 3,
            code: 0,
            value: v,
        };
        assert_eq!(c.event(axis(900), Duration::ZERO), vec![Action::Right]);
        assert!(c.event(axis(500), Duration::ZERO).is_empty());
        assert_eq!(c.event(axis(-900), Duration::ZERO), vec![Action::Left]);
        c.event(axis(0), Duration::ZERO);
        assert!(!c.held());
        assert!(c.event(axis(99999), Duration::ZERO).is_empty());
    }
    #[test]
    fn disconnect_releases_everything() {
        let mut c = setup();
        c.event(key(304, 1), Duration::ZERO);
        c.event(key(305, 1), Duration::ZERO);
        c.disconnect();
        assert!(!c.held());
        assert!(c.tick(Duration::from_secs(5)).is_empty());
        assert_eq!(
            c.event(key(304, 1), Duration::from_secs(6)),
            vec![Action::Confirm]
        );
    }
    #[test]
    fn dropped_stream_requires_report() {
        let mut c = setup();
        c.event(key(305, 1), Duration::ZERO);
        c.event(
            RawEvent {
                kind: 0,
                code: 3,
                value: 0,
            },
            Duration::ZERO,
        );
        assert!(c.event(key(304, 1), Duration::ZERO).is_empty());
        assert!(!c.held());
        c.event(
            RawEvent {
                kind: 0,
                code: 0,
                value: 0,
            },
            Duration::ZERO,
        );
        assert_eq!(c.event(key(304, 1), Duration::ZERO), vec![Action::Confirm]);
    }
    #[test]
    fn multiple_sources_do_not_duplicate_actions() {
        let mut c = Controller::new(Mapping {
            buttons: vec![
                Button {
                    code: 1,
                    action: Action::Down,
                },
                Button {
                    code: 2,
                    action: Action::Down,
                },
            ],
            axes: vec![],
        })
        .unwrap();
        assert_eq!(c.event(key(1, 1), Duration::ZERO), vec![Action::Down]);
        assert!(c.event(key(2, 1), Duration::ZERO).is_empty());
        c.event(key(1, 0), Duration::ZERO);
        assert!(c.held());
        c.event(key(2, 0), Duration::ZERO);
        assert!(!c.held());
    }
    #[test]
    fn malformed_mapping_is_rejected() {
        let mut map = setup().mapping;
        map.buttons.push(map.buttons[0].clone());
        assert!(map.validate().is_err());
        let mut map = setup().mapping;
        map.axes[0].max = map.axes[0].min;
        assert!(map.validate().is_err());
    }
}

#[cfg(test)]
mod opposed_tests {
    use super::*;
    #[test]
    fn opposite_held_directions_do_not_oscillate() {
        let mut c = Controller::new(Mapping {
            buttons: vec![
                Button {
                    code: 1,
                    action: Action::Left,
                },
                Button {
                    code: 2,
                    action: Action::Right,
                },
            ],
            axes: vec![],
        })
        .unwrap();
        for code in [1, 2] {
            c.event(
                RawEvent {
                    kind: 1,
                    code,
                    value: 1,
                },
                Duration::ZERO,
            );
        }
        assert!(c.tick(Duration::from_secs(1)).is_empty());
        c.event(
            RawEvent {
                kind: 1,
                code: 2,
                value: 0,
            },
            Duration::from_secs(1),
        );
        assert_eq!(c.tick(Duration::from_secs(2)), vec![Action::Left]);
    }
}
