use super::*;
use emotional_core::{ambient_art::AmbientTheme, environment::SceneResolver, solar::Location};
fn runtime() -> Runtime {
    Runtime::new(Settings {
        timezone: Some("Europe/Madrid".into()),
        location: Some(Location {
            latitude: 40.,
            longitude: -3.,
        }),
        ..Default::default()
    })
    .unwrap()
}
#[test]
fn madrid_dst_explicit_offsets_resolve_both_repeated_hours_and_the_spring_gap() {
    let r = runtime();
    for (at, offset, local) in [
        ("2026-03-29T00:30:00Z", 3600, "2026-03-29 01:30"),
        ("2026-03-29T01:30:00Z", 7200, "2026-03-29 03:30"),
        ("2026-10-25T00:30:00Z", 7200, "2026-10-25 02:30"),
        ("2026-10-25T01:30:00Z", 3600, "2026-10-25 02:30"),
    ] {
        let clock = r.demo_clock(at).unwrap();
        let c = r.capture(&clock, WeatherKind::Unknown, true);
        assert!(c.time.reliable);
        assert_eq!(c.zone.offset_seconds, Some(offset));
        assert_eq!(
            c.time.local.unwrap().format("%Y-%m-%d %H:%M").to_string(),
            local
        );
    }
    for local in ["2026-03-29T02:30", "2026-10-25T02:30"] {
        let c = r.capture(&r.demo_clock(local).unwrap(), WeatherKind::Unknown, true);
        assert!(!c.time.reliable);
        assert_eq!(SceneResolver::resolve(&c.scene(), false).id, "neutro");
    }
}
#[test]
fn ephemeris_is_cached_until_date_or_location_changes() {
    let mut r = runtime();
    let first = r.demo_clock("2024-02-29T12:00").unwrap();
    for _ in 0..1000 {
        r.capture(&first, WeatherKind::Unknown, true);
    }
    assert_eq!(r.solar_calculations.get(), 1);
    r.capture(
        &r.demo_clock("2024-02-29T18:00").unwrap(),
        WeatherKind::Unknown,
        true,
    );
    assert_eq!(r.solar_calculations.get(), 1);
    r.capture(
        &r.demo_clock("2024-03-01T00:01").unwrap(),
        WeatherKind::Unknown,
        true,
    );
    assert_eq!(r.solar_calculations.get(), 2);
    r.settings.location = Some(Location {
        latitude: 45.,
        longitude: 0.,
    });
    r.capture(&first, WeatherKind::Unknown, true);
    assert_eq!(r.solar_calculations.get(), 3);
    let mut bad = first;
    bad.unknown();
    r.capture(&bad, WeatherKind::Snow, true);
    assert_eq!(r.solar_calculations.get(), 3);
}
#[test]
fn midnight_and_resume_resolve_current_context_without_replaying_old_dates() {
    let r = runtime();
    for (before, after) in [
        ("2026-06-21T23:59", "2026-06-22T00:01"),
        ("2026-12-31T23:59", "2027-01-01T00:01"),
    ] {
        let a = r
            .capture(&r.demo_clock(before).unwrap(), WeatherKind::Unknown, true)
            .artwork_blend();
        let b = r
            .capture(&r.demo_clock(after).unwrap(), WeatherKind::Unknown, true)
            .artwork_blend();
        assert_eq!((a.from, a.to), (b.from, b.to));
        assert!((a.mix - b.mix).abs() < 0.02);
    }
    let c = r.capture(
        &r.demo_clock("2027-07-01T16:00").unwrap(),
        WeatherKind::Unknown,
        true,
    );
    assert_eq!(c.season, Some(emotional_core::calendar::Season::Summer));
    assert_eq!(c.solar.unwrap().noon.date(), c.time.local.unwrap().date());
}
#[test]
fn demo_weather_cannot_leak_to_real_clock_or_cache() {
    let mut r = runtime();
    let clock = r.demo_clock("2026-12-25T16:00").unwrap();
    let now = r
        .capture(&clock, WeatherKind::Unknown, true)
        .zone
        .utc
        .unwrap();
    let record = Observation {
        location: r.settings.location.unwrap(),
        observed_utc: now,
        received_utc: Some(now),
        expires_utc: Some(now + chrono::Duration::hours(3)),
        condition: WeatherKind::Snow,
        cloud_percent: 80.,
        precipitation_mm: 1.,
        temperature_c: -1.,
        rain_mm: Some(0.),
        snowfall_cm: Some(1.),
    };
    r.set_demo_observation(WeatherSource::Cached, Some(record));
    assert_eq!(
        r.capture(&clock, WeatherKind::Unknown, true).weather.source,
        WeatherSource::Cached
    );
    assert_eq!(
        r.capture(&crate::clock::Clock::new(None), WeatherKind::Unknown, true)
            .weather
            .source,
        WeatherSource::Unknown
    );
    assert!(r.cached.is_none() && r.live.is_none());
    let mut bad = clock;
    bad.unknown();
    let c = r.capture(&bad, WeatherKind::Unknown, true);
    let t = AmbientTheme::resolve(&c, &r.settings, false);
    assert_eq!(t.day.from, 7);
    assert_eq!(t.weather_index(), 0);
    assert!(t.season.is_none());
}
