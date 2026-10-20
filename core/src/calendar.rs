use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}
/// Northern hemisphere; region-specific calendars will be supplied by adapters.
pub fn season(date: NaiveDate) -> Season {
    match date.month() {
        3..=5 => Season::Spring,
        6..=8 => Season::Summer,
        9..=11 => Season::Autumn,
        _ => Season::Winter,
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Holiday {
    pub month: u32,
    pub day: u32,
    pub region: String,
    pub label: String,
}
pub fn holiday<'a>(date: NaiveDate, region: &str, rules: &'a [Holiday]) -> Option<&'a str> {
    let country = region.split('-').next().unwrap_or(region);
    rules
        .iter()
        .filter(|r| r.month == date.month() && r.day == date.day())
        .filter(|r| r.region == region || r.region == country || r.region == "*")
        .min_by_key(|r| {
            if r.region == region {
                0
            } else if r.region == country {
                1
            } else {
                2
            }
        })
        .map(|r| r.label.as_str())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seasonal_boundaries() {
        for (m, s) in [
            (2, Season::Winter),
            (3, Season::Spring),
            (6, Season::Summer),
            (9, Season::Autumn),
            (12, Season::Winter),
        ] {
            assert_eq!(season(NaiveDate::from_ymd_opt(2026, m, 1).unwrap()), s);
        }
    }
    #[test]
    fn holidays_are_regional() {
        let rules = [Holiday {
            month: 10,
            day: 31,
            region: "ES".into(),
            label: "Halloween".into(),
        }];
        let d = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        assert_eq!(holiday(d, "ES", &rules), Some("Halloween"));
        assert_eq!(holiday(d, "JP", &rules), None);
    }
}

impl Season {
    pub fn label(self) -> &'static str {
        match self {
            Self::Spring => "Primavera",
            Self::Summer => "Verano",
            Self::Autumn => "Otoño",
            Self::Winter => "Invierno",
        }
    }
}
#[cfg(test)]
mod boundaries {
    use super::*;
    #[test]
    fn new_year_and_leap_day() {
        let d = NaiveDate::from_ymd_opt(2024, 2, 29).unwrap();
        assert_eq!(season(d), Season::Winter);
        assert!(NaiveDate::from_ymd_opt(2026, 2, 29).is_none());
        assert_eq!(
            season(NaiveDate::from_ymd_opt(2027, 1, 1).unwrap()),
            Season::Winter
        );
    }
    #[test]
    fn wildcard_and_no_holiday() {
        let rules = [Holiday {
            month: 12,
            day: 25,
            region: "*".into(),
            label: "Navidad".into(),
        }];
        assert_eq!(
            holiday(
                NaiveDate::from_ymd_opt(2026, 12, 25).unwrap(),
                "ES-MD",
                &rules
            ),
            Some("Navidad")
        );
        assert_eq!(
            holiday(
                NaiveDate::from_ymd_opt(2026, 12, 26).unwrap(),
                "ES-MD",
                &rules
            ),
            None
        );
    }
}

#[cfg(test)]
mod region_tests {
    use super::*;
    #[test]
    fn region_overrides_country_and_global() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let rules = [
            Holiday {
                month: 10,
                day: 31,
                region: "*".into(),
                label: "Global".into(),
            },
            Holiday {
                month: 10,
                day: 31,
                region: "ES".into(),
                label: "País".into(),
            },
            Holiday {
                month: 10,
                day: 31,
                region: "ES-MD".into(),
                label: "Región".into(),
            },
        ];
        assert_eq!(holiday(date, "ES-MD", &rules), Some("Región"));
        assert_eq!(holiday(date, "ES-CT", &rules), Some("País"));
        assert_eq!(holiday(date, "JP", &rules), Some("Global"));
    }
}
