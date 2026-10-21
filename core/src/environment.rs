//! Offline environmental context; no clock, network, files or renderer access.
pub use crate::{calendar::Season, scene::Conditions as WeatherState};
use chrono::{Datelike, NaiveDateTime, Timelike};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeSource {
    System,
    NetworkSynchronized,
    LastKnown,
    Simulated,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeContext {
    pub local: Option<NaiveDateTime>,
    pub source: TimeSource,
    /// Plausible current time, not proof of NTP synchronization.
    pub reliable: bool,
}
pub fn plausible_time(at: NaiveDateTime) -> bool {
    (2020..=2100).contains(&at.year())
}
/// A saved wall time is a historical hint, never an invented current timestamp.
pub fn resolve_time(
    system: Option<NaiveDateTime>,
    synchronized: Option<NaiveDateTime>,
    saved: Option<NaiveDateTime>,
) -> TimeContext {
    for (sample, source) in [
        (system, TimeSource::System),
        (synchronized, TimeSource::NetworkSynchronized),
    ] {
        if let Some(local) = sample.filter(|at| plausible_time(*at)) {
            return TimeContext {
                local: Some(local),
                source,
                reliable: true,
            };
        }
    }
    if let Some(local) = saved.filter(|at| plausible_time(*at)) {
        return TimeContext {
            local: Some(local),
            source: TimeSource::LastKnown,
            reliable: false,
        };
    }
    TimeContext {
        local: None,
        source: TimeSource::Unknown,
        reliable: false,
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayMoment {
    LateNight,
    EarlyMorning,
    MidMorning,
    EarlyAfternoon,
    LateAfternoon,
    Dusk,
    Night,
}
impl DayMoment {
    pub const ALL: [Self; 7] = [
        Self::LateNight,
        Self::EarlyMorning,
        Self::MidMorning,
        Self::EarlyAfternoon,
        Self::LateAfternoon,
        Self::Dusk,
        Self::Night,
    ];
    // One policy location, local minutes since midnight. Provisional fixed bands.
    pub const START_MINUTES: [u32; 7] = [0, 360, 540, 720, 960, 1140, 1260];
    pub fn at(at: NaiveDateTime) -> Self {
        let minute = at.hour() * 60 + at.minute();
        Self::ALL[Self::START_MINUTES
            .iter()
            .rposition(|start| minute >= *start)
            .unwrap_or(0)]
    }
    pub fn index(self) -> usize {
        self as usize
    }
    pub fn id(self) -> &'static str {
        [
            "madrugada",
            "manana_temprana",
            "media_manana",
            "primera_tarde",
            "media_tarde",
            "anochecer",
            "noche",
        ][self.index()]
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneContext {
    pub time: TimeContext,
    pub moment: Option<DayMoment>,
    pub season: Option<Season>,
    pub weather: WeatherState,
}
impl SceneContext {
    pub fn new(time: TimeContext, weather: WeatherState) -> Self {
        let at = time.local.filter(|_| time.reliable);
        Self {
            time,
            moment: at.map(DayMoment::at),
            season: at.map(|d| crate::calendar::season(d.date())),
            weather: if at.is_some() {
                weather
            } else {
                WeatherState::Unknown
            },
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneSelection {
    pub id: &'static str,
    /// Seven base slots plus a neutral slot, bound to local embedded resources.
    pub background: usize,
    pub tint_rgba: [u8; 4],
    pub lamp: f32,
    pub placeholder: bool,
}
pub struct SceneResolver;
impl SceneResolver {
    pub fn resolve(context: &SceneContext, neutral: bool) -> SceneSelection {
        let Some(moment) = context.moment.filter(|_| !neutral) else {
            return SceneSelection {
                id: "neutro",
                background: DayMoment::ALL.len(),
                tint_rgba: [35, 45, 39, 40],
                lamp: 0.,
                placeholder: true,
            };
        };
        // Lighting and lamp are now baked in the seven fixed-camera resources.
        // UI color/material parameters live together in daycycle/palettes.json.
        SceneSelection {
            id: moment.id(),
            background: moment.index(),
            tint_rgba: [0, 0, 0, 0],
            lamp: 0.,
            placeholder: true,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn date(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M").unwrap()
    }
    #[test]
    fn seven_complete_bands_at_every_minute() {
        let starts = [0, 360, 540, 720, 960, 1140, 1260, 1440];
        for band in 0..7 {
            for minute in starts[band]..starts[band + 1] {
                let at = date("2026-10-20T00:00") + chrono::Duration::minutes(minute);
                assert_eq!(DayMoment::at(at), DayMoment::ALL[band]);
            }
        }
    }
    #[test]
    fn source_priority_and_unknown_clock() {
        let now = date("2026-10-20T09:00");
        let bad = date("1970-01-01T00:00");
        assert_eq!(
            resolve_time(Some(now), Some(now), Some(now)).source,
            TimeSource::System
        );
        assert_eq!(
            resolve_time(Some(bad), Some(now), Some(now)).source,
            TimeSource::NetworkSynchronized
        );
        let saved = resolve_time(Some(bad), None, Some(now));
        assert_eq!(saved.source, TimeSource::LastKnown);
        assert!(!saved.reliable);
        assert_eq!(
            resolve_time(None, None, Some(bad)).source,
            TimeSource::Unknown
        );
    }
    #[test]
    fn offline_seasons_and_missing_weather() {
        for (text, season) in [
            ("2026-03-01T09:00", Season::Spring),
            ("2026-06-01T09:00", Season::Summer),
            ("2026-09-01T09:00", Season::Autumn),
            ("2026-12-01T09:00", Season::Winter),
        ] {
            let c = SceneContext::new(
                resolve_time(Some(date(text)), None, None),
                WeatherState::Unknown,
            );
            assert_eq!(c.season, Some(season));
            assert_eq!(c.weather, WeatherState::Unknown);
        }
    }
    #[test]
    fn selected_scene_is_stable_and_all_seven_are_reachable() {
        for m in DayMoment::ALL {
            let at = date("2026-10-20T00:00")
                + chrono::Duration::minutes(i64::from(DayMoment::START_MINUTES[m.index()]));
            let c = SceneContext::new(resolve_time(Some(at), None, None), WeatherState::Unknown);
            let selected = SceneResolver::resolve(&c, false);
            assert_eq!(selected.background, m.index());
            assert_eq!(selected.id, m.id());
            assert_eq!(selected, SceneResolver::resolve(&c, false));
        }
    }
    #[test]
    fn stale_time_and_neutral_do_not_invent_context() {
        let saved = SceneContext::new(
            resolve_time(None, None, Some(date("2026-10-31T21:30"))),
            WeatherState::Rain,
        );
        assert_eq!(saved.weather, WeatherState::Unknown);
        assert_eq!(saved.season, None);
        assert_eq!(SceneResolver::resolve(&saved, false).id, "neutro");
        let good = SceneContext::new(
            resolve_time(Some(date("2026-10-31T21:30")), None, None),
            WeatherState::Rain,
        );
        assert_eq!(SceneResolver::resolve(&good, true).id, "neutro");
    }
}
