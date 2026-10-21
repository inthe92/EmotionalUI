//! Optional modelled current weather. All I/O runs on a short-lived worker.
//! Never logs coordinates, response bodies, city names or request URLs.
use crate::ambient::{read_selected_cache, write_selected_cache};
use chrono::{DateTime, NaiveDateTime};
use emotional_core::{
    ambient::{weather, Observation, WeatherKind, WeatherSource},
    solar::Location,
};
use serde::Deserialize;
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
        Arc,
    },
    time::{Duration, Instant},
};

const MAX_BYTES: usize = 65536;
const MIN_REFRESH: Duration = Duration::from_secs(15 * 60);

/// Kept in an ignored, owner-only *.weather-private.toml, never the public theme.
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub authorized: bool,
    pub location: Option<Location>,
    pub city: Option<String>,
    pub country_code: Option<String>,
}
impl Config {
    pub fn read(path: &Path) -> Result<Self, &'static str> {
        if !path.to_string_lossy().ends_with(".weather-private.toml") {
            return Err("Weather configuration requires an ignored .weather-private.toml filename");
        }
        let file =
            std::fs::File::open(path).map_err(|_| "Cannot open private weather configuration")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if file
                .metadata()
                .map_err(|_| "Cannot inspect weather configuration")?
                .permissions()
                .mode()
                & 0o077
                != 0
            {
                return Err("Private weather configuration requires permissions 0600");
            }
        }
        let mut text = String::new();
        file.take(8193)
            .read_to_string(&mut text)
            .map_err(|_| "Cannot read weather configuration")?;
        if text.len() > 8192 {
            return Err("Weather configuration too large");
        }
        let config: Self =
            toml::from_str(&text).map_err(|_| "Invalid private weather configuration")?;
        if !config.valid() {
            return Err("Choose coordinates OR a city; check location and country code");
        }
        Ok(config)
    }
    pub fn valid(&self) -> bool {
        self.location.is_none_or(Location::valid)
            && !(self.location.is_some() && self.city.is_some())
            && self.city.as_ref().is_none_or(|s| {
                !s.trim().is_empty() && s.len() <= 160 && !s.chars().any(char::is_control)
            })
            && self.country_code.as_ref().is_none_or(|s| {
                self.city.is_some() && s.len() == 2 && s.bytes().all(|b| b.is_ascii_uppercase())
            })
    }
    fn enabled(&self) -> bool {
        self.valid() && self.authorized && (self.location.is_some() || self.city.is_some())
    }
    fn key(&self) -> String {
        match self.location {
            Some(l) => format!("coords:{},{}", l.latitude, l.longitude),
            None => format!(
                "city:{}:{}",
                self.city.as_deref().unwrap_or_default(),
                self.country_code.as_deref().unwrap_or_default()
            ),
        }
    }
}

