//! Pure scene selection. Geometry belongs to a single shared art resource.
use crate::calendar::{season, Season};
use chrono::{NaiveDateTime, Timelike};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Morning,
    Noon,
    Afternoon,
    Dusk,
    Night,
    Late,
}
impl Phase {
    pub fn index(self) -> i32 {
        self as i32
    }
    pub fn at(hour: u32) -> Self {
        match hour {
            6..=10 => Self::Morning,
            11..=13 => Self::Noon,
            14..=16 => Self::Afternoon,
            17..=19 => Self::Dusk,
            20..=23 => Self::Night,
            _ => Self::Late,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Conditions {
    Clear,
    Cloud,
    Rain,
    Fog,
    Snow,
    #[default]
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scene {
    pub phase: Phase,
    pub season: Option<Season>,
    pub weather: Conditions,
    pub decoration: i32,
    pub trusted: bool,
}
impl Scene {
    pub fn select(
        at: Option<NaiveDateTime>,
        weather: Conditions,
        events: &[&str],
        neutral: bool,
    ) -> Self {
        if neutral || at.is_none() {
            return Self {
                phase: Phase::Noon,
                season: None,
                weather: Conditions::Unknown,
                decoration: 0,
                trusted: false,
            };
        }
        let at = at.expect("checked");
        Self {
            phase: Phase::at(at.hour()),
            season: Some(season(at.date())),
            weather,
            decoration: if events.contains(&"halloween") {
                1
            } else if events.contains(&"christmas") {
                2
            } else if events.contains(&"new-year") {
                3
            } else {
                0
            },
            trusted: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_boundaries() {
        for (h, p) in [
            (0, Phase::Late),
            (5, Phase::Late),
            (6, Phase::Morning),
            (10, Phase::Morning),
            (11, Phase::Noon),
            (13, Phase::Noon),
            (14, Phase::Afternoon),
            (16, Phase::Afternoon),
            (17, Phase::Dusk),
            (19, Phase::Dusk),
            (20, Phase::Night),
            (23, Phase::Night),
        ] {
            assert_eq!(Phase::at(h), p);
        }
    }
    #[test]
    fn unknown_and_neutral_do_not_invent_weather() {
        for neutral in [true, false] {
            let s = Scene::select(None, Conditions::Rain, &["halloween"], neutral);
            assert_eq!(s.weather, Conditions::Unknown);
            assert_eq!(s.decoration, 0);
            assert!(!s.trusted);
        }
    }
    #[test]
    fn direct_boot_and_calendar() {
        let at = NaiveDateTime::parse_from_str("2026-10-31T21:30", "%Y-%m-%dT%H:%M").unwrap();
        let s = Scene::select(Some(at), Conditions::Rain, &["halloween"], false);
        assert_eq!(s.phase, Phase::Night);
        assert_eq!(s.season, Some(Season::Autumn));
        assert_eq!(s.decoration, 1);
        assert_eq!(
            s,
            Scene::select(Some(at), Conditions::Rain, &["halloween"], false)
        );
    }
}
