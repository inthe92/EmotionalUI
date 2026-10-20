//! Optional authored metadata. Unknown values never become inferred genre labels.
use chrono::{Datelike, NaiveDate};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Profile {
    pub version: u8,
    pub calm: Option<u8>,
    pub intensity: Option<u8>,
    pub nostalgia: Option<u8>,
    pub mystery: Option<u8>,
    pub exploration: Option<u8>,
    pub mental_demand: Option<u8>,
    pub tension: Option<u8>,
    pub pace: Option<u8>,
    pub resume_ease: Option<u8>,
    pub session_minutes: Option<u16>,
    pub seasons: BTreeMap<String, u8>,
    pub celebrations: BTreeMap<String, u8>,
    pub extra: BTreeMap<String, u8>,
}
impl Default for Profile {
    fn default() -> Self {
        Self {
            version: 1,
            calm: None,
            intensity: None,
            nostalgia: None,
            mystery: None,
            exploration: None,
            mental_demand: None,
            tension: None,
            pace: None,
            resume_ease: None,
            session_minutes: None,
            seasons: BTreeMap::new(),
            celebrations: BTreeMap::new(),
            extra: BTreeMap::new(),
        }
    }
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        let values = [
            self.calm,
            self.intensity,
            self.nostalgia,
            self.mystery,
            self.exploration,
            self.mental_demand,
            self.tension,
            self.pace,
            self.resume_ease,
        ];
        if self.version != 1
            || values.into_iter().flatten().any(|v| v > 100)
            || self.session_minutes.is_some_and(|v| v == 0 || v > 240)
        {
            return Err("Invalid editorial schema, attribute or duration".into());
        }
        for map in [&self.seasons, &self.celebrations, &self.extra] {
            if map.len() > 64
                || map
                    .iter()
                    .any(|(k, v)| k.trim().is_empty() || k.len() > 64 || *v > 100)
            {
                return Err("Invalid editorial affinity".into());
            }
        }
        if self
            .seasons
            .keys()
            .any(|v| !matches!(v.as_str(), "spring" | "summer" | "autumn" | "winter"))
        {
            return Err("Unknown season identifier".into());
        }
        Ok(())
    }
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub profile: Profile,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: u8,
    pub games: Vec<Entry>,
}
impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        if self.version != 1 || self.games.len() > 100_000 {
            return Err("Unsupported editorial catalog".into());
        }
        for entry in &self.games {
            if entry.id.trim().is_empty() || entry.id.len() > 4096 || !ids.insert(&entry.id) {
                return Err("Empty/duplicate editorial ID".into());
            }
            entry.profile.validate()?;
        }
        Ok(())
    }
    pub fn apply(&self, games: &mut [crate::library::Game]) -> Result<(), String> {
        self.validate()?;
        let entries = self
            .games
            .iter()
            .map(|e| (e.id.as_str(), &e.profile))
            .collect::<BTreeMap<_, _>>();
        for game in games {
            if let Some(profile) = entries.get(game.id.as_str()) {
                game.editorial = (*profile).clone();
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub id: String,
    #[serde(default)]
    pub month: Option<u32>,
    #[serde(default)]
    pub day: Option<u32>,
    #[serde(default)]
    pub season: Option<String>,
    #[serde(default)]
    pub weekend: bool,
    #[serde(default)]
    pub region: Option<String>,
}
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    #[serde(default)]
    pub events: Vec<Event>,
}
pub fn season_id(date: NaiveDate) -> &'static str {
    match date.month() {
        3..=5 => "spring",
        6..=8 => "summer",
        9..=11 => "autumn",
        _ => "winter",
    }
}
impl Calendar {
    pub fn validate(&self) -> Result<(), String> {
        if self.events.len() > 256 {
            return Err("Too many editorial events".into());
        }
        let mut ids = HashSet::new();
        for e in &self.events {
            if e.id.trim().is_empty()
                || e.id.len() > 64
                || !ids.insert(&e.id)
                || e.region.as_ref().is_some_and(|r| r.trim().is_empty())
            {
                return Err("Invalid/duplicate calendar event".into());
            }
            if e.day.is_some() && e.month.is_none()
                || e.month.is_some_and(|m| !(1..=12).contains(&m))
                || e.day.is_some_and(|d| {
                    NaiveDate::from_ymd_opt(2000, e.month.unwrap_or(0), d).is_none()
                })
            {
                return Err("Invalid calendar date".into());
            }
            if e.season
                .as_ref()
                .is_some_and(|s| !matches!(s.as_str(), "spring" | "summer" | "autumn" | "winter"))
            {
                return Err("Invalid calendar season".into());
            }
        }
        Ok(())
    }
    pub fn active<'a>(&'a self, date: Option<NaiveDate>, region: &str) -> Vec<&'a str> {
        let Some(date) = date else { return vec![] };
        let country = region.split('-').next().unwrap_or(region);
        self.events
            .iter()
            .filter(|e| {
                e.month.is_none_or(|m| m == date.month())
                    && e.day.is_none_or(|d| d == date.day())
                    && e.season.as_ref().is_none_or(|s| s == season_id(date))
                    && (!e.weekend || date.weekday().number_from_monday() >= 6)
                    && e.region
                        .as_ref()
                        .is_none_or(|r| r == "*" || r == region || r == country)
            })
            .map(|e| e.id.as_str())
            .collect()
    }
}
#[derive(Clone, Debug, Default)]
pub struct Signals<'a> {
    pub date: Option<NaiveDate>,
    pub events: Vec<&'a str>,
    pub daylight: Option<bool>,
    pub energy_preference: Option<u8>,
}
/// Context contributes at most 24 points. Choices/continuation can outweigh it.
pub fn bonus(profile: &Profile, signals: &Signals<'_>, raining: bool) -> i64 {
    if profile.validate().is_err() {
        return 0;
    }
    let mut value = 0;
    if let Some(date) = signals.date {
        value += i64::from(*profile.seasons.get(season_id(date)).unwrap_or(&0)) / 20;
    }
    // Several simultaneous events cannot stack into a mandatory thematic playlist.
    value += signals
        .events
        .iter()
        .filter_map(|e| profile.celebrations.get(*e))
        .copied()
        .max()
        .map_or(0, |v| i64::from(v) / 10);
    if raining {
        value += profile.calm.map_or(0, |v| i64::from(v) / 25)
            + profile.mystery.map_or(0, |v| i64::from(v) / 25);
    }
    if signals.daylight == Some(false) {
        value += profile.nostalgia.map_or(0, |v| i64::from(v) / 50);
    }
    if let (Some(wanted), Some(energy)) = (signals.energy_preference, profile.intensity) {
        value += (100 - i64::from(wanted.abs_diff(energy))) / 20;
    }
    value.min(24)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unknown_stays_unknown_and_invalid_is_rejected() {
        let p = Profile::default();
        assert_eq!(p.intensity, None);
        assert_eq!(bonus(&p, &Signals::default(), true), 0);
        let mut p = p;
        p.calm = Some(101);
        assert!(p.validate().is_err());
    }
    #[test]
    fn regional_weekend_and_unknown_date() {
        let c = Calendar {
            events: vec![Event {
                id: "local".into(),
                month: Some(10),
                day: Some(31),
                season: None,
                weekend: true,
                region: Some("ES-MD".into()),
            }],
        };
        c.validate().unwrap();
        let d = NaiveDate::from_ymd_opt(2026, 10, 31);
        assert_eq!(c.active(d, "ES-MD"), ["local"]);
        assert!(c.active(d, "ES-CT").is_empty());
        assert!(c.active(None, "ES-MD").is_empty());
    }
    #[test]
    fn event_effect_is_bounded_and_authored() {
        let mut p = Profile::default();
        p.celebrations.insert("halloween".into(), 100);
        p.calm = Some(100);
        p.mystery = Some(100);
        let s = Signals {
            events: vec!["halloween"; 20],
            ..Signals::default()
        };
        assert_eq!(bonus(&p, &s, true), 18);
        assert_eq!(bonus(&Profile::default(), &s, true), 0);
    }
}
