use chrono::{Duration, NaiveDate, NaiveDateTime};
use emotional_core::{
    ambient::*,
    calendar::Season,
    environment::*,
    solar::{self, Daylight, Location},
};
fn at(s: &str) -> NaiveDateTime {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
}
fn zone(time: NaiveDateTime) -> ZonedTime {
    ZonedTime {
        timezone: Some("Etc/UTC".into()),
        utc: Some(time),
        offset_seconds: Some(0),
        dst: Some(false),
    }
}
#[test]
fn north_south_boundaries_and_leap_year() {
    for (date, north, south) in [
        ("2024-02-29T23:59", Season::Winter, Season::Summer),
        ("2024-03-01T00:00", Season::Spring, Season::Autumn),
        ("2026-06-01T00:00", Season::Summer, Season::Winter),
        ("2026-09-01T00:00", Season::Autumn, Season::Spring),
        ("2026-12-01T00:00", Season::Winter, Season::Summer),
        ("2027-01-01T00:00", Season::Winter, Season::Summer),
    ] {
        assert_eq!(season(at(date).date(), Hemisphere::North), north);
        assert_eq!(season(at(date).date(), Hemisphere::South), south);
    }
}
#[test]
fn no_location_retains_all_fixed_bands() {
    let settings = Settings::default();
    for minute in 0..1440 {
        let now = at("2026-10-20T00:00") + Duration::minutes(minute);
        let c = AmbientContext::resolve(
            resolve_time(Some(now), None, None),
            zone(now),
            &settings,
            WeatherContext::default(),
            false,
        );
        assert_eq!(c.moment, Some(DayMoment::at(now)));
        assert!(c.solar.is_none());
    }
}
#[test]
fn unreliable_clock_has_no_current_season_or_solar_or_weather() {
    let time = resolve_time(None, None, Some(at("2026-10-20T00:00")));
    let settings = Settings {
        location: Some(Location {
            latitude: 40.,
            longitude: -105.,
        }),
        ..Default::default()
    };
    let c = AmbientContext::resolve(
        time,
        zone(at("2026-10-20T00:00")),
        &settings,
        WeatherContext {
            source: WeatherSource::Demo,
            demo: WeatherKind::Snow,
            observation: None,
        },
        true,
    );
    assert!(c.moment.is_none() && c.solar.is_none() && c.season.is_none());
    assert_eq!(c.weather.source, WeatherSource::Unknown);
    assert!(c.reduced_motion);
}
#[test]
fn solar_matches_usno_published_references_within_five_minutes() {
    // Retrieved 2026-10-20 from USNO /api/rstt/oneday. Offset already includes DST.
    // https://aa.usno.navy.mil/data/api ; no network is called by this test.
    for (date, lat, lon, offset, times) in [
        (
            "2026-06-21",
            40.,
            -105.,
            -6,
            ["04:59", "05:31", "13:02", "20:32", "21:05"],
        ),
        (
            "2026-12-21",
            40.,
            -105.,
            -7,
            ["06:48", "07:18", "11:58", "16:38", "17:09"],
        ),
        (
            "2026-03-20",
            0.,
            0.,
            0,
            ["05:44", "06:04", "12:07", "18:11", "18:31"],
        ),
    ] {
        let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();
        let s = solar::day(
            d,
            Location {
                latitude: lat,
                longitude: lon,
            },
        )
        .unwrap();
        for (actual, expected) in [s.dawn, s.rise, Some(s.noon), s.set, s.dusk]
            .into_iter()
            .zip(times)
        {
            let local = actual.unwrap() + Duration::hours(offset);
            let expected = at(&format!("{date}T{expected}"));
            assert!(
                (local - expected).num_seconds().abs() <= 300,
                "{date} {local} vs {expected}"
            );
        }
    }
}
#[test]
fn polar_days_and_nights_have_no_fabricated_sunrise() {
    for (date, lat, status) in [
        ("2026-06-21T12:00", 89., Daylight::PolarDay),
        ("2026-12-21T12:00", 89., Daylight::PolarNight),
        ("2026-12-21T12:00", -89., Daylight::PolarDay),
        ("2026-06-21T12:00", -90., Daylight::PolarNight),
    ] {
        let s = solar::day(
            at(date).date(),
            Location {
                latitude: lat,
                longitude: 0.,
            },
        )
        .unwrap();
        assert_eq!(s.daylight, status);
        assert!(s.rise.is_none() && s.set.is_none());
    }
}
#[test]
fn invalid_locations_are_not_used() {
    for latitude in [f64::NAN, f64::INFINITY, 91., -91.] {
        assert!(solar::day(
            at("2026-06-21T12:00").date(),
            Location {
                latitude,
                longitude: 0.
            }
        )
        .is_none());
    }
}
#[test]
fn utc_events_keep_date_rollover_at_dateline() {
    let date = at("2026-12-31T12:00").date();
    let s = solar::day(
        date,
        Location {
            latitude: 0.,
            longitude: 179.,
        },
    )
    .unwrap();
    assert!(s.rise.unwrap().date() < date);
    assert!(
        s.dawn.unwrap() < s.rise.unwrap()
            && s.rise.unwrap() < s.noon
            && s.noon < s.set.unwrap()
            && s.set.unwrap() < s.dusk.unwrap()
    );
}
#[test]
fn all_seven_solar_bands_are_reachable_on_a_normal_day() {
    let now = at("2026-06-21T00:00");
    let s = solar::day(
        now.date(),
        Location {
            latitude: 40.,
            longitude: 0.,
        },
    )
    .unwrap();
    let mut reached = [false; 7];
    for minute in 0..1440 {
        let at = now + Duration::minutes(minute);
        reached[s.moment(at, at).index()] = true;
    }
    assert_eq!(reached, [true; 7]);
}
fn observation() -> Observation {
    Observation {
        location: Location {
            latitude: 0.,
            longitude: 0.,
        },
        observed_utc: at("2026-06-21T12:00"),
        condition: WeatherKind::Rain,
        cloud_percent: 80.,
        precipitation_mm: 1.,
        temperature_c: 18.,
        received_utc: None,
        expires_utc: None,
        rain_mm: None,
        snowfall_cm: None,
    }
}
#[test]
fn weather_live_cached_expired_and_missing_location() {
    let o = observation();
    let now = o.observed_utc;
    assert_eq!(
        weather(Some(now), Some(o.location), Some(&o), Some(&o), 180).source,
        WeatherSource::Live
    );
    assert_eq!(
        weather(
            Some(now + Duration::minutes(180)),
            Some(o.location),
            None,
            Some(&o),
            180
        )
        .source,
        WeatherSource::Cached
    );
    for (now, expected) in [
        (now - Duration::seconds(1), WeatherSource::Unknown),
        (now + Duration::minutes(181), WeatherSource::Stale),
    ] {
        assert_eq!(
            weather(Some(now), Some(o.location), None, Some(&o), 180).source,
            expected
        );
    }
    assert_eq!(
        weather(Some(now), None, None, Some(&o), 180).source,
        WeatherSource::Unknown
    );
    assert_eq!(
        weather(None, Some(o.location), None, Some(&o), 180).source,
        WeatherSource::Unknown
    );
    assert_eq!(
        weather(
            Some(now),
            Some(Location {
                latitude: 1.,
                longitude: 0.
            }),
            None,
            Some(&o),
            180
        )
        .source,
        WeatherSource::Unknown
    );
}

