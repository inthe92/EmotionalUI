//! Prepared knowledge and bounded local statistics. No I/O or model inference.
use crate::{editorial::Profile, library::Game};
use chrono::{Datelike, NaiveDate, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Knowledge {
    pub id: String,
    pub genres: Vec<String>,
    pub settings: Vec<String>,
    pub tones: Vec<String>,
    pub themes: Vec<String>,
    pub aesthetics: Vec<String>,
    /// Confidence of these editorial interpretations, never identity certainty.
    pub confidence: u8,
    pub sources: Vec<String>,
    pub provenance: String,
    pub interruptibility: Option<u8>,
}
impl Knowledge {
    pub fn validate(&self) -> Result<(), String> {
        if self.confidence > 100 || self.interruptibility.is_some_and(|v| v > 100)
            || self.id.len() > 128 || self.provenance.len() > 1024
            || [&self.genres, &self.settings, &self.tones, &self.themes, &self.aesthetics, &self.sources]
                .iter().any(|v| v.len() > 32 || v.iter().any(|s| s.is_empty() || s.len() > 512))
        { return Err("Invalid Atlas knowledge".into()); }
        Ok(())
    }
    pub fn has(&self, tag: &str) -> bool {
        [&self.genres, &self.settings, &self.tones, &self.themes, &self.aesthetics]
            .iter().any(|v| v.iter().any(|s| s == tag))
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub system: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub profile: Profile,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Catalog {
    pub version: String,
    pub games: Vec<Entry>,
}
/// Punctuation/case tolerance only. No fuzzy matching or filename inference.
pub fn title_key(s: &str) -> String {
    s.to_lowercase().chars().map(|c| match c {
        'á' | 'à' | 'ä' | 'â' => 'a', 'é' | 'è' | 'ë' | 'ê' => 'e',
        'í' | 'ì' | 'ï' | 'î' => 'i', 'ó' | 'ò' | 'ö' | 'ô' => 'o',
        'ú' | 'ù' | 'ü' | 'û' => 'u', 'ñ' => 'n',
        c if c.is_alphanumeric() => c, _ => ' ',
    }).collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}
pub fn system_key(s: &str) -> String {
    match s.to_lowercase().as_str() {
        "sfc" | "superfamicom" | "super nintendo" | "snes" => "snes".into(),
        "gameboyadvance" | "game boy advance" | "gba" => "gba".into(),
        "megadrive" | "genesis" => "megadrive".into(),
        other => other.to_owned(),
    }
}
impl Catalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != "1.0.0" || self.games.len() > 100_000 { return Err("Unsupported classic Atlas".into()); }
        let mut ids = HashSet::new();
        for e in &self.games {
            if e.id.is_empty() || !ids.insert(&e.id) || e.title.is_empty() || e.system.is_empty()
                || e.profile.knowledge.id != e.id || e.aliases.len() > 32
            { return Err("Invalid classic Atlas identity".into()); }
            e.profile.validate()?;
        }
        Ok(())
    }
    pub fn apply(&self, games: &mut [Game]) -> Result<usize, String> {
        self.validate()?;
        let mut index: HashMap<(String, String), Option<&Entry>> = HashMap::new();
        for e in &self.games {
            for title in std::iter::once(&e.title).chain(&e.aliases) {
                let key = (system_key(&e.system), title_key(title));
                index.entry(key).and_modify(|old| {
                    if old.is_some_and(|v| v.id != e.id) { *old = None; }
                }).or_insert(Some(e));
            }
        }
        let mut count = 0;
        for g in games {
            let Some(Some(e)) = index.get(&(system_key(&g.system), title_key(&g.title))) else { continue; };
            let p = &mut g.editorial;
            macro_rules! fill { ($($f:ident),*) => { $(if p.$f.is_none() { p.$f = e.profile.$f; })* }; }
            fill!(calm, intensity, nostalgia, mystery, exploration, mental_demand, tension, pace, resume_ease, session_minutes);
            for (k,v) in &e.profile.extra { p.extra.entry(k.clone()).or_insert(*v); }
            if p.knowledge.id.is_empty() { p.knowledge = e.profile.knowledge.clone(); }
            if g.genre == "Sin clasificar" { g.genre = e.profile.knowledge.genres.join(" · "); }
            count += 1;
        }
        Ok(count)
    }
}

#[derive(Clone, Debug)]
pub struct Observation {
    pub game_id: String,
    pub started_utc: i64,
    pub local_hour: u8,
    pub weekend: bool,
    pub seconds: u32,
    pub recommended: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Habits {
    pub target_minutes: Option<u16>,
    pub samples: usize,
    pub genre_affinity: BTreeMap<String, i64>,
    pub context_choices: BTreeMap<String, i64>,
    pub recent: BTreeMap<String, i64>,
    pub conversions: usize,
}
pub fn period(hour: u8) -> u8 { match hour { 6..=11 => 0, 12..=18 => 1, 19..=23 => 2, _ => 3 } }
impl Habits {
    /// Require 5 matching sessions or 8 global ones. Latest 256 / 180 days.
    /// Local hour/weekend are captured at launch, not reconstructed in a new zone.
    pub fn learn(games: &[Game], rows: &[Observation], now: Option<(i64, u8, bool)>) -> Self {
        let Some((utc, hour, weekend)) = now else { return Self::default(); };
        let by_id: HashMap<_,_> = games.iter().filter(|g| g.available).map(|g| (g.id.as_str(),g)).collect();
        let mut rows: Vec<_> = rows.iter().filter(|r| r.local_hour < 24 && (2..=14400).contains(&r.seconds)
            && utc.checked_sub(r.started_utc).is_some_and(|age| (0..=180*86400).contains(&age))
            && by_id.contains_key(r.game_id.as_str())).collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.started_utc));
        let mut seen = HashSet::new();
        rows.retain(|r| seen.insert((r.game_id.as_str(),r.started_utc)));
        rows.truncate(256);
        let same: Vec<_> = rows.iter().copied().filter(|r| period(r.local_hour)==period(hour)).collect();
        let specific: Vec<_> = same.iter().copied().filter(|r| r.weekend==weekend).collect();
        let selected = if specific.len() >= 5 { &specific } else if same.len() >= 5 { &same } else { &rows };
        let enough = selected.len() >= if same.len() >= 5 { 5 } else { 8 };
        let mut h = Self { samples: selected.len(), conversions: rows.iter().filter(|r| r.recommended).count(), ..Self::default() };
        if enough {
            // Chronological EWMA: recent samples contribute 1/4, rounded minutes.
            let mut mean = f64::from(selected.last().unwrap().seconds);
            for r in selected.iter().rev().skip(1) { mean = mean*0.75 + f64::from(r.seconds)*0.25; }
            h.target_minutes = Some(((mean/60.).round() as u16).clamp(5,240));
        }
        let mut genres: BTreeMap<String, usize> = BTreeMap::new();
        for r in &rows {
            let age = utc-r.started_utc;
            let penalty = if age < 86400 { 12 } else if age < 3*86400 { 6 } else { 0 };
            let prior = h.recent.entry(r.game_id.clone()).or_default(); *prior=(*prior).max(penalty);
            for genre in &by_id[r.game_id.as_str()].editorial.knowledge.genres { *genres.entry(genre.clone()).or_default() += 1; }
        }
        if rows.len() >= 8 {
            for (genre,n) in genres { if n >= 3 { h.genre_affinity.insert(genre, (n as i64*12/rows.len() as i64).min(12)); } }
        }
        if specific.len() >= 5 {
            let mut counts = BTreeMap::<String,i64>::new();
            for r in specific { *counts.entry(r.game_id.clone()).or_default() += 1; }
            for (id,n) in counts { if n>=3 { h.context_choices.insert(id,(n*2).min(10)); } }
        }
        h
    }
}
pub fn local_context(local: Option<chrono::NaiveDateTime>, utc: Option<i64>) -> Option<(i64,u8,bool)> {
    local.zip(utc).map(|(d,t)| (t,d.hour() as u8,d.weekday().number_from_monday()>=6))
}
pub fn weekend(date: Option<NaiveDate>) -> bool { date.is_some_and(|d| d.weekday().number_from_monday() >= 6) }
