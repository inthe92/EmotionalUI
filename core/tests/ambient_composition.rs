use chrono::{Duration, NaiveDateTime};
use emotional_core::{
    ambient::*,
    ambient_art::*,
    environment::*,
    solar::{self, Location},
};
fn context(date: &str) -> AmbientContext {
    let t = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S").unwrap();
    AmbientContext::resolve(
        resolve_time(Some(t), None, None),
        ZonedTime {
            timezone: None,
            utc: Some(t),
            offset_seconds: Some(0),
            dst: Some(false),
        },
        &Settings {
            hemisphere: Some(Hemisphere::North),
            ..Default::default()
        },
        WeatherContext::default(),
        false,
    )
}
fn position(b: Blend) -> f32 {
    b.from as f32 + b.mix
}
#[test]
fn progression_does_not_step_at_civil_band_changes_or_midnight() {
    for time in [
        "2026-12-25T00:00:00",
        "2026-12-25T06:00:00",
        "2026-12-25T09:00:00",
        "2026-12-25T12:00:00",
        "2026-12-25T16:00:00",
        "2026-12-25T19:00:00",
        "2026-12-25T21:00:00",
    ] {
        let mut a = context(time);
        let b = a.artwork_blend();
        a.time.local = a.time.local.map(|t| t - Duration::seconds(1));
        let before = a.artwork_blend();
        assert!(
            (position(b) - position(before)).abs() < 0.001,
            "{time}: {before:?} {b:?}"
        );
    }
}
#[test]
fn every_keyframe_is_exact_and_every_minute_is_bounded() {
    for (i, sec) in CIVIL_KEYS.iter().enumerate() {
        let mut c = context("2026-12-25T00:00:00");
        c.time.local = c.time.local.map(|t| t + Duration::seconds(*sec));
        assert_eq!(
            c.artwork_blend(),
            Blend {
                from: i,
                to: (i + 1) % 7,
                mix: 0.
            }
        );
    }
    let mut c = context("2026-12-25T00:00:00");
    for _ in 0..1440 {
        let b = c.artwork_blend();
        assert!(b.from < 7 && b.to < 7 && (0. ..=1.).contains(&b.mix));
        c.time.local = c.time.local.map(|t| t + Duration::minutes(1));
    }
}
#[test]
fn christmas_is_date_based_optional_and_not_inferred_from_snow() {
    let mut c = context("2026-12-25T16:00:00");
    let enabled = Settings::default().celebrations;
    assert_eq!(c.celebration(&enabled), Some(Celebration::Christmas));
    assert_eq!(c.celebration(&[]), None);
    assert_eq!(c.weather.kind(), WeatherKind::Unknown);
    assert_eq!(c.season, Some(emotional_core::calendar::Season::Winter));
    c.time.local = c.time.local.map(|t| t + Duration::days(1));
    c.weather = WeatherContext {
        source: WeatherSource::Demo,
        demo: WeatherKind::Snow,
        observation: None,
    };
    assert_eq!(c.celebration(&enabled), None);
    assert_eq!(
        context("2027-01-01T12:00:00").celebration(&enabled),
        Some(Celebration::NewYear)
    );
}
#[test]
fn unreliable_clock_never_activates_date_or_daylight() {
    let mut c = context("2026-12-25T16:00:00");
    c.time.reliable = false;
    assert_eq!(c.artwork_blend(), Blend::still(7));
    assert_eq!(c.celebration(&Settings::default().celebrations), None);
}
#[test]
fn solar_progression_uses_utc_events_not_civil_assumptions() {
    let mut c = context("2026-12-25T16:00:00");
    let s = solar::day(
        c.time.local.unwrap().date(),
        Location {
            latitude: 40.,
            longitude: -105.,
        },
    )
    .unwrap();
    c.zone.utc = s.rise;
    c.solar = Some(s);
    assert_eq!(
        c.artwork_blend(),
        Blend {
            from: 1,
            to: 2,
            mix: 0.
        }
    );
    c.zone.utc = s.rise.map(|t| t + Duration::seconds(1));
    assert!(c.artwork_blend().mix < 0.001);
}
#[test]
fn polar_day_stays_daylit_and_celebration_does_not_force_winter() {
    let mut c = context("2026-12-25T16:00:00");
    let s = solar::day(
        c.time.local.unwrap().date(),
        Location {
            latitude: -89.,
            longitude: 0.,
        },
    )
    .unwrap();
    c.solar = Some(s);
    c.moment = Some(DayMoment::EarlyAfternoon);
    assert_eq!(c.artwork_blend(), Blend::still(3));
    c.season = Some(emotional_core::calendar::Season::Summer);
    assert_eq!(
        c.celebration(&Settings::default().celebrations),
        Some(Celebration::Christmas)
    );
    assert_eq!(c.season, Some(emotional_core::calendar::Season::Summer));
}

