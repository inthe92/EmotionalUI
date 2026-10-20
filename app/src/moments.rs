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
#[cfg(test)]
mod tests {
    use super::*;
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
