//! Read-only Atlas adapter; explicit local identities, no filename guessing.
use crate::store::Store;
use emotional_core::{editorial::Profile, library::Game};
use rusqlite::{Connection, OpenFlags};
use std::{error::Error, path::Path};
pub struct Atlas {
    db: Connection,
    pub version: String,
    experimental: bool,
}
impl Atlas {
    pub fn open(path: &Path) -> Result<Self, Box<dyn Error>> {
        Self::open_channel(path, false)
    }
    pub fn open_channel(path: &Path, experimental: bool) -> Result<Self, Box<dyn Error>> {
        if std::fs::metadata(path)?.len() > 32 * 1024 * 1024 {
            return Err("Atlas exceeds 32 MiB pilot budget".into());
        }
        let db = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        db.pragma_update(None, "query_only", true)?;
        let id: i64 = db.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let schema: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if id != 1161909324 || schema != 1 {
            return Err("Unsupported Atlas identity/schema".into());
        }
        let version: String =
            db.query_row("SELECT value FROM meta WHERE key='version'", [], |r| {
                r.get(0)
            })?;
        let hash: String = db.query_row(
            "SELECT value FROM meta WHERE key='source_sha256'",
            [],
            |r| r.get(0),
        )?;
        if version.len() > 80 || hash.len() != 64 {
            return Err("Invalid Atlas version".into());
        }
        Ok(Self {
            db,
            version: format!("{version}:{hash}:experimental={experimental}"),
            experimental,
        })
    }
    pub fn contains(&self, id: &str) -> rusqlite::Result<bool> {
        self.db.query_row(
            "SELECT EXISTS(SELECT 1 FROM games WHERE id=?1)",
            [id],
            |r| r.get(0),
        )
    }
    pub fn explanations(
        &self,
        store: &Store,
    ) -> Result<std::collections::BTreeMap<String, String>, Box<dyn Error>> {
        let mut result = std::collections::BTreeMap::new();
        if self.experimental {
            use rusqlite::OptionalExtension;
            let mut q = self
                .db
                .prepare("SELECT explanation FROM profiles_experimental WHERE game_id=?1")?;
            for (local, id) in store.atlas_links()? {
                if let Some(text) = q.query_row([id], |r| r.get::<_, String>(0)).optional()? {
                    if text.chars().count() > 180 {
                        return Err("Atlas explanation too long".into());
                    }
                    result.insert(local, format!("Atlas experimental: {text}"));
                }
            }
        }
        Ok(result)
    }
    pub fn apply(&self, store: &Store, games: &mut [Game]) -> Result<usize, Box<dyn Error>> {
        let links = store.atlas_links()?;
        let mut n = 0;
        let mut profiles = std::collections::HashMap::<String, Option<Profile>>::new();
        let sql = if self.experimental {
            "SELECT COALESCE(e.profile,p.profile) FROM profiles p LEFT JOIN profiles_experimental e ON e.game_id=p.game_id WHERE p.game_id=?1"
        } else {
            "SELECT profile FROM profiles WHERE game_id=?1"
        };
        let mut q = self.db.prepare(sql)?;
        for g in games {
            if let Some(id) = links.get(&g.id) {
                use rusqlite::OptionalExtension;
                let profile = if let Some(profile) = profiles.get(id) {
                    profile.clone()
                } else {
                    let text: Option<String> = q.query_row([id], |r| r.get(0)).optional()?;
                    let profile = text
                        .map(|text| -> Result<Profile, Box<dyn Error>> {
                            if text.len() > 65536 {
                                return Err("Atlas profile too large".into());
                            }
                            let p: Profile = toml::from_str(&text)?;
                            p.validate()?;
                            Ok(p)
                        })
                        .transpose()?;
                    profiles.insert(id.clone(), profile.clone());
                    profile
                };
                if let Some(profile) = profile {
                    let mut profile = profile;
                    // Keep this exact SQLite version authoritative. Otherwise the
                    // prepared fallback can silently mix attributes from an older card.
                    profile.knowledge.id = id.clone();
                    profile.knowledge.provenance = self.version.clone();
                    macro_rules! local {($($name:ident),*)=>{$(if g.editorial.$name.is_some(){profile.$name=g.editorial.$name;})*};}
                    local!(
                        calm,
                        intensity,
                        nostalgia,
                        mystery,
                        exploration,
                        mental_demand,
                        tension,
                        pace,
                        resume_ease,
                        session_minutes
                    );
                    profile.seasons.extend(g.editorial.seasons.clone());
                    profile
                        .celebrations
                        .extend(g.editorial.celebrations.clone());
                    profile.extra.extend(g.editorial.extra.clone());
                    g.editorial = profile;
                    n += 1;
                }
            }
        }
        Ok(n)
    }
}
pub fn day(clock: &emotional_services::clock::Clock) -> Option<i64> {
    use chrono::Datelike;
    match clock.trust {
        emotional_services::clock::Trust::UserConfirmed
        | emotional_services::clock::Trust::Simulated => clock
            .known_now()
            .map(|d| i64::from(d.date().num_days_from_ce())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/atlas/emotional-atlas-0.1.0-pilot.db")
    }
    #[test]
    fn beta_export_keeps_exact_sqlite_version_and_unknowns() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/atlas/emotional-atlas.db");
        let atlas = Atlas::open_channel(&path, true).unwrap();
        assert!(atlas.version.starts_with("0.4.0-ai-experimental.5-snes-research.2:"));
        let store = Store::open(Path::new(":memory:")).unwrap();
        store.0.execute("INSERT INTO imported_games VALUES('demo-0','/synthetic/non-rom','/synthetic/gamelist','unused')",[]).unwrap();
        store.atlas_link("demo-0", "atlas:snes:000005").unwrap();
        let mut games = emotional_core::library::demo_catalog();
        games[0].system = "snes".into();
        games[0].title = "Super Metroid".into();
        assert_eq!(atlas.apply(&store, &mut games).unwrap(), 1);
        assert_eq!(games[0].editorial.knowledge.id, "atlas:snes:000005");
        assert_eq!(games[0].editorial.exploration, Some(100));
        assert_eq!(games[0].editorial.session_minutes, None);
        let before = toml::to_string(&games[0].editorial).unwrap();
        crate::atlas_classic::apply(&mut games).unwrap();
        assert_eq!(toml::to_string(&games[0].editorial).unwrap(), before);
        assert!(atlas.db.execute("DELETE FROM games", []).is_err());
    }
    #[test]
    fn pilot_is_read_only_and_never_guesses_a_filename() {
        let a = Atlas::open(&path()).unwrap();
        assert!(a.contains("atlas:snes:000001").unwrap());
        assert!(a.db.execute("DELETE FROM games", []).is_err());
        let s = Store::open(Path::new(":memory:")).unwrap();
        let mut g = emotional_core::library::demo_catalog();
        assert_eq!(a.apply(&s, &mut g).unwrap(), 0);
    }
    #[test]
    fn explicit_link_preserves_personal_overrides_and_unknowns() {
        let a = Atlas::open(&path()).unwrap();
        let s = Store::open(Path::new(":memory:")).unwrap();
        s.0.execute("INSERT INTO imported_games VALUES('demo-0','/synthetic/non-rom','/synthetic/gamelist','unused')",[]).unwrap();
        s.atlas_link("demo-0", "atlas:snes:000001").unwrap();
        let mut g = emotional_core::library::demo_catalog();
        g[0].editorial.calm = Some(93);
        assert_eq!(a.apply(&s, &mut g).unwrap(), 1);
        assert_eq!(g[0].editorial.calm, Some(93));
        assert_eq!(g[0].editorial.intensity, None);
        assert!(s.atlas_link("unimported", "atlas:snes:000001").is_err());
    }
    #[test]
    fn experimental_channel_requires_opt_in_and_preserves_unknowns() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/atlas/emotional-atlas-0.2.0-experimental.1.db");
        let default = Atlas::open(&path).unwrap();
        let experimental = Atlas::open_channel(&path, true).unwrap();
        assert_ne!(default.version, experimental.version);
        let s = Store::open(Path::new(":memory:")).unwrap();
        s.0.execute("INSERT INTO imported_games VALUES('demo-0','/synthetic/non-rom','/synthetic/gamelist','unused')",[]).unwrap();
        s.atlas_link("demo-0", "atlas:snes:000001").unwrap();
        let mut g = emotional_core::library::demo_catalog();
        for game in &mut g {
            game.editorial = Default::default();
        }
        default.apply(&s, &mut g).unwrap();
        assert_eq!(g[0].editorial.calm, None);
        experimental.apply(&s, &mut g).unwrap();
        assert_eq!(g[0].editorial.calm, Some(45));
        assert_eq!(g[0].editorial.nostalgia, None);
        assert!(default.explanations(&s).unwrap().is_empty());
        assert!(experimental.explanations(&s).unwrap()["demo-0"].contains("experimental"));
    }
    #[test]
    fn reserve_roundtrip_and_history_erasure() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        let g = emotional_core::library::demo_catalog();
        let r = emotional_core::reserve::Reserve::prepare(&g, "v1", None, true);
        s.save_reserve(&r).unwrap();
        assert_eq!(s.reserve().unwrap().unwrap().pool, r.pool);
        s.clear_usage().unwrap();
        assert!(s.reserve().unwrap().is_none());
    }
    #[test]
    fn evidence_catalog_keeps_opt_in_unknowns_and_local_preferences() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/atlas/emotional-atlas-0.2.0-experimental.2.db");
        let stable = Atlas::open(&path).unwrap();
        let experimental = Atlas::open_channel(&path, true).unwrap();
        let store = Store::open(Path::new(":memory:")).unwrap();
        store.0.execute("INSERT INTO imported_games VALUES('demo-0','/synthetic/non-rom','/synthetic/gamelist','unused')", []).unwrap();
        store.atlas_link("demo-0", "atlas:snes:000001").unwrap();
        let mut games = emotional_core::library::demo_catalog();
        games[0].editorial = Default::default();
        stable.apply(&store, &mut games).unwrap();
        assert_eq!(games[0].editorial.exploration, None);
        experimental.apply(&store, &mut games).unwrap();
        assert_eq!(games[0].editorial.exploration, Some(60));
        assert_eq!(games[0].editorial.intensity, Some(60));
        assert_eq!(games[0].editorial.nostalgia, None);
        assert_eq!(games[0].editorial.calm, None);
        assert_eq!(games[0].editorial.session_minutes, None);
        games[0].editorial.exploration = Some(93);
        experimental.apply(&store, &mut games).unwrap();
        assert_eq!(games[0].editorial.exploration, Some(93));
        assert!(stable.explanations(&store).unwrap().is_empty());
        assert!(experimental.explanations(&store).unwrap()["demo-0"].contains("rutas secretas"));
        assert!(experimental
            .db
            .execute("DELETE FROM research_claims", [])
            .is_err());
    }
    #[test]
    fn compact_identity_export_and_optional_scale_fixture_use_existing_reader() {
        let small = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/atlas/exports/emotional-atlas-0.3.0-metadata-pilot.db");
        let stable = Atlas::open(&small).unwrap();
        let experimental = Atlas::open_channel(&small, true).unwrap();
        assert!(stable.contains("atlas:snes:000005").unwrap());
        // The new metadata candidate is unresolved and has no emotional profile.
        assert!(experimental.contains("atlas:snes:000053").unwrap());
        let count: i64 = experimental
            .db
            .query_row("SELECT count(*) FROM profiles_experimental", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 21);
        let store = Store::open(Path::new(":memory:")).unwrap();
        store.0.execute("INSERT INTO imported_games VALUES('demo-0','/synthetic/non-rom','/synthetic/gamelist','unused')", []).unwrap();
        store.atlas_link("demo-0", "atlas:snes:000053").unwrap();
        let mut games = emotional_core::library::demo_catalog();
        games[0].editorial = Default::default();
        experimental.apply(&store, &mut games).unwrap();
        assert_eq!(games[0].editorial.calm, None);
        assert!(experimental.explanations(&store).unwrap().is_empty());
        assert!(experimental.db.execute("DELETE FROM games", []).is_err());
        // Optional large fixture is generated in development, never shipped as real data.
        if let Some(path) = std::env::var_os("EMOTIONAL_ATLAS_SCALE_FIXTURE") {
            let large = Atlas::open_channel(Path::new(&path), true).unwrap();
            let kind: String = large
                .db
                .query_row("SELECT value FROM meta WHERE key='dataset_kind'", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(kind, "synthetic");
            let count: i64 = large
                .db
                .query_row("SELECT count(*) FROM games", [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 20000);
            store.atlas_link("demo-0", "atlas:snes:000001").unwrap();
            large.apply(&store, &mut games).unwrap();
            assert_eq!(games[0].editorial.calm, Some(80));
            assert!(large.db.execute("DELETE FROM profiles", []).is_err());
        }
    }
}
