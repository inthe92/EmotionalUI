//! Local time-zone adapter, optional private weather cache and nonblocking worker.
use chrono::{Local, Offset, TimeZone, Utc};
use chrono_tz::{OffsetComponents, Tz};
use emotional_core::{ambient::*, environment::TimeSource};
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};
type SolarCacheEntry = (
    chrono::NaiveDate,
    emotional_core::solar::Location,
    Option<emotional_core::solar::SolarDay>,
);
#[cfg(test)]
mod offline_tests;

pub trait WeatherProvider {
    /// Blocking adapters must run on a worker, never on the Slint thread.
    fn observe(&self, location: emotional_core::solar::Location) -> Result<Observation, String>;
}
pub struct UnavailableProvider;
impl WeatherProvider for UnavailableProvider {
    fn observe(&self, _: emotional_core::solar::Location) -> Result<Observation, String> {
        Err("Weather provider not connected".into())
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CacheFile {
    version: u32,
    record: Observation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    selection: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
}
pub fn read_cache(path: &Path) -> Result<Observation, Box<dyn std::error::Error>> {
    Ok(read_cache_file(path)?.record)
}
fn read_cache_file(path: &Path) -> Result<CacheFile, Box<dyn std::error::Error>> {
    let file = std::fs::File::open(path)?;
    let mut text = String::new();
    file.take(16385).read_to_string(&mut text)?;
    if text.len() > 16384 {
        return Err("Weather cache too large".into());
    }
    let c: CacheFile = toml::from_str(&text)?;
    if c.version != 1 || !c.record.valid() {
        return Err("Invalid weather cache".into());
    }
    Ok(c)
}
pub fn write_cache(path: &Path, record: &Observation) -> Result<(), Box<dyn std::error::Error>> {
    write_cache_inner(path, record, None)
}
pub(crate) fn read_selected_cache(
    path: &Path,
    selection: &str,
) -> Result<Observation, Box<dyn std::error::Error>> {
    let c = read_cache_file(path)?;
    if c.selection.as_deref() != Some(selection) || c.provider.as_deref() != Some("open-meteo") {
        return Err("Weather cache does not match selected location".into());
    }
    Ok(c.record)
}
pub(crate) fn write_selected_cache(
    path: &Path,
    record: &Observation,
    selection: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    write_cache_inner(path, record, Some(selection.to_owned()))
}
fn write_cache_inner(
    path: &Path,
    record: &Observation,
    selection: Option<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !record.valid() {
        return Err("Invalid weather observation".into());
    }
    let text = toml::to_string(&CacheFile {
        version: 1,
        record: record.clone(),
        provider: selection.as_ref().map(|_| "open-meteo".to_owned()),
        selection,
    })?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let temp = path.with_extension(format!("{}.{}.tmp", std::process::id(), nonce));
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut created = false;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut file = options.open(&temp)?;
        created = true;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&temp, path)?;
        Ok(())
    })();
    if result.is_err() && created {
        let _ = std::fs::remove_file(temp);
    }
    result
}
fn system_zone() -> Option<Tz> {
    if let Ok(value) = std::env::var("TZ") {
        return value.parse().ok();
    }
    if let Ok(path) = std::fs::read_link("/etc/localtime") {
        if let Some((_, id)) = path.to_str()?.split_once("/zoneinfo/") {
            return id.parse().ok();
        }
    }
    std::fs::read_to_string("/etc/timezone")
        .ok()?
        .trim()
        .parse()
        .ok()
}
pub struct Runtime {
    pub settings: Settings,
    zone: Option<Tz>,
    pub cached: Option<Observation>,
    pub demo_season: Option<emotional_core::calendar::Season>,
    live: Option<Observation>,
    provider: Option<crate::open_meteo::Worker>,
    last_weather_source: WeatherSource,
    attributed_cache: bool,
    solar_cache: std::cell::Cell<Option<SolarCacheEntry>>,
    #[cfg(test)]
    solar_calculations: std::cell::Cell<usize>,
    demo_observation: Option<(WeatherSource, Option<Observation>)>,
}
impl Runtime {
    pub fn new(settings: Settings) -> Result<Self, String> {
        if !settings.valid() {
            return Err("Invalid ambient settings: coordinates or weather TTL".into());
        }
        let zone = match settings.timezone.as_deref() {
            Some(name) => Some(name.parse::<Tz>().map_err(|_| "Invalid IANA timezone")?),
            None => system_zone(),
        };
        Ok(Self {
            settings,
            zone,
            cached: None,
            demo_season: None,
            live: None,
            provider: None,
            last_weather_source: WeatherSource::Unknown,
            attributed_cache: false,
            solar_cache: std::cell::Cell::new(None),
            #[cfg(test)]
            solar_calculations: std::cell::Cell::new(0),
            demo_observation: None,
        })
    }
    pub fn load_cache(&mut self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let cache = read_cache_file(path)?;
        if cache.provider.as_deref() != Some("open-meteo") {
            return Err("Weather cache source unverified".into());
        }
        self.attributed_cache = cache.provider.as_deref() == Some("open-meteo");
        self.cached = Some(cache.record);
        Ok(())
    }
    pub fn configure_weather(&mut self, config: crate::open_meteo::Config, cache: PathBuf) {
        self.live = None;
        self.cached = None;
        self.attributed_cache = false;
        let city_selected = config.city.is_some();
        let worker = crate::open_meteo::Worker::new(config, cache);
        if let Some(location) = worker.configured_location() {
            self.settings.location = Some(location);
        } else if city_selected {
            self.settings.location = None;
        }
        self.provider = Some(worker);
    }
    pub fn weather_configured(&self) -> bool {
        self.provider
            .as_ref()
            .is_some_and(crate::open_meteo::Worker::enabled)
    }
    pub fn weather_attributed(&self) -> bool {
        self.attributed_cache
    }
    pub fn demo_context(&self) -> bool {
        self.demo_observation.is_some()
    }
    pub fn demo_clock(&self, text: &str) -> Result<crate::clock::Clock, String> {
        if let Ok(value) = chrono::DateTime::parse_from_rfc3339(text) {
            if !emotional_core::environment::plausible_time(value.naive_utc()) {
                return Err("Simulated year outside supported environmental range".into());
            }
            let mut clock = crate::clock::Clock::new(None);
            clock.set_zoned(value, self.zone.ok_or("Explicit IANA timezone required")?);
            Ok(clock)
        } else {
            let at = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M")
                .map_err(|_| "Invalid simulated time")?;
            if !emotional_core::environment::plausible_time(at) {
                return Err("Simulated year outside supported range".into());
            }
            Ok(crate::clock::Clock::new(Some(at)))
        }
    }
    /// Only consumed with a simulated clock; never persisted or made LIVE on a real clock.
    pub fn set_demo_observation(&mut self, source: WeatherSource, record: Option<Observation>) {
        self.demo_observation = Some((source, record));
    }
    /// Called by the existing UI timer: channels and scheduling only, no I/O.
    pub fn weather_tick(&mut self, clock: &crate::clock::Clock, demo: bool, neutral: bool) -> bool {
        let context = self.capture(clock, WeatherKind::Unknown, true);
        // Repaint expiry even while focus is held or a menu is open; no stale
        // precipitation remains visible until the next navigation/ranking tick.
        let changed = context.weather.source != self.last_weather_source;
        self.last_weather_source = context.weather.source;
        let allowed = !demo
            && !neutral
            && self.demo_season.is_none()
            && self.demo_observation.is_none()
            && context.time.reliable
            && matches!(
                context.time.source,
                TimeSource::System | TimeSource::NetworkSynchronized
            );
        let Some(now) = context.zone.utc else {
            return changed;
        };
        let Some(worker) = self.provider.as_mut() else {
            return changed;
        };
        if let Some((record, source)) = worker.poll(allowed, now, self.settings.weather_ttl_minutes)
        {
            if let Some(record) = record {
                // Coordinates were explicitly chosen or uniquely resolved from
                // the user's city. A response from another worker is never used.
                self.settings.location = Some(record.location);
                self.live = (source == WeatherSource::Live).then(|| record.clone());
                self.cached = Some(record);
                self.attributed_cache = true;
            } else {
                self.live = None;
                self.cached = None;
                self.attributed_cache = false;
            }
            return true;
        }
        changed
    }
    pub fn capture(
        &self,
        clock: &crate::clock::Clock,
        demo_weather: WeatherKind,
        reduced: bool,
    ) -> AmbientContext {
        let mut time = clock.context();
        let mut zone = ZonedTime {
            timezone: self.zone.map(|tz| tz.name().to_owned()),
            utc: None,
            offset_seconds: None,
            dst: None,
        };
        if let Some(local) = time.local.filter(|_| time.reliable) {
            if let Some(tz) = self.zone {
                let zoned = if time.source == TimeSource::System {
                    Some(Utc::now().with_timezone(&tz))
                } else if let Some(injected) = clock.injected_utc() {
                    Some(injected.with_timezone(&tz))
                } else {
                    tz.from_local_datetime(&local).single()
                };
                if let Some(at) = zoned {
                    time.local = Some(at.naive_local());
                    zone.utc = Some(at.naive_utc());
                    zone.offset_seconds = Some(at.offset().fix().local_minus_utc());
                    zone.dst = Some(at.offset().dst_offset() != chrono::Duration::zero());
                } else {
                    time.reliable = false;
                }
            } else if let Some(at) = Local.from_local_datetime(&local).single() {
                zone.utc = Some(at.naive_utc());
                zone.offset_seconds = Some(at.offset().fix().local_minus_utc());
            } else {
                time.reliable = false;
            }
        }
        let w = if let Some((source, record)) = self
            .demo_observation
            .as_ref()
            .filter(|_| time.source == TimeSource::Simulated)
        {
            weather(
                zone.utc,
                self.settings.location,
                record.as_ref().filter(|_| *source == WeatherSource::Live),
                record.as_ref().filter(|_| *source != WeatherSource::Live),
                self.settings.weather_ttl_minutes,
            )
        } else if demo_weather != WeatherKind::Unknown {
            WeatherContext {
                source: WeatherSource::Demo,
                observation: None,
                demo: demo_weather,
            }
        } else {
            weather(
                zone.utc,
                self.settings.location,
                self.live.as_ref(),
                self.cached.as_ref(),
                self.settings.weather_ttl_minutes,
            )
        };
        let solar = time
            .local
            .filter(|_| time.reliable && zone.utc.is_some())
            .zip(self.settings.location)
            .and_then(|(at, location)| {
                if let Some((date, old, solar)) = self
                    .solar_cache
                    .get()
                    .filter(|(date, old, _)| *date == at.date() && *old == location)
                {
                    let _ = (date, old);
                    return solar;
                }
                #[cfg(test)]
                self.solar_calculations
                    .set(self.solar_calculations.get() + 1);
                let solar = emotional_core::solar::day(at.date(), location);
                self.solar_cache.set(Some((at.date(), location, solar)));
                solar
            });
        let mut context =
            AmbientContext::resolve_with_solar(time, zone, &self.settings, w, reduced, solar);
        if time.source == TimeSource::Simulated && time.reliable {
            if let Some(season) = self.demo_season {
                context.season = Some(season);
            }
        }
        context
    }
    pub fn cache_path(db: &Path) -> PathBuf {
        db.with_extension("weather-cache.toml")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use emotional_core::solar::Location;
    fn at(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }
    fn runtime(name: &str) -> Runtime {
        Runtime::new(Settings {
            timezone: Some(name.into()),
            ..Default::default()
        })
        .unwrap()
    }
    #[test]
    fn iana_dst_uses_injected_date_not_todays_offset() {
        let r = runtime("Europe/Madrid");
        for (date, offset, dst) in [
            ("2026-03-29T01:30", 3600, false),
            ("2026-03-29T03:30", 7200, true),
            ("2026-10-25T03:30", 3600, false),
        ] {
            let c = r.capture(
                &crate::clock::Clock::new(Some(at(date))),
                WeatherKind::Unknown,
                false,
            );
            assert_eq!(c.zone.offset_seconds, Some(offset));
            assert_eq!(c.zone.dst, Some(dst));
        }
        for date in ["2026-03-29T02:30", "2026-10-25T02:30"] {
            let c = r.capture(
                &crate::clock::Clock::new(Some(at(date))),
                WeatherKind::Unknown,
                false,
            );
            assert!(!c.time.reliable);
            assert!(c.moment.is_none());
        }
    }
    #[test]
    fn timezone_validation_and_fractional_offsets() {
        assert!(Runtime::new(Settings {
            timezone: Some("invented/city".into()),
            ..Default::default()
        })
        .is_err());
        let c = runtime("Asia/Kolkata").capture(
            &crate::clock::Clock::new(Some(at("2026-01-01T00:05"))),
            WeatherKind::Unknown,
            false,
        );
        assert_eq!(c.zone.offset_seconds, Some(19800));
        assert_eq!(c.zone.utc, Some(at("2025-12-31T18:35")));
    }
    #[test]
    fn local_cache_roundtrip_corruption_and_provider_failure() {
        let path =
            std::env::temp_dir().join(format!("emotional-e1-cache-{}.toml", std::process::id()));
        let location = Location {
            latitude: 0.,
            longitude: 0.,
        };
        let o = Observation {
            location,
            observed_utc: at("2026-06-21T12:00"),
            condition: WeatherKind::Rain,
            cloud_percent: 80.,
            precipitation_mm: 1.,
            temperature_c: 18.,
            received_utc: None,
            expires_utc: None,
            rain_mm: None,
            snowfall_cm: None,
        };
        assert!(UnavailableProvider.observe(location).is_err());
        write_cache(&path, &o).unwrap();
        assert_eq!(read_cache(&path).unwrap(), o);
        std::fs::write(&path, "broken response, not an observation").unwrap();
        assert!(read_cache(&path).is_err());
        std::fs::write(&path, "a".repeat(16385)).unwrap();
        assert!(read_cache(&path).is_err());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn demo_does_not_become_live_or_enter_cache() {
        let c = runtime("Etc/UTC").capture(
            &crate::clock::Clock::new(Some(at("2026-06-21T12:00"))),
            WeatherKind::Snow,
            true,
        );
        assert_eq!(c.weather.source, WeatherSource::Demo);
        assert!(c.weather.observation.is_none());
        assert!(c.reduced_motion);
    }
}

#[cfg(test)]
mod runtime_cache_tests {
    use super::*;
    use emotional_core::solar::Location;
    fn clock(s: &str) -> crate::clock::Clock {
        crate::clock::Clock::new(Some(
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap(),
        ))
    }
    #[test]
    fn expiry_clock_correction_missing_city_and_location_change_are_honest() {
        let mut r = Runtime::new(Settings {
            timezone: Some("Etc/UTC".into()),
            location: Some(Location {
                latitude: 0.,
                longitude: 0.,
            }),
            weather_ttl_minutes: 30,
            ..Default::default()
        })
        .unwrap();
        let c = clock("2026-10-20T16:15");
        r.cached = Some(Observation {
            location: r.settings.location.unwrap(),
            observed_utc: chrono::NaiveDateTime::parse_from_str(
                "2026-10-20T16:00",
                "%Y-%m-%dT%H:%M",
            )
            .unwrap(),
            condition: WeatherKind::Rain,
            cloud_percent: 80.,
            precipitation_mm: 1.,
            temperature_c: 18.,
            received_utc: None,
            expires_utc: None,
            rain_mm: None,
            snowfall_cm: None,
        });
        assert_eq!(
            r.capture(&c, WeatherKind::Unknown, false).weather.source,
            WeatherSource::Cached
        );
        for (c, expected) in [
            (clock("2026-10-20T17:00"), WeatherSource::Stale),
            (clock("2026-10-20T15:59"), WeatherSource::Unknown),
        ] {
            assert_eq!(
                r.capture(&c, WeatherKind::Unknown, false).weather.source,
                expected
            );
        }
        r.settings.location = Some(Location {
            latitude: 1.,
            longitude: 0.,
        });
        assert_eq!(
            r.capture(&c, WeatherKind::Unknown, false).weather.source,
            WeatherSource::Unknown
        );
        r.settings.location = None;
        assert_eq!(
            r.capture(&c, WeatherKind::Unknown, false).weather.source,
            WeatherSource::Unknown
        );
    }
}

#[cfg(test)]
mod connected_weather_tests {
    use super::*;
    use crate::open_meteo::{Config, Transport, Worker};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    struct Offline(Arc<AtomicUsize>);
    impl Transport for Offline {
        fn get(&self, _: &str) -> Result<Vec<u8>, &'static str> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Err("offline fixture")
        }
    }
    #[test]
    fn worker_cache_reaches_scene_expires_and_demo_unknown_clock_never_query() {
        let path = std::env::temp_dir().join(format!(
            "emotional-runtime-weather-{}.toml",
            std::process::id()
        ));
        let location = emotional_core::solar::Location {
            latitude: 0.,
            longitude: 0.,
        };
        let config = Config {
            authorized: true,
            location: Some(location),
            ..Default::default()
        };
        let observation = Observation {
            location,
            observed_utc: Utc::now().naive_utc() - chrono::Duration::minutes(1),
            condition: WeatherKind::Snow,
            cloud_percent: 95.,
            precipitation_mm: 1.,
            temperature_c: -2.,
            received_utc: None,
            expires_utc: None,
            rain_mm: Some(0.),
            snowfall_cm: Some(0.8),
        };
        write_selected_cache(&path, &observation, "coords:0,0").unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let mut r = Runtime::new(Settings {
            timezone: Some("Etc/UTC".into()),
            ..Default::default()
        })
        .unwrap();
        r.settings.location = Some(location);
        r.provider = Some(Worker::with_transport(
            config.clone(),
            path.clone(),
            Arc::new(Offline(count.clone())),
        ));
        let clock = crate::clock::Clock::new(None);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while r.cached.is_none() {
            r.weather_tick(&clock, false, false);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let c = r.capture(&clock, WeatherKind::Unknown, false);
        assert_eq!(c.weather.source, WeatherSource::Cached);
        assert_eq!(crate::open_meteo::condition(73), Some(c.weather.kind()));
        assert_eq!(
            emotional_core::ambient_art::AmbientTheme::resolve(&c, &r.settings, false)
                .weather_index(),
            4
        );
        assert_eq!(count.load(Ordering::Relaxed), 0);
        r.live = Some(observation.clone());
        assert_eq!(
            r.capture(&clock, WeatherKind::Unknown, false)
                .weather
                .source,
            WeatherSource::Live
        );
        r.weather_tick(&clock, false, false);
        r.live.as_mut().unwrap().observed_utc -= chrono::Duration::hours(4);
        r.cached.as_mut().unwrap().observed_utc -= chrono::Duration::hours(4);
        assert!(r.weather_tick(&clock, false, false)); // expiry requests repaint, even if focus is held
        let c = r.capture(&clock, WeatherKind::Unknown, false);
        assert_eq!(c.weather.source, WeatherSource::Stale);
        assert_eq!(
            emotional_core::ambient_art::AmbientTheme::resolve(&c, &r.settings, false)
                .weather_index(),
            0
        );
        // A new worker would query if the existing clock/demo/neutral gate were wrong.
        for mode in 0..4 {
            r.provider = Some(Worker::with_transport(
                config.clone(),
                path.with_extension("absent"),
                Arc::new(Offline(count.clone())),
            ));
            let mut gated = clock.clone();
            if mode == 0 {
                gated.unknown();
            }
            if mode == 1 {
                gated.set(Utc::now().naive_utc());
            }
            r.weather_tick(&gated, mode == 2, mode == 3);
        }
        assert_eq!(count.load(Ordering::Relaxed), 0);
        r.configure_weather(
            Config {
                authorized: false,
                location: Some(emotional_core::solar::Location {
                    latitude: 10.,
                    longitude: 20.,
                }),
                ..Default::default()
            },
            path.clone(),
        );
        assert!(r.cached.is_none() && r.live.is_none());
        assert_eq!(
            r.capture(&clock, WeatherKind::Unknown, false)
                .weather
                .kind(),
            WeatherKind::Unknown
        );
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod editorial_config_tests {
    use super::*;
    #[test]
    fn optional_editorial_configuration_preserves_old_settings() {
        let old: Settings = toml::from_str("timezone='Etc/UTC'").unwrap();
        assert!(old.valid());
        let off: Settings = toml::from_str("celebrations=[]").unwrap();
        assert!(off.celebrations.is_empty());
        let custom:Settings=toml::from_str("celebrations=['halloween','christmas','new_year']\n[[editorial_events]]\nid='quiet-day'\nmonth=10\nday=31\nstyle='none'\n").unwrap();
        assert!(custom.valid());
        assert_eq!(
            custom.editorial_events[0].style,
            emotional_core::ambient_art::EditorialEvent::None
        );
        assert!(toml::from_str::<Settings>("celebrations=['invented']").is_err());
    }
}