#[test]
fn stale_never_paints_precipitation_and_receipt_and_expiry_are_checked() {
    let mut o = observation();
    let now = o.observed_utc;
    o.received_utc = Some(now);
    o.expires_utc = Some(now + Duration::minutes(30));
    let cached = weather(Some(now), Some(o.location), None, Some(&o), 180);
    assert_eq!(cached.source, WeatherSource::Cached);
    let stale = weather(
        Some(now + Duration::minutes(31)),
        Some(o.location),
        None,
        Some(&o),
        180,
    );
    assert_eq!(stale.source, WeatherSource::Stale);
    assert_eq!(stale.kind(), WeatherKind::Unknown);
    assert!(stale.observation.is_none());
    o.received_utc = Some(now + Duration::minutes(1));
    assert_eq!(
        weather(Some(now), Some(o.location), None, Some(&o), 180).source,
        WeatherSource::Unknown
    );
    o.received_utc = Some(now - Duration::seconds(1));
    assert!(!o.valid());
}

#[test]
fn every_meteorological_boundary_in_both_hemispheres() {
    for (before, after, old, new) in [
        (
            "2026-02-28T23:59",
            "2026-03-01T00:00",
            Season::Winter,
            Season::Spring,
        ),
        (
            "2026-05-31T23:59",
            "2026-06-01T00:00",
            Season::Spring,
            Season::Summer,
        ),
        (
            "2026-08-31T23:59",
            "2026-09-01T00:00",
            Season::Summer,
            Season::Autumn,
        ),
        (
            "2026-11-30T23:59",
            "2026-12-01T00:00",
            Season::Autumn,
            Season::Winter,
        ),
    ] {
        assert_eq!(season(at(before).date(), Hemisphere::North), old);
        assert_eq!(season(at(after).date(), Hemisphere::North), new);
        assert_ne!(season(at(after).date(), Hemisphere::South), new);
    }
}
#[test]
fn weather_invalid_numbers_and_units_are_rejected() {
    let mut o = observation();
    o.temperature_c = f64::NAN;
    assert!(!o.valid());
    o = observation();
    o.cloud_percent = 101.;
    assert!(!o.valid());
    o = observation();
    o.precipitation_mm = -1.;
    assert!(!o.valid());
    assert!(!Settings {
        weather_ttl_minutes: 0,
        ..Default::default()
    }
    .valid());
}
#[test]
fn no_hemisphere_or_location_does_not_guess_a_season() {
    let now = at("2026-06-21T12:00");
    let c = AmbientContext::resolve(
        resolve_time(Some(now), None, None),
        zone(now),
        &Settings::default(),
        WeatherContext::default(),
        false,
    );
    assert!(c.season.is_none());
    let settings = Settings {
        location: Some(Location {
            latitude: -35.,
            longitude: 0.,
        }),
        ..Default::default()
    };
    let c = AmbientContext::resolve(
        resolve_time(Some(now), None, None),
        zone(now),
        &settings,
        WeatherContext::default(),
        false,
    );
    assert_eq!(c.hemisphere, Some(Hemisphere::South));
    assert_eq!(c.season, Some(Season::Winter));
}
#[test]
fn high_latitude_continuous_twilight_is_not_painted_as_dark_night() {
    let date = at("2026-06-21T00:00");
    let s = solar::day(
        date.date(),
        Location {
            latitude: 65.,
            longitude: 0.,
        },
    )
    .unwrap();
    assert_eq!(s.daylight, Daylight::Normal);
    assert!(s.dawn.is_none() && s.dusk.is_none());
    assert_eq!(s.moment(date, date), DayMoment::Dusk);
}