#[test]
fn four_requested_contexts_resolve_one_theme_without_conflating_layers() {
    use emotional_core::calendar::Season::*;
    for (date, weather, season, event) in [
        (
            "2026-12-25T16:00:00",
            WeatherKind::Snow,
            Winter,
            EditorialEvent::Christmas,
        ),
        (
            "2026-10-31T20:00:00",
            WeatherKind::Rain,
            Autumn,
            EditorialEvent::Halloween,
        ),
        (
            "2026-06-21T10:30:00",
            WeatherKind::Clear,
            Summer,
            EditorialEvent::None,
        ),
        (
            "2026-01-15T03:00:00",
            WeatherKind::Fog,
            Winter,
            EditorialEvent::None,
        ),
    ] {
        let mut c = context(date);
        c.weather = WeatherContext {
            source: WeatherSource::Demo,
            demo: weather,
            observation: None,
        };
        let settings = Settings::default();
        let theme = AmbientTheme::resolve(&c, &settings, false);
        assert_eq!(
            (theme.season, theme.weather, theme.event),
            (Some(season), weather, event)
        );
        assert_eq!(theme.day, c.artwork_blend());
        let neutral = AmbientTheme::resolve(&c, &settings, true);
        assert_eq!(
            (neutral.day, neutral.weather_index(), neutral.event),
            (Blend::still(7), 0, EditorialEvent::None)
        );
    }
}
#[test]
fn explicit_editorial_dates_can_reuse_or_disable_a_style() {
    let c = context("2026-10-31T20:00:00");
    let mut settings = Settings::default();
    assert_eq!(
        AmbientTheme::resolve(&c, &settings, false).event,
        EditorialEvent::Halloween
    );
    settings.editorial_events = vec![EditorialEventRule {
        id: "quiet-evening".into(),
        month: 10,
        day: 31,
        style: EditorialEvent::None,
    }];
    assert!(settings.valid());
    let t = AmbientTheme::resolve(&c, &settings, false);
    assert_eq!(t.event_id, Some("quiet-evening"));
    assert_eq!(t.event, EditorialEvent::None);
    settings.editorial_events[0].style = EditorialEvent::NewYear;
    assert_eq!(
        AmbientTheme::resolve(&c, &settings, false).event,
        EditorialEvent::NewYear
    );
    settings.celebrations.clear();
    settings.editorial_events.clear();
    assert_eq!(
        AmbientTheme::resolve(&c, &settings, false).event,
        EditorialEvent::None
    );
}
#[test]
fn editorial_rules_reject_ambiguous_or_invalid_configuration() {
    let rule = || EditorialEventRule {
        id: "reading-day".into(),
        month: 2,
        day: 29,
        style: EditorialEvent::None,
    };
    let mut settings = Settings {
        editorial_events: vec![rule()],
        ..Default::default()
    };
    assert!(settings.valid()); // Leap-day observance only matches a real 29/2.
    settings.editorial_events.push(rule());
    assert!(!settings.valid());
    settings.editorial_events.pop();
    settings.editorial_events[0].day = 30;
    assert!(!settings.valid());
    settings.editorial_events[0] = rule();
    settings.editorial_events[0].id = "../private".into();
    assert!(!settings.valid());
    settings.editorial_events = (0..33)
        .map(|i| EditorialEventRule {
            id: format!("event-{i}"),
            month: 1,
            day: 1,
            style: EditorialEvent::None,
        })
        .collect();
    assert!(!settings.valid());
}