/// Injected by tests; production uses the system curl, without a shell or TLS crate.
pub trait Transport: Send + Sync {
    fn get(&self, url: &str) -> Result<Vec<u8>, &'static str>;
}
pub struct Curl;
impl Transport for Curl {
    fn get(&self, url: &str) -> Result<Vec<u8>, &'static str> {
        // URL goes through stdin, not the process list. No curlrc, redirects,
        // stderr payloads or insecure TLS; each call has a four-second deadline.
        let mut child = Command::new("/usr/bin/curl")
            .args([
                "--disable",
                "--silent",
                "--fail",
                "--proto",
                "=https",
                "--connect-timeout",
                "2",
                "--max-time",
                "4",
                "--max-filesize",
                "65536",
                "--config",
                "-",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Weather transport unavailable (system curl required)")?;
        let result = (|| {
            let mut input = child
                .stdin
                .take()
                .ok_or("Weather transport input unavailable")?;
            writeln!(input, "url = \"{url}\"").map_err(|_| "Weather transport failed")?;
            drop(input);
            let mut bytes = Vec::new();
            child
                .stdout
                .take()
                .ok_or("Weather transport output unavailable")?
                .take((MAX_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|_| "Weather transport failed")?;
            if bytes.len() > MAX_BYTES {
                return Err("Weather response too large");
            }
            if !child
                .wait()
                .map_err(|_| "Weather transport failed")?
                .success()
            {
                return Err("Weather service unavailable or timed out");
            }
            Ok(bytes)
        })();
        if result.is_err() {
            let _ = child.kill();
            let _ = child.wait();
        }
        result
    }
}

pub struct OpenMeteo<T = Curl> {
    pub transport: T,
}
impl<T: Transport> crate::ambient::WeatherProvider for OpenMeteo<T> {
    fn observe(&self, location: Location) -> Result<Observation, String> {
        self.observe(location, chrono::Utc::now().naive_utc(), 180)
            .map_err(str::to_owned)
    }
}
impl<T: Transport> OpenMeteo<T> {
    pub fn observe(
        &self,
        location: Location,
        now: NaiveDateTime,
        ttl: u32,
    ) -> Result<Observation, &'static str> {
        if !location.valid() {
            return Err("Invalid selected location");
        }
        let url = format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code,cloud_cover,precipitation,rain,snowfall&temperature_unit=celsius&precipitation_unit=mm&timeformat=unixtime&timezone=GMT&forecast_days=1", location.latitude, location.longitude);
        let started = Instant::now();
        let body = self.transport.get(&url)?;
        let received = now + chrono::Duration::seconds(started.elapsed().as_secs() as i64);
        parse(&body, location, received, ttl)
    }
    fn resolve(&self, config: &Config) -> Result<Location, &'static str> {
        if let Some(location) = config.location {
            return Ok(location);
        }
        let name = config.city.as_deref().ok_or("No location selected")?;
        let country = config
            .country_code
            .as_deref()
            .map(|s| format!("&countryCode={s}"))
            .unwrap_or_default();
        let bytes = self.transport.get(&format!("https://geocoding-api.open-meteo.com/v1/search?name={}&count=2&language=en&format=json{country}", encode(name)))?;
        #[derive(Deserialize)]
        struct Cities {
            #[serde(default)]
            results: Vec<City>,
        }
        #[derive(Deserialize)]
        struct City {
            name: String,
            country_code: Option<String>,
            latitude: f64,
            longitude: f64,
        }
        if bytes.len() > MAX_BYTES {
            return Err("City response too large");
        }
        let cities: Cities = serde_json::from_slice(&bytes).map_err(|_| "Invalid city response")?;
        // Never choose the first approximate/ambiguous match. The user can use
        // coordinates or a more specific city + country instead.
        if cities.results.len() != 1 {
            return Err("City missing or ambiguous; configure coordinates");
        }
        let city = &cities.results[0];
        if city.name.to_lowercase() != name.to_lowercase()
            || config
                .country_code
                .as_ref()
                .is_some_and(|code| city.country_code.as_ref() != Some(code))
        {
            return Err("City match is not exact; configure coordinates");
        }
        let location = Location {
            latitude: city.latitude,
            longitude: city.longitude,
        };
        location
            .valid()
            .then_some(location)
            .ok_or("Invalid city coordinates")
    }
}
fn encode(text: &str) -> String {
    text.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[derive(Deserialize)]
struct Response {
    latitude: f64,
    longitude: f64,
    utc_offset_seconds: i32,
    current: Current,
    current_units: Units,
}
#[derive(Deserialize)]
struct Current {
    time: i64,
    interval: u32,
    temperature_2m: f64,
    weather_code: u8,
    cloud_cover: f64,
    precipitation: f64,
    rain: f64,
    snowfall: f64,
}
#[derive(Deserialize)]
struct Units {
    time: String,
    interval: String,
    temperature_2m: String,
    weather_code: String,
    cloud_cover: String,
    precipitation: String,
    rain: String,
    snowfall: String,
}
pub fn condition(code: u8) -> Option<WeatherKind> {
    Some(match code {
        0 | 1 => WeatherKind::Clear,
        2 | 3 => WeatherKind::Cloud,
        45 | 48 => WeatherKind::Fog,
        51 | 53 | 55 | 56 | 57 | 61 | 63 | 65 | 66 | 67 | 80 | 81 | 82 => WeatherKind::Rain,
        71 | 73 | 75 | 77 | 85 | 86 => WeatherKind::Snow,
        95 | 96 | 97 | 99 => WeatherKind::Storm,
        _ => return None,
    })
}
pub fn parse(
    bytes: &[u8],
    location: Location,
    now: NaiveDateTime,
    ttl: u32,
) -> Result<Observation, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err("Weather response too large");
    }
    let r: Response = serde_json::from_slice(bytes).map_err(|_| "Invalid weather JSON")?;
    let u = r.current_units;
    if u.time != "unixtime"
        || u.interval != "seconds"
        || u.temperature_2m != "°C"
        || u.weather_code != "wmo code"
        || u.cloud_cover != "%"
        || u.precipitation != "mm"
        || u.rain != "mm"
        || u.snowfall != "cm"
        || r.utc_offset_seconds != 0
    {
        return Err("Unexpected weather units or timezone");
    }
    // Models return their grid point, not the exact requested coordinates.
    let longitude_delta = ((r.longitude - location.longitude + 540.).rem_euclid(360.) - 180.).abs();
    let grid = Location {
        latitude: r.latitude,
        longitude: r.longitude,
    };
    if !grid.valid()
        || (r.latitude - location.latitude).abs() > 1.
        || longitude_delta > 1.
        || !(1..=3600).contains(&r.current.interval)
    {
        return Err("Unexpected weather grid or interval");
    }
    let c = r.current;
    let o = Observation {
        location,
        observed_utc: DateTime::from_timestamp(c.time, 0)
            .ok_or("Invalid weather date")?
            .naive_utc(),
        received_utc: Some(now),
        expires_utc: DateTime::from_timestamp(c.time, 0).and_then(|t| {
            t.naive_utc()
                .checked_add_signed(chrono::Duration::minutes(i64::from(ttl)))
        }),
        condition: condition(c.weather_code).ok_or("Unknown WMO code")?,
        cloud_percent: c.cloud_cover,
        precipitation_mm: c.precipitation,
        temperature_c: c.temperature_2m,
        rain_mm: Some(c.rain),
        snowfall_cm: Some(c.snowfall),
    };
    if weather(Some(now), Some(location), Some(&o), None, ttl).source != WeatherSource::Live {
        return Err("Invalid, future or expired weather record");
    }
    Ok(o)
}

