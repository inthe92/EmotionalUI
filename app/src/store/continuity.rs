use super::*;
use emotional_core::continuity::{Evidence, Preference};
use std::collections::{BTreeMap, HashSet};
impl Store {
    pub fn continuity_preferences(
        &self,
    ) -> Result<BTreeMap<String, Preference>, Box<dyn std::error::Error>> {
        let mut q = self
            .0
            .prepare("SELECT game_id,data FROM continuity_preferences")?;
        let mut out: BTreeMap<String, Preference> = BTreeMap::new();
        for row in q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
            let (id, data) = row?;
            out.insert(id, toml::from_str(&data)?);
        }
        for id in self.history()?.completed {
            out.entry(id).or_default().completed = true;
        }
        Ok(out)
    }
    pub fn set_continuity(
        &mut self,
        id: &str,
        p: &Preference,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if id.is_empty() || !(-1..=1).contains(&p.affinity) {
            return Err("Invalid continuity preference".into());
        }
        let tx = self.0.transaction()?;
        tx.execute("INSERT INTO continuity_preferences VALUES(?1,?2) ON CONFLICT(game_id) DO UPDATE SET data=excluded.data",params![id,toml::to_string(p)?])?;
        tx.execute("INSERT INTO progress VALUES(?1,?2) ON CONFLICT(game_id) DO UPDATE SET completed=excluded.completed",params![id,p.completed])?;
        tx.commit()?;
        Ok(())
    }
    pub fn continuity_activity(&self) -> Result<Vec<Evidence>, Box<dyn std::error::Error>> {
        let mut q = self
            .0
            .prepare("SELECT rowid,data FROM launch_log ORDER BY rowid")?;
        let mut out = Vec::new();
        let mut dated = HashSet::new();
        for row in q.query_map([], |r| Ok((r.get::<_, u64>(0)?, r.get::<_, String>(1)?)))? {
            let (ordinal, data) = row?;
            let r: emotional_services::launcher::configured::LaunchRecord = toml::from_str(&data)?;
            if let Some(t) = r.started_utc {
                dated.insert((r.game_id.clone(), t));
            }
            out.push(Evidence {
                game_id: r.game_id,
                ordinal: ordinal + 1_000_000,
                utc: r.started_utc,
                seconds: Some((r.elapsed_ms / 1000).min(u64::from(u32::MAX)) as u32),
                confirmed: r.outcome == "exited-success" && r.elapsed_ms >= 2000,
                unexpected: r.outcome == "exit-error",
                voluntary_return: None,
                saved: None,
            });
        }
        let h = self.history()?;
        let n = h.sessions.len();
        for (i, s) in h.sessions.into_iter().enumerate() {
            if !dated.contains(&(s.game_id.clone(), s.started_utc)) {
                out.push(Evidence {
                    game_id: s.game_id,
                    ordinal: n.saturating_sub(i) as u64,
                    utc: Some(s.started_utc),
                    seconds: s.duration_seconds,
                    confirmed: true,
                    unexpected: false,
                    voluntary_return: None,
                    saved: None,
                });
            }
        }
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn additive_preferences_preserve_metadata_audio_and_completion() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let p = Preference {
            pinned: true,
            removed: true,
            ..Default::default()
        };
        s.set_continuity("a", &p).unwrap();
        assert_eq!(s.continuity_preferences().unwrap()["a"], p);
        s.complete("a").unwrap();
        assert!(s.continuity_preferences().unwrap()["a"].completed);
        s.set_continuity("a", &p).unwrap();
        assert!(!s.continuity_preferences().unwrap()["a"].completed);
        s.clear_history().unwrap();
        assert!(s.continuity_preferences().unwrap()["a"].pinned);
        assert_eq!(s.audio().unwrap().master, 40);
    }
    #[test]
    fn migrated_activity_has_unknown_save_and_no_false_voluntary_return() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        s.record_session(&emotional_core::product::Session {
            game_id: "a".into(),
            started_utc: 1792317600,
            duration_seconds: Some(300),
        })
        .unwrap();
        let a = s.continuity_activity().unwrap();
        assert_eq!(a.len(), 1);
        assert!(a[0].saved.is_none() && a[0].voluntary_return.is_none());
    }
}
