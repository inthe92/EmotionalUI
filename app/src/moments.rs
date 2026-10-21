//! Development fixtures. Never reads or sets the operating-system clock.
use chrono::{Datelike, NaiveDateTime};
use emotional_services::{clock::Clock, weather::Weather};
pub const NAMES: [&str; 6] = [
    "summer-morning",
    "autumn-rain",
    "halloween",
    "christmas",
    "winter-sunday",
    "quiet-dawn",
];
pub fn context(name: &str) -> Result<(Clock, Weather), String> {
    let (at, weather) = match name {
        "summer-morning" => ("2026-07-15T08:30", Weather::Clear),
        "autumn-rain" => ("2026-10-18T15:30", Weather::Rain),
        "halloween" => ("2026-10-31T21:30", Weather::Cloud),
        "christmas" => ("2026-12-25T09:00", Weather::Clear),
        "winter-sunday" => ("2026-01-18T11:00", Weather::Unknown),
        "quiet-dawn" => ("2026-02-02T02:00", Weather::Unknown),
        _ => return Err(format!("Unknown development moment: {name}")),
    };
    let date = NaiveDateTime::parse_from_str(at, "%Y-%m-%dT%H:%M").map_err(|e| e.to_string())?;
    println!(
        "DEVELOPMENT MOMENT: {name} | weekday={} | season={} | weather={weather:?}",
        date.weekday(),
        emotional_core::calendar::season(date.date()).label()
    );
    Ok((Clock::new(Some(date)), weather))
}

