use super::*;
use serde_json::{json, Value};
use std::sync::{atomic::AtomicUsize, Mutex};

fn now() -> NaiveDateTime {
    DateTime::from_timestamp(1792497600, 0).unwrap().naive_utc()
}
fn location() -> Location {
    Location {
        latitude: 0.,
        longitude: 0.,
    }
}
fn config() -> Config {
    Config {
        authorized: true,
        location: Some(location()),
        ..Default::default()
    }
}
fn fixture() -> Value {
    json!({"latitude":0.01,"longitude":0.01,"utc_offset_seconds":0,
        "current_units":{"time":"unixtime","interval":"seconds","temperature_2m":"°C","weather_code":"wmo code","cloud_cover":"%","precipitation":"mm","rain":"mm","snowfall":"cm"},
        "current":{"time":1792497600,"interval":900,"temperature_2m":-2.5,"weather_code":73,"cloud_cover":95,"precipitation":1.2,"rain":0,"snowfall":0.8}})
}
struct Fake {
    responses: Mutex<Vec<Result<Vec<u8>, &'static str>>>,
    calls: AtomicUsize,
}
impl Fake {
    fn new(responses: Vec<Result<Vec<u8>, &'static str>>) -> Self {
        Self {
            responses: Mutex::new(responses.into_iter().rev().collect()),
            calls: AtomicUsize::new(0),
        }
    }
}
impl Transport for Fake {
    fn get(&self, _: &str) -> Result<Vec<u8>, &'static str> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        self.responses
            .lock()
            .unwrap()
            .pop()
            .expect("Unexpected network request")
    }
}
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "emotional-weather-{}-{}",
            std::process::id(),
            ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn cache(&self) -> PathBuf {
        self.0.join("library.weather-cache.toml")
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn fetch_with(config: &Config, dir: &Scratch, transport: &Fake) -> Update {
    fetch(
        config,
        &dir.cache(),
        transport,
        now(),
        180,
        &AtomicBool::new(true),
    )
}
#[test]
fn valid_current_preserves_units_requested_location_and_precipitation() {
    let o = parse(
        &serde_json::to_vec(&fixture()).unwrap(),
        location(),
        now(),
        180,
    )
    .unwrap();
    assert_eq!(o.condition, WeatherKind::Snow);
    assert_eq!(o.location, location()); // grid coordinates are not the user's identity
    assert_eq!(o.rain_mm, Some(0.));
    assert_eq!(o.snowfall_cm, Some(0.8));
    assert_eq!(o.temperature_c, -2.5);
    assert_eq!(o.cloud_percent, 95.);
    for (code, kind) in [
        (0, WeatherKind::Clear),
        (2, WeatherKind::Cloud),
        (45, WeatherKind::Fog),
        (48, WeatherKind::Fog),
        (61, WeatherKind::Rain),
        (85, WeatherKind::Snow),
        (97, WeatherKind::Storm),
    ] {
        assert_eq!(condition(code), Some(kind));
    }
    assert_eq!(condition(100), None);
}
#[test]
fn invalid_json_units_dates_codes_ranges_and_grid_are_rejected() {
    for bytes in [
        b"invalid".to_vec(),
        vec![b' '; MAX_BYTES + 1],
        br#"{"error":true,"reason":"bad request"}"#.to_vec(),
    ] {
        assert!(parse(&bytes, location(), now(), 180).is_err());
    }
    for (path, value) in [
        ("/current_units/time", json!("iso8601")),
        ("/current_units/temperature_2m", json!("°F")),
        ("/current_units/snowfall", json!("mm")),
        ("/utc_offset_seconds", json!(3600)),
        ("/current/weather_code", json!(42)),
        ("/current/cloud_cover", json!(101)),
        ("/current/temperature_2m", json!(null)),
        ("/current/rain", json!(-1)),
        ("/current/snowfall", json!(501)),
        ("/current/precipitation", json!(-1)),
        ("/current/time", json!(1792497601_i64)),
        ("/current/time", json!(1792486799_i64)),
        ("/current/time", json!(i64::MAX)),
        ("/current/interval", json!(0)),
        ("/latitude", json!(60)),
    ] {
        let mut v = fixture();
        *v.pointer_mut(path).unwrap() = value;
        assert!(
            parse(&serde_json::to_vec(&v).unwrap(), location(), now(), 180).is_err(),
            "{path}"
        );
    }
}
#[test]
fn fresh_cache_survives_restart_without_any_network_and_expires_safely() {
    let dir = Scratch::new();
    let c = config();
    let f = Fake::new(vec![Ok(serde_json::to_vec(&fixture()).unwrap())]);
    assert_eq!(fetch_with(&c, &dir, &f).source, WeatherSource::Live);
    let offline = Fake::new(vec![]);
    assert_eq!(fetch_with(&c, &dir, &offline).source, WeatherSource::Cached);
    assert_eq!(offline.calls.load(Ordering::Relaxed), 0);
    let offline = Fake::new(vec![Err("unreachable")]);
    let expired = fetch(
        &c,
        &dir.cache(),
        &offline,
        now() + chrono::Duration::minutes(181),
        180,
        &AtomicBool::new(true),
    );
    assert_eq!(expired.source, WeatherSource::Stale);
    assert!(expired.record.is_some()); // retained as history, never current precipitation
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(dir.cache()).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn unavailable_timeout_and_bad_response_do_not_become_weather() {
    for response in [Err("unreachable"), Err("timeout"), Ok(b"bad JSON".to_vec())] {
        let dir = Scratch::new();
        let f = Fake::new(vec![response]);
        let result = fetch_with(&config(), &dir, &f);
        assert_eq!(result.source, WeatherSource::Unknown);
        assert!(result.record.is_none());
        assert!(!dir.cache().exists());
    }
}
#[test]
fn no_consent_no_location_invalid_location_and_cancelled_mean_zero_queries() {
    let dir = Scratch::new();
    let f = Fake::new(vec![]);
    for c in [
        Config::default(),
        Config {
            authorized: false,
            ..config()
        },
        Config {
            authorized: true,
            ..Default::default()
        },
        Config {
            location: Some(Location {
                latitude: 91.,
                longitude: 0.,
            }),
            ..config()
        },
    ] {
        assert_eq!(fetch_with(&c, &dir, &f).source, WeatherSource::Unknown);
    }
    assert_eq!(
        fetch(
            &config(),
            &dir.cache(),
            &f,
            now(),
            180,
            &AtomicBool::new(false)
        )
        .source,
        WeatherSource::Unknown
    );
    assert_eq!(f.calls.load(Ordering::Relaxed), 0);
}
#[test]
fn changed_location_never_uses_previous_cache() {
    let dir = Scratch::new();
    let first = Fake::new(vec![Ok(serde_json::to_vec(&fixture()).unwrap())]);
    fetch_with(&config(), &dir, &first);
    let other = Config {
        location: Some(Location {
            latitude: 10.,
            longitude: 20.,
        }),
        ..config()
    };
    let unavailable = Fake::new(vec![Err("offline")]);
    assert_eq!(
        fetch_with(&other, &dir, &unavailable).source,
        WeatherSource::Unknown
    );
    assert_eq!(unavailable.calls.load(Ordering::Relaxed), 1);
}
#[test]
fn city_is_explicit_unambiguous_exact_and_percent_encoded() {
    let c = Config {
        authorized: true,
        city: Some("Test Town".into()),
        country_code: Some("ZZ".into()),
        ..Default::default()
    };
    let provider = OpenMeteo {
        transport: Fake::new(vec![Ok(
            br#"{"results":[{"name":"Test Town","country_code":"ZZ","latitude":0,"longitude":0}]}"#
                .to_vec(),
        )]),
    };
    assert_eq!(provider.resolve(&c).unwrap(), location());
    assert_eq!(encode("A&B/é"), "A%26B%2F%C3%A9");
    for result in [
        json!({}),
        json!({"results":[{"name":"Test Town","latitude":0,"longitude":0},{"name":"Test Town","latitude":1,"longitude":1}]}),
        json!({"results":[{"name":"Other Town","country_code":"ZZ","latitude":0,"longitude":0}]}),
    ] {
        assert!(OpenMeteo {
            transport: Fake::new(vec![Ok(serde_json::to_vec(&result).unwrap())])
        }
        .resolve(&c)
        .is_err());
    }
}
#[test]
fn worker_does_not_block_ui_and_throttles_repeated_ticks() {
    let dir = Scratch::new();
    let f = Arc::new(Fake::new(vec![Ok(serde_json::to_vec(&fixture()).unwrap())]));
    let mut w = Worker::with_transport(config(), dir.cache(), f.clone());
    for _ in 0..10 {
        assert!(w.poll(false, now(), 180).is_none());
    } // DEMO/unknown clock/neutral
    assert_eq!(f.calls.load(Ordering::Relaxed), 0);
    assert!(w.poll(true, now(), 180).is_none()); // scheduling only, not waiting for I/O
    let deadline = Instant::now() + Duration::from_secs(2);
    let result = loop {
        if let Some(result) = w.poll(true, now(), 180) {
            break result;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    };
    assert_eq!(result.1, WeatherSource::Live);
    for _ in 0..1000 {
        assert!(w.poll(true, now(), 180).is_none());
    }
    assert_eq!(f.calls.load(Ordering::Relaxed), 1);
    let failed = Arc::new(Fake::new(vec![Err("offline")]));
    let mut w = Worker::with_transport(config(), dir.0.join("other-cache"), failed.clone());
    w.poll(true, now(), 180);
    while w.poll(true, now(), 180).is_none() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    for _ in 0..1000 {
        assert!(w.poll(true, now(), 180).is_none());
    }
    assert_eq!(failed.calls.load(Ordering::Relaxed), 1);
}
#[test]
fn private_configuration_is_bounded_validated_and_not_public() {
    let dir = Scratch::new();
    let p = dir.0.join("test.weather-private.toml");
    std::fs::write(
        &p,
        "authorized = false\n[location]\nlatitude = 0\nlongitude = 0\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(Config::read(&p).is_err());
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let c = Config::read(&p).unwrap();
    assert!(!c.enabled());
    for body in [
        "authorized = true\ncity = 'Test'\n[location]\nlatitude = 0\nlongitude = 0",
        "location = 'secret-value'",
        &" ".repeat(8193),
    ] {
        std::fs::write(&p, body).unwrap();
        let error = Config::read(&p).err().unwrap();
        assert!(!error.contains("secret-value"));
    }
    assert!(Config::read(&dir.0.join("public.toml")).is_err());
}