struct Update {
    record: Option<Observation>,
    source: WeatherSource,
}
/// One bounded in-flight request. Polling is nonblocking; disabled means no thread,
/// cache I/O, city lookup or network. DEMO/unreliable clocks never initiate I/O.
pub struct Worker {
    config: Config,
    cache: PathBuf,
    transport: Arc<dyn Transport>,
    pending: Option<Receiver<Update>>,
    next: Instant,
    active: Arc<AtomicBool>,
}
impl Worker {
    pub fn new(config: Config, cache: PathBuf) -> Self {
        Self::with_transport(config, cache, Arc::new(Curl))
    }
    pub(crate) fn with_transport(
        config: Config,
        cache: PathBuf,
        transport: Arc<dyn Transport>,
    ) -> Self {
        Self {
            config,
            cache,
            transport,
            pending: None,
            next: Instant::now(),
            active: Arc::new(AtomicBool::new(true)),
        }
    }
    pub fn enabled(&self) -> bool {
        self.config.enabled()
    }
    pub fn configured_location(&self) -> Option<Location> {
        self.config.location
    }
    pub fn poll(
        &mut self,
        allowed: bool,
        now: NaiveDateTime,
        ttl: u32,
    ) -> Option<(Option<Observation>, WeatherSource)> {
        if !self.enabled() || !allowed {
            return None;
        }
        if let Some(receiver) = &self.pending {
            match receiver.try_recv() {
                Ok(update) => {
                    self.pending = None;
                    let remaining = update
                        .record
                        .as_ref()
                        .map(|o| i64::from(ttl) * 60 - (now - o.observed_utc).num_seconds())
                        .unwrap_or(0)
                        .max(0) as u64;
                    let delay = if update.source == WeatherSource::Cached {
                        Duration::from_secs(remaining.max(1))
                    } else {
                        Duration::from_secs(remaining).max(MIN_REFRESH)
                    };
                    self.next = Instant::now() + delay;
                    return Some((update.record, update.source));
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.pending = None;
                    self.next = Instant::now() + MIN_REFRESH;
                }
                Err(mpsc::TryRecvError::Empty) => (),
            }
        }
        if self.pending.is_none() && Instant::now() >= self.next {
            let (sender, receiver) = mpsc::sync_channel(1);
            let config = self.config.clone();
            let cache = self.cache.clone();
            let transport = self.transport.clone();
            let active = self.active.clone();
            self.next = Instant::now() + MIN_REFRESH;
            if std::thread::Builder::new()
                .name("weather".into())
                .spawn(move || {
                    let update = fetch(&config, &cache, transport.as_ref(), now, ttl, &active);
                    let _ = sender.send(update);
                })
                .is_ok()
            {
                self.pending = Some(receiver);
            }
        }
        None
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Relaxed);
    }
}
impl<T: Transport + ?Sized> Transport for &T {
    fn get(&self, url: &str) -> Result<Vec<u8>, &'static str> {
        (**self).get(url)
    }
}
fn fetch(
    config: &Config,
    cache: &Path,
    transport: &dyn Transport,
    now: NaiveDateTime,
    ttl: u32,
    active: &AtomicBool,
) -> Update {
    let missing = || Update {
        record: None,
        source: WeatherSource::Unknown,
    };
    if !config.enabled() || !active.load(Ordering::Relaxed) {
        return missing();
    }
    let key = config.key();
    let previous = read_selected_cache(cache, &key)
        .ok()
        .filter(|record| config.location.is_none_or(|l| l == record.location));
    if let Some(record) = previous.as_ref() {
        if weather(Some(now), Some(record.location), None, Some(record), ttl).source
            == WeatherSource::Cached
        {
            return Update {
                record: Some(record.clone()),
                source: WeatherSource::Cached,
            };
        }
    }
    let fallback = || {
        let context = weather(
            Some(now),
            previous.as_ref().map(|o| o.location),
            None,
            previous.as_ref(),
            ttl,
        );
        Update {
            record: previous.clone(),
            source: context.source,
        }
    };
    let provider = OpenMeteo { transport };
    let Ok(location) = provider.resolve(config) else {
        return fallback();
    };
    if !active.load(Ordering::Relaxed) {
        return missing();
    }
    let Ok(record) = provider.observe(location, now, ttl) else {
        return fallback();
    };
    if !active.load(Ordering::Relaxed) {
        return missing();
    }
    // Failure to persist is optional: valid LIVE data can still be used this run.
    let _ = write_selected_cache(cache, &record, &key);
    Update {
        record: Some(record),
        source: WeatherSource::Live,
    }
}

#[cfg(test)]
mod tests;