/// DEV-only data injected into the existing frontend, never a product screen.
#[derive(serde::Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Environment {
    pub at: String,
    pub ambient: emotional_core::ambient::Settings,
    pub moment: Option<String>,
    pub unknown_clock: bool,
    pub cache_file: Option<std::path::PathBuf>,
    pub condition: emotional_core::ambient::WeatherKind,
    pub weather_state: FixtureSource,
}
#[derive(Clone, Copy, Default, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixtureSource {
    Live,
    Cached,
    Stale,
    #[default]
    Unknown,
}
impl Environment {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let mut text = String::new();
        std::fs::File::open(path)
            .map_err(|_| "Cannot open DEV environment")?
            .take(16385)
            .read_to_string(&mut text)
            .map_err(|_| "Cannot read DEV environment")?;
        if text.len() > 16384 {
            return Err("DEV environment too large".into());
        }
        let mut result: Self = toml::from_str(&text).map_err(|_| "Invalid DEV environment")?;
        if result.ambient.timezone.is_none() {
            result.ambient.timezone = Some("Etc/UTC".into());
        }
        if result.at.is_empty() || !result.ambient.valid() {
            return Err("DEV environment requires valid date/settings".into());
        }
        Ok(result)
    }
    pub fn install(
        &self,
        runtime: &mut emotional_services::ambient::Runtime,
    ) -> Result<Clock, String> {
        use emotional_core::{
            ambient::{Observation, WeatherSource},
            environment::DayMoment,
        };
        let mut clock = runtime.demo_clock(&self.at)?;
        if let Some(id) = &self.moment {
            let m = DayMoment::ALL
                .into_iter()
                .find(|m| m.id() == id)
                .ok_or("Unknown DEV moment")?;
            let c = runtime.capture(&clock, Default::default(), true);
            let Some(s) = c.solar else {
                return Err(
                    "Moment selection requires a configured location and reliable date".into(),
                );
            };
            let (Some(dawn), Some(rise), Some(set), Some(dusk)) = (s.dawn, s.rise, s.set, s.dusk)
            else {
                return Err("No solar events for this date/location; use an explicit time".into());
            };
            let morning = rise + (s.noon - rise) * 35 / 100;
            let afternoon = s.noon + (set - s.noon) * 55 / 100;
            let utc = match m {
                DayMoment::LateNight => dawn - chrono::Duration::hours(2),
                DayMoment::EarlyMorning => dawn + (morning - dawn) / 2,
                DayMoment::MidMorning => morning + (s.noon - morning) / 2,
                DayMoment::EarlyAfternoon => s.noon + (afternoon - s.noon) / 2,
                DayMoment::LateAfternoon => afternoon + (set - afternoon) / 2,
                DayMoment::Dusk => set + (dusk - set) / 2,
                DayMoment::Night => dusk + chrono::Duration::hours(1),
            };
            clock = runtime.demo_clock(&utc.and_utc().to_rfc3339())?;
        }
        let c = runtime.capture(&clock, Default::default(), true);
        let source = match self.weather_state {
            FixtureSource::Live => WeatherSource::Live,
            FixtureSource::Cached => WeatherSource::Cached,
            FixtureSource::Stale => WeatherSource::Stale,
            FixtureSource::Unknown => WeatherSource::Unknown,
        };
        let record = if let Some(path) = &self.cache_file {
            runtime
                .load_cache(path)
                .map_err(|_| "Invalid or unverified DEV cache")?;
            runtime.cached.clone()
        } else if matches!(self.weather_state, FixtureSource::Unknown) {
            None
        } else {
            let location = runtime
                .settings
                .location
                .ok_or("Simulated weather requires a chosen test location")?;
            c.zone.utc.map(|now| {
                let ttl = i64::from(runtime.settings.weather_ttl_minutes);
                let age = if matches!(self.weather_state, FixtureSource::Stale) {
                    ttl + 1
                } else {
                    ttl.min(10) / 2
                };
                let observed = now - chrono::Duration::minutes(age);
                Observation {
                    location,
                    observed_utc: observed,
                    received_utc: Some(observed),
                    expires_utc: Some(observed + chrono::Duration::minutes(ttl)),
                    condition: self.condition,
                    cloud_percent: 50.,
                    precipitation_mm: 0.,
                    temperature_c: 10.,
                    rain_mm: None,
                    snowfall_cm: None,
                }
            })
        };
        runtime.set_demo_observation(source, record);
        if self.unknown_clock {
            clock.unknown();
        }
        Ok(clock)
    }
}
pub fn verify_environment(state: &crate::state::State, ui: &crate::Home) {
    let c = state
        .ambient
        .capture(&state.clock, Default::default(), true);
    let theme =
        emotional_core::ambient_art::AmbientTheme::resolve(&c, &state.ambient.settings, false);
    let scene = emotional_core::environment::SceneResolver::resolve(&c.scene(), false);
    use slint::ComponentHandle;
    assert_eq!(ui.get_scene_id().as_str(), scene.id);
    assert_eq!(ui.get_scene_weather(), theme.weather_index());
    assert_eq!(
        ui.global::<crate::DaycycleAssets>().get_palette_index(),
        theme.day.from as i32
    );
    let style = ui.global::<crate::DaycycleAssets>();
    assert_eq!(style.get_palette_next_index(), theme.day.to as i32);
    assert!((style.get_day_mix() - theme.day.mix).abs() < 0.00001);
    assert_eq!(style.get_season_index(), theme.season_index());
    assert_eq!(style.get_weather_index(), theme.weather_index());
    assert_eq!(style.get_celebration_index(), theme.event.decoration());
    assert!(ui.get_ambient_label().starts_with("DEMO"));
    let solar = c
        .solar
        .map(|s| {
            format!(
                "rise={} set={}",
                s.rise.map(|t| t.to_string()).unwrap_or_default(),
                s.set.map(|t| t.to_string()).unwrap_or_default()
            )
        })
        .unwrap_or_default();
    println!("DEMO_CONTEXT scene={} weather={:?} kind={:?} season={:?} palette={} next={} mix={:.5} offset={:?} event={} solar_utc=[{}] ui_verified=true",scene.id,c.weather.source,c.weather.kind(),c.season,theme.day.from,theme.day.to,theme.day.mix,c.zone.offset_seconds,theme.event.id(),solar);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seven_solar_moments_times_four_calculated_seasons() {
        use emotional_core::{
            ambient::Settings, calendar::Season, environment::DayMoment, solar::Location,
        };
        for (date, season) in [
            ("2026-03-20T12:00", Season::Spring),
            ("2026-06-21T12:00", Season::Summer),
            ("2026-09-22T12:00", Season::Autumn),
            ("2026-12-21T12:00", Season::Winter),
        ] {
            for moment in DayMoment::ALL {
                let fixture = Environment {
                    at: date.into(),
                    ambient: Settings {
                        timezone: Some("Europe/Madrid".into()),
                        hemisphere: Some(emotional_core::ambient::Hemisphere::North),
                        location: Some(Location {
                            latitude: 40.,
                            longitude: -3.,
                        }),
                        ..Default::default()
                    },
                    moment: Some(moment.id().into()),
                    weather_state: FixtureSource::Cached,
                    condition: emotional_core::ambient::WeatherKind::Rain,
                    ..Default::default()
                };
                let mut r =
                    emotional_services::ambient::Runtime::new(fixture.ambient.clone()).unwrap();
                let clock = fixture.install(&mut r).unwrap();
                let c = r.capture(&clock, Default::default(), true);
                assert_eq!(c.moment, Some(moment));
                assert_eq!(c.season, Some(season));
                assert!(r.demo_context());
                assert!(r.cached.is_none());
            }
        }
    }
    #[test]
    fn synthetic_weather_states_and_events_use_the_real_resolvers() {
        use emotional_core::{
            ambient::{Settings, WeatherKind, WeatherSource},
            ambient_art::AmbientTheme,
            solar::Location,
        };
        for (input, expected, layer) in [
            (FixtureSource::Live, WeatherSource::Live, 4),
            (FixtureSource::Cached, WeatherSource::Cached, 4),
            (FixtureSource::Stale, WeatherSource::Stale, 0),
            (FixtureSource::Unknown, WeatherSource::Unknown, 0),
        ] {
            for (at, event) in [
                ("2026-10-31T20:00", "halloween"),
                ("2026-12-25T16:00", "christmas"),
                ("2027-01-01T01:00", "new_year"),
            ] {
                let case = Environment {
                    at: at.into(),
                    ambient: Settings {
                        timezone: Some("Etc/UTC".into()),
                        location: Some(Location {
                            latitude: 0.,
                            longitude: 0.,
                        }),
                        ..Default::default()
                    },
                    weather_state: input,
                    condition: WeatherKind::Snow,
                    ..Default::default()
                };
                let mut r =
                    emotional_services::ambient::Runtime::new(case.ambient.clone()).unwrap();
                let clock = case.install(&mut r).unwrap();
                let c = r.capture(&clock, Default::default(), true);
                let theme = AmbientTheme::resolve(&c, &r.settings, false);
                assert_eq!(c.weather.source, expected);
                assert_eq!(theme.weather_index(), layer);
                assert_eq!(theme.event.id(), event);
            }
        }
    }
    #[test]
    fn scenarios_compute_dates_seasons_holidays_and_weather() {
        for name in NAMES {
            let (clock, _) = context(name).unwrap();
            assert!(clock.known_now().is_some());
        }
        let (clock, w) = context("winter-sunday").unwrap();
        assert_eq!(clock.now().weekday(), chrono::Weekday::Sun);
        assert_eq!(w, Weather::Unknown);
        for (name, expected) in [("halloween", "Halloween"), ("christmas", "Navidad")] {
            let (clock, _) = context(name).unwrap();
            let config: crate::Config =
                toml::from_str(include_str!("../../assets/themes/default.toml")).unwrap();
            assert_eq!(
                emotional_core::calendar::holiday(clock.now().date(), "ES", &config.holidays),
                Some(expected)
            );
        }
        assert!(context("invalid").is_err());
    }
}
