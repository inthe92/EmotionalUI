//! The single renderer-independent environmental context. Never queries hardware.
use crate::{
    calendar::Season,
    environment::{DayMoment, SceneContext, TimeContext},
    solar::{self, Location, SolarDay},
};
use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hemisphere {
    #[default]
    North,
    South,
}
pub fn season(date: NaiveDate, hemisphere: Hemisphere) -> Season {
    let north = crate::calendar::season(date);
    if hemisphere == Hemisphere::North {
        return north;
    }
    match north {
        Season::Spring => Season::Autumn,
        Season::Summer => Season::Winter,
        Season::Autumn => Season::Spring,
        Season::Winter => Season::Summer,
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub hemisphere: Option<Hemisphere>,
    pub location: Option<Location>,
    pub timezone: Option<String>,
    pub weather_ttl_minutes: u32,
    /// Cultural overlays; [] disables them. Not a legal holiday calendar.
    pub celebrations: Vec<crate::ambient_art::Celebration>,
    pub editorial_events: Vec<crate::ambient_art::EditorialEventRule>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            hemisphere: None,
            location: None,
            timezone: None,
            weather_ttl_minutes: 180,
            editorial_events: vec![],
            celebrations: vec![
                crate::ambient_art::Celebration::Halloween,
                crate::ambient_art::Celebration::Christmas,
                crate::ambient_art::Celebration::NewYear,
            ],
        }
    }
}
impl Settings {
    pub fn effective_hemisphere(&self) -> Option<Hemisphere> {
        self.hemisphere.or_else(|| {
            self.location.filter(|l| l.valid()).and_then(|l| {
                if l.latitude > 0. {
                    Some(Hemisphere::North)
                } else if l.latitude < 0. {
                    Some(Hemisphere::South)
                } else {
                    None
                }
            })
        })
    }
    pub fn valid(&self) -> bool {
        let mut ids = std::collections::HashSet::new();
        let mut dates = std::collections::HashSet::new();
        self.location.is_none_or(Location::valid)
            && (1..=1440).contains(&self.weather_ttl_minutes)
            && self.editorial_events.len() <= 32
            && self
                .editorial_events
                .iter()
                .all(|r| r.valid() && ids.insert(&r.id) && dates.insert((r.month, r.day)))
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeatherKind {
    Clear,
    Cloud,
    Rain,
    Fog,
    Snow,
    Storm,
    #[default]
    Unknown,
}
impl WeatherKind {
    pub fn scene(self) -> crate::scene::Conditions {
        use crate::scene::Conditions as C;
        match self {
            Self::Clear => C::Clear,
            Self::Cloud | Self::Storm => C::Cloud,
            Self::Rain => C::Rain,
            Self::Fog => C::Fog,
            Self::Snow => C::Snow,
            Self::Unknown => C::Unknown,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WeatherSource {
    Live,
    Cached,
    Stale,
    #[default]
    Unknown,
    Demo,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observation {
    pub location: Location,
    pub observed_utc: NaiveDateTime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub received_utc: Option<NaiveDateTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_utc: Option<NaiveDateTime>,
    pub condition: WeatherKind,
    pub cloud_percent: f64,
    pub precipitation_mm: f64,
    pub temperature_c: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rain_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snowfall_cm: Option<f64>,
}
impl Observation {
    pub fn valid(&self) -> bool {
        self.location.valid()
            && self.cloud_percent.is_finite()
            && (0. ..=100.).contains(&self.cloud_percent)
            && self.precipitation_mm.is_finite()
            && (0. ..=500.).contains(&self.precipitation_mm)
            && self.temperature_c.is_finite()
            && (-100. ..=65.).contains(&self.temperature_c)
            && self
                .rain_mm
                .is_none_or(|v| v.is_finite() && (0. ..=500.).contains(&v))
            && self
                .snowfall_cm
                .is_none_or(|v| v.is_finite() && (0. ..=500.).contains(&v))
            && crate::environment::plausible_time(self.observed_utc)
            && self
                .received_utc
                .is_none_or(|t| crate::environment::plausible_time(t) && t >= self.observed_utc)
            && self
                .expires_utc
                .is_none_or(|t| crate::environment::plausible_time(t) && t >= self.observed_utc)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WeatherContext {
    pub source: WeatherSource,
    pub observation: Option<Observation>,
    pub demo: WeatherKind,
}
impl WeatherContext {
    pub fn kind(&self) -> WeatherKind {
        if matches!(self.source, WeatherSource::Stale | WeatherSource::Unknown) {
            WeatherKind::Unknown
        } else if self.source == WeatherSource::Demo {
            self.demo
        } else {
            self.observation
                .as_ref()
                .map(|o| o.condition)
                .unwrap_or_default()
        }
    }
}
pub fn weather(
    now: Option<NaiveDateTime>,
    location: Option<Location>,
    live: Option<&Observation>,
    cached: Option<&Observation>,
    ttl_minutes: u32,
) -> WeatherContext {
    let Some((now, location)) = now.zip(location) else {
        return WeatherContext::default();
    };
    if !(1..=1440).contains(&ttl_minutes) {
        return WeatherContext::default();
    }
    let mut stale = false;
    for (o, source) in [(live, WeatherSource::Live), (cached, WeatherSource::Cached)] {
        if let Some(o) = o.filter(|o| {
            o.valid()
                && o.location == location
                && now >= o.observed_utc
                && o.received_utc.is_none_or(|t| t <= now)
        }) {
            if now - o.observed_utc > Duration::minutes(i64::from(ttl_minutes))
                || o.expires_utc.is_some_and(|t| now > t)
            {
                stale = true;
                continue;
            }
            return WeatherContext {
                source,
                observation: Some(o.clone()),
                demo: WeatherKind::Unknown,
            };
        }
    }
    WeatherContext {
        source: if stale {
            WeatherSource::Stale
        } else {
            WeatherSource::Unknown
        },
        ..Default::default()
    }
}
#[derive(Clone, Debug)]
pub struct ZonedTime {
    pub timezone: Option<String>,
    pub utc: Option<NaiveDateTime>,
    pub offset_seconds: Option<i32>,
    pub dst: Option<bool>,
}
#[derive(Clone, Debug)]
pub struct AmbientContext {
    pub time: TimeContext,
    pub zone: ZonedTime,
    pub hemisphere: Option<Hemisphere>,
    pub location: Option<Location>,
    pub solar: Option<SolarDay>,
    pub moment: Option<DayMoment>,
    pub season: Option<Season>,
    pub weather: WeatherContext,
    pub reduced_motion: bool,
}
impl AmbientContext {
    pub fn resolve(
        time: TimeContext,
        zone: ZonedTime,
        settings: &Settings,
        weather: WeatherContext,
        reduced_motion: bool,
    ) -> Self {
        let at = time.local.filter(|_| time.reliable);
        let solar = at.and_then(|at| {
            settings
                .location
                .filter(|l| l.valid())
                .zip(zone.utc)
                .and_then(|(l, _)| solar::day(at.date(), l))
        });
        Self::resolve_with_solar(time, zone, settings, weather, reduced_motion, solar)
    }
    /// Adapter supplies its bounded day/location cache; pure callers can use resolve.
    pub fn resolve_with_solar(
        time: TimeContext,
        zone: ZonedTime,
        settings: &Settings,
        weather: WeatherContext,
        reduced_motion: bool,
        solar: Option<SolarDay>,
    ) -> Self {
        let at = time.local.filter(|_| time.reliable);
        let solar =
            solar.filter(|_| at.is_some() && settings.location.is_some_and(Location::valid));
        let moment = at.map(|at| {
            solar
                .zip(zone.utc)
                .map(|(s, u)| s.moment(u, at))
                .unwrap_or_else(|| DayMoment::at(at))
        });
        let hemisphere = settings.effective_hemisphere();
        Self {
            time,
            zone,
            hemisphere,
            location: settings.location,
            solar,
            moment,
            season: at.zip(hemisphere).map(|(at, h)| season(at.date(), h)),
            weather: if at.is_some() {
                weather
            } else {
                WeatherContext::default()
            },
            reduced_motion,
        }
    }
    pub fn scene(&self) -> SceneContext {
        SceneContext {
            time: self.time,
            moment: self.moment,
            season: self.season,
            weather: self.weather.kind().scene(),
        }
    }
}
