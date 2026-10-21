//! Composable art direction, independent of renderer, recommendation and audio.
use crate::{ambient::AmbientContext, solar::Daylight};
use chrono::{Datelike, Timelike};
use serde::Deserialize;

/// Opt-in/out cultural observances, not a claim about public holidays worldwide.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EditorialEvent {
    None,
    Halloween,
    Christmas,
    NewYear,
}
pub type Celebration = EditorialEvent;
impl EditorialEvent {
    pub fn on(self, month: u32, day: u32) -> bool {
        matches!(
            (self, month, day),
            (Self::Halloween, 10, 31) | (Self::Christmas, 12, 25) | (Self::NewYear, 1, 1)
        )
    }
    pub fn id(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Halloween => "halloween",
            Self::Christmas => "christmas",
            Self::NewYear => "new_year",
        }
    }
    pub fn decoration(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Halloween => 1,
            Self::Christmas => 2,
            Self::NewYear => 3,
        }
    }
}
/// Configured annual editorial date. Reuses one restrained visual style.
/// This is not a legal calendar or an automatic claim of local observance.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditorialEventRule {
    pub id: String,
    pub month: u32,
    pub day: u32,
    pub style: EditorialEvent,
}
impl EditorialEventRule {
    pub fn valid(&self) -> bool {
        !self.id.is_empty()
            && self.id.len() <= 48
            && self
                .id
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_')
            && chrono::NaiveDate::from_ymd_opt(2000, self.month, self.day).is_some()
    }
}
/// One art recipe consumed by both views; no renderer, I/O or audio dependency.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientTheme<'a> {
    pub day: Blend,
    pub season: Option<crate::calendar::Season>,
    pub weather: crate::ambient::WeatherKind,
    pub event: EditorialEvent,
    pub event_id: Option<&'a str>,
}
impl<'a> AmbientTheme<'a> {
    pub fn resolve(
        c: &AmbientContext,
        settings: &'a crate::ambient::Settings,
        neutral: bool,
    ) -> Self {
        if neutral || !c.time.reliable || c.time.local.is_none() {
            return Self {
                day: Blend::still(7),
                season: None,
                weather: Default::default(),
                event: EditorialEvent::None,
                event_id: None,
            };
        }
        let date = c.time.local.expect("checked").date();
        // An explicit custom rule wins, including style=none to suppress decoration.
        let custom = settings
            .editorial_events
            .iter()
            .find(|r| r.valid() && r.month == date.month() && r.day == date.day());
        let (event, event_id) = if let Some(r) = custom {
            (r.style, Some(r.id.as_str()))
        } else if let Some(event) = c.celebration(&settings.celebrations) {
            (event, Some(event.id()))
        } else {
            (EditorialEvent::None, None)
        };
        Self {
            day: c.artwork_blend(),
            season: c.season,
            weather: c.weather.kind(),
            event,
            event_id,
        }
    }
    pub fn season_index(self) -> i32 {
        use crate::calendar::Season::*;
        match self.season {
            Some(Winter) => 0,
            Some(Spring) => 1,
            Some(Summer) => 2,
            Some(Autumn) => 3,
            None => 4,
        }
    }
    pub fn weather_index(self) -> i32 {
        use crate::ambient::WeatherKind::*;
        match self.weather {
            Cloud | Storm => 1,
            Rain => 2,
            Fog => 3,
            Snow => 4,
            Clear | Unknown => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blend {
    pub from: usize,
    pub to: usize,
    pub mix: f32,
}
impl Blend {
    pub fn still(index: usize) -> Self {
        Self {
            from: index,
            to: index,
            mix: 0.,
        }
    }
}
// Artistic keyframes, in local seconds. Separate from semantic day-band boundaries.
pub const CIVIL_KEYS: [i64; 7] = [
    3 * 3600,
    7 * 3600 + 1800,
    10 * 3600 + 1800,
    14 * 3600,
    17 * 3600 + 1800,
    20 * 3600,
    22 * 3600 + 1800,
];
fn between(now: i64, keys: [i64; 7]) -> Blend {
    let now = keys[0] + (now - keys[0]).rem_euclid(86400);
    let from = keys.iter().rposition(|t| now >= *t).unwrap_or(0);
    let to = (from + 1) % 7;
    let end = if to == 0 { keys[0] + 86400 } else { keys[to] };
    let linear = (now - keys[from]) as f32 / (end - keys[from]) as f32;
    // Zero slope at keyframes; no flash or color step across midnight.
    Blend {
        from,
        to,
        mix: linear * linear * (3. - 2. * linear),
    }
}
impl AmbientContext {
    pub fn artwork_blend(&self) -> Blend {
        let Some(local) = self.time.local.filter(|_| self.time.reliable) else {
            return Blend::still(7);
        };
        if let Some(s) = self.solar {
            if s.daylight != Daylight::Normal || s.dawn.is_none() || s.dusk.is_none() {
                // Never fabricate a dark night during polar day/continuous twilight.
                return Blend::still(self.moment.map(|m| m.index()).unwrap_or(7));
            }
            if let (Some(utc), Some(dawn), Some(rise), Some(set), Some(dusk)) =
                (self.zone.utc, s.dawn, s.rise, s.set, s.dusk)
            {
                let n = |t: chrono::NaiveDateTime| t.and_utc().timestamp();
                let keys = [
                    n(dawn) - 3 * 3600,
                    n(rise),
                    n(rise + (s.noon - rise) / 2),
                    n(s.noon + (set - s.noon) * 27 / 100),
                    n(s.noon + (set - s.noon) * 77 / 100),
                    n(set + (dusk - set) / 2),
                    n(dusk) + 5400,
                ];
                if keys.windows(2).all(|p| p[1] > p[0]) && keys[6] - keys[0] < 86400 {
                    return between(n(utc), keys);
                }
                return Blend::still(self.moment.map(|m| m.index()).unwrap_or(7));
            }
        }
        between(i64::from(local.num_seconds_from_midnight()), CIVIL_KEYS)
    }
    pub fn celebration(&self, enabled: &[Celebration]) -> Option<Celebration> {
        let at = self.time.local.filter(|_| self.time.reliable)?;
        enabled.iter().copied().find(|c| c.on(at.month(), at.day()))
    }
}
