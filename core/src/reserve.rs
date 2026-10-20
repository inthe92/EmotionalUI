//! Small, persisted discovery reserve. No I/O, dates fabricated or popularity inputs.
use crate::{
    editorial::Signals,
    library::Game,
    recommendation::{restore_ranked, Context},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reserve {
    pub schema: u8,
    #[serde(default)]
    pub affinity: BTreeMap<String, i8>,
    pub key: String,
    pub prepared_day: Option<i64>,
    pub atlas_version: String,
    pub pool: Vec<String>,
    pub selections: BTreeMap<String, Vec<String>>,
}
pub fn fingerprint(games: &[Game], version: &str, personalized: bool) -> String {
    struct Fnv(u64);
    impl std::hash::Hasher for Fnv {
        fn finish(&self) -> u64 {
            self.0
        }
        fn write(&mut self, bytes: &[u8]) {
            for b in bytes {
                self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x100000001b3);
            }
        }
    }
    use std::hash::{Hash, Hasher};
    let mut h = Fnv(0xcbf29ce484222325);
    version.hash(&mut h);
    personalized.hash(&mut h);
    for g in games {
        g.id.hash(&mut h);
        g.available.hash(&mut h);
        g.started.hash(&mut h);
        g.favorite.hash(&mut h);
        g.choices.hash(&mut h);
        g.choices_by_mood.hash(&mut h);
        let p = &g.editorial;
        [
            p.calm,
            p.intensity,
            p.nostalgia,
            p.mystery,
            p.exploration,
            p.mental_demand,
            p.tension,
            p.pace,
            p.resume_ease,
        ]
        .hash(&mut h);
        p.session_minutes.hash(&mut h);
        p.seasons.hash(&mut h);
        p.celebrations.hash(&mut h);
        p.extra.hash(&mut h);
    }
    format!("{:016x}", h.finish())
}
impl Reserve {
    pub fn valid(&self, key: &str, day: Option<i64>) -> bool {
        self.schema == 1
            && self.key == key
            && self.pool.len() <= 96
            && self.selections.len() <= 64
            && self.selections.values().all(|v| v.len() <= 96)
            && match (self.prepared_day, day) {
                (Some(a), Some(b)) => b.checked_sub(a).is_some_and(|days| (0..14).contains(&days)),
                (None, None) => true,
                _ => false,
            }
    }
    pub fn prepare(games: &[Game], version: &str, day: Option<i64>, personalized: bool) -> Self {
        Self::prepare_affinity(games, version, day, personalized, BTreeMap::new())
    }
    pub fn prepare_affinity(
        games: &[Game],
        version: &str,
        day: Option<i64>,
        personalized: bool,
        affinity: BTreeMap<String, i8>,
    ) -> Self {
        let mut ids = HashSet::new();
        let mut pool = Vec::new();
        for i in 0..12 {
            let month = [1, 4, 7, 10][i % 4];
            let signals = Signals {
                date: chrono::NaiveDate::from_ymd_opt(2024, month, 15),
                ..Default::default()
            };
            let ctx = Context {
                mood: [
                    crate::mood::Mood::Dawn,
                    crate::mood::Mood::Horizon,
                    crate::mood::Mood::Shelter,
                    crate::mood::Mood::Silence,
                ][i / 3],
                minutes: 30,
                raining: i % 3 == 0,
                slot: i as u64,
                refresh: 0,
            };
            for n in
                crate::recommendation::rank_affinity(games, ctx, &signals, personalized, &affinity)
                    .into_iter()
                    .take(8)
            {
                if ids.insert(games[n].id.clone()) {
                    pool.push(games[n].id.clone());
                }
            }
        }
        Self {
            affinity,
            schema: 1,
            key: fingerprint(games, version, personalized),
            prepared_day: day,
            atlas_version: version.into(),
            pool,
            selections: BTreeMap::new(),
        }
    }
    pub fn select(
        &mut self,
        games: &[Game],
        ctx: Context,
        signals: &Signals<'_>,
        personalized: bool,
        context: &str,
        refresh: bool,
    ) -> Vec<usize> {
        let pool = self.pool.iter().map(String::as_str).collect::<HashSet<_>>();
        let mapping = games
            .iter()
            .enumerate()
            .filter(|(_, g)| g.available && pool.contains(g.id.as_str()))
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        let subset = mapping
            .iter()
            .map(|i| games[*i].clone())
            .collect::<Vec<_>>();
        let ranked = crate::recommendation::rank_affinity(
            &subset,
            ctx,
            signals,
            personalized,
            &self.affinity,
        );
        let saved = self.selections.get(context).cloned().unwrap_or_default();
        let order = if !refresh {
            restore_ranked(&subset, &saved, ranked)
        } else {
            ranked
        };
        let selected = order.into_iter().map(|i| mapping[i]).collect::<Vec<_>>();
        if self.selections.len() >= 64 && !self.selections.contains_key(context) {
            self.selections.clear();
        }
        self.selections.insert(
            context.into(),
            selected.iter().map(|i| games[*i].id.clone()).collect(),
        );
        selected
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ctx() -> Context {
        Context {
            mood: crate::mood::Mood::Shelter,
            minutes: 30,
            raining: true,
            slot: 42,
            refresh: 0,
        }
    }
    #[test]
    fn stable_across_reboot_and_unknown_clock() {
        let g = crate::library::demo_catalog();
        let mut r = Reserve::prepare(&g, "test", None, true);
        assert!(r.valid(&r.key, None));
        assert!(!r.valid(&r.key, Some(1)));
        let a = r.select(&g, ctx(), &Signals::default(), true, "night-rain", false);
        assert_eq!(
            a,
            r.clone()
                .select(&g, ctx(), &Signals::default(), true, "night-rain", false)
        );
    }
    #[test]
    fn ttl_versions_preferences_library_and_backward_clock() {
        let mut g = crate::library::demo_catalog();
        let r = Reserve::prepare(&g, "v1", Some(10), true);
        assert!(r.valid(&r.key, Some(23)));
        assert!(!r.valid(&r.key, Some(24)));
        assert!(!r.valid(&r.key, Some(9)));
        g[0].available = false;
        assert!(!r.valid(&fingerprint(&g, "v1", true), Some(11)));
        assert!(!r.valid(&fingerprint(&g, "v2", true), Some(11)));
        assert!(!r.valid(&fingerprint(&g, "v1", false), Some(11)));
    }
    #[test]
    fn exhausted_empty_and_unavailable_never_escape_pool() {
        let mut g = crate::library::demo_catalog();
        let mut r = Reserve::prepare(&g, "v", None, true);
        for a in &mut g {
            a.available = false;
        }
        assert!(r
            .select(&g, ctx(), &Signals::default(), true, "unknown", true)
            .is_empty());
        assert!(Reserve::prepare(&[], "v", None, true).pool.is_empty());
    }
    #[test]
    fn no_popularity_field_and_low_profile_title_can_win() {
        let mut g = crate::library::demo_catalog();
        for a in &mut g {
            a.started = false;
            a.favorite = false;
            a.choices = 0;
        }
        g[4].choices = 100;
        assert_eq!(
            crate::recommendation::rank_policy(&g, ctx(), &Signals::default(), true)[0],
            4
        );
        assert!(!Reserve::prepare(&g, "v", None, true).pool.is_empty());
    }
}
