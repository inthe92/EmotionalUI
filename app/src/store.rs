#[path = "store/atlas.rs"]
mod atlas;
#[path = "store/continuity.rs"]
mod continuity;
use emotional_core::library::Game;
use rusqlite::{params, Connection};
use std::path::Path;
pub struct Store(pub(super) Connection);
impl Store {
    pub fn last_valid_time(&self) -> rusqlite::Result<Option<chrono::NaiveDateTime>> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='last_valid_local_time'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        Ok(value
            .and_then(|v| chrono::NaiveDateTime::parse_from_str(&v, "%Y-%m-%dT%H:%M:%S").ok())
            .filter(|d| emotional_core::environment::plausible_time(*d)))
    }
    fn save_last_valid_time(&self, value: chrono::NaiveDateTime) -> rusqlite::Result<()> {
        if !emotional_core::environment::plausible_time(value) {
            return Ok(());
        }
        self.0.execute("INSERT INTO settings VALUES('last_valid_local_time',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [value.format("%Y-%m-%dT%H:%M:%S").to_string()])?;
        Ok(())
    }
    pub fn launch_rom(&self, id: &str) -> rusqlite::Result<Option<std::path::PathBuf>> {
        use rusqlite::OptionalExtension;
        self.0
            .query_row(
                "SELECT rom_path FROM imported_games WHERE id=?1",
                [id],
                |r| r.get::<_, String>(0).map(Into::into),
            )
            .optional()
    }
    pub fn library_systems(
        &self,
        root: &Path,
    ) -> rusqlite::Result<std::collections::BTreeMap<String, String>> {
        let canonical = root.canonicalize().unwrap_or_else(|_| root.to_owned());
        let root = canonical.as_path();
        let mut query = self.0.prepare("SELECT id,source_xml FROM imported_games")?;
        let rows = query
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .map(|row| {
                row.map(|(id, source)| {
                    let parent = std::path::Path::new(&source).parent();
                    let key = parent
                        .and_then(|p| p.strip_prefix(root).ok())
                        .and_then(|p| p.components().next())
                        .map(|c| c.as_os_str())
                        .or_else(|| parent.filter(|p| *p == root).and_then(|p| p.file_name()))
                        .and_then(|p| p.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    (id, key)
                })
            })
            .collect();
        rows
    }
    pub fn last_launched(&self) -> rusqlite::Result<Option<String>> {
        use rusqlite::OptionalExtension;
        self.0.query_row("SELECT game_id FROM launch_log WHERE outcome='exited-success' ORDER BY rowid DESC LIMIT 1",[],|r|r.get(0)).optional()
    }
    pub fn record_launch(
        &mut self,
        r: &emotional_services::launcher::configured::LaunchRecord,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.0.transaction()?;
        tx.execute(
            "INSERT INTO launch_log VALUES(?1,?2,?3,?4)",
            params![r.id, r.game_id, r.outcome, toml::to_string(r)?],
        )?;
        if r.session_registered && r.outcome == "exited-success" && r.elapsed_ms >= 2000 {
            let utc = r
                .started_utc
                .ok_or("Cannot date a session with unknown clock")?;
            tx.execute("INSERT INTO activity VALUES(?1,?2,?3) ON CONFLICT(game_id,started_utc) DO UPDATE SET duration_seconds=excluded.duration_seconds",params![r.game_id,utc,r.elapsed_ms/1000])?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn history(&self) -> rusqlite::Result<emotional_core::product::History> {
        let mut q = self.0.prepare(
            "SELECT game_id,started_utc,duration_seconds FROM activity ORDER BY started_utc DESC",
        )?;
        let sessions = q
            .query_map([], |r| {
                Ok(emotional_core::product::Session {
                    game_id: r.get(0)?,
                    started_utc: r.get(1)?,
                    duration_seconds: r.get(2)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let mut q = self
            .0
            .prepare("SELECT game_id FROM progress WHERE completed=1 ORDER BY game_id")?;
        let completed = q
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(emotional_core::product::History {
            sessions,
            completed,
        })
    }
    #[cfg(test)]
    pub fn record_session(
        &self,
        session: &emotional_core::product::Session,
    ) -> rusqlite::Result<()> {
        self.0.execute("INSERT INTO activity VALUES(?1,?2,?3) ON CONFLICT(game_id,started_utc) DO UPDATE SET duration_seconds=excluded.duration_seconds",params![session.game_id,session.started_utc,session.duration_seconds])?;
        Ok(())
    }
    pub fn complete(&self, id: &str) -> rusqlite::Result<()> {
        self.0.execute(
            "INSERT INTO progress VALUES(?1,1) ON CONFLICT(game_id) DO UPDATE SET completed=1",
            [id],
        )?;
        Ok(())
    }
    pub fn audio(&self) -> Result<emotional_core::audio::Preferences, Box<dyn std::error::Error>> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .0
            .query_row(
                "SELECT value FROM settings WHERE key='ambient_audio'",
                [],
                |r| r.get(0),
            )
            .optional()?;
        let preferences: emotional_core::audio::Preferences = value
            .map(|v| toml::from_str(&v))
            .transpose()?
            .unwrap_or_default();
        preferences.validate()?;
        Ok(preferences)
    }
    fn save_audio(
        &self,
        value: &emotional_core::audio::Preferences,
    ) -> Result<(), Box<dyn std::error::Error>> {
        value.validate()?;
        self.0.execute("INSERT INTO settings VALUES('ambient_audio',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[toml::to_string(value)?])?;
        Ok(())
    }
    pub fn privacy(&self) -> Result<emotional_core::product::Privacy, Box<dyn std::error::Error>> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .0
            .query_row("SELECT value FROM settings WHERE key='privacy'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(value
            .map(|v| toml::from_str(&v))
            .transpose()?
            .unwrap_or_default())
    }
    fn save_privacy(
        &self,
        value: &emotional_core::product::Privacy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.0.execute("INSERT INTO settings VALUES('privacy',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[toml::to_string(value)?])?;
        Ok(())
    }
    pub fn clear_usage(&self) -> rusqlite::Result<()> {
        self.clear_history()
    }
    fn clear_history(&self) -> rusqlite::Result<()> {
        self.0.execute_batch("BEGIN IMMEDIATE;DELETE FROM activity;DELETE FROM launch_log;DELETE FROM preferences;DELETE FROM habits;DELETE FROM session;DELETE FROM session_items;DELETE FROM settings WHERE key='discovery_reserve';COMMIT;")
    }
    pub fn import_editorial(
        &mut self,
        catalog: &emotional_core::editorial::Catalog,
    ) -> Result<(), Box<dyn std::error::Error>> {
        catalog.validate()?;
        let tx = self.0.transaction()?;
        for entry in &catalog.games {
            tx.execute("INSERT INTO editorial VALUES(?1,?2) ON CONFLICT(game_id) DO UPDATE SET metadata=excluded.metadata",params![entry.id,toml::to_string(&entry.profile)?])?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let db = Connection::open(path)?;
        let version: i64 = db.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > 4 {
            return Err(rusqlite::Error::InvalidQuery);
        }
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode=WAL; BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS preferences (id TEXT PRIMARY KEY, choices INTEGER NOT NULL DEFAULT 0, recent INTEGER NOT NULL DEFAULT 0); CREATE TABLE IF NOT EXISTS session (slot TEXT PRIMARY KEY, ids TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS session_items(slot TEXT NOT NULL,position INTEGER NOT NULL,id TEXT NOT NULL,PRIMARY KEY(slot,position));
CREATE TABLE IF NOT EXISTS habits(id TEXT NOT NULL,mood INTEGER NOT NULL,choices INTEGER NOT NULL,PRIMARY KEY(id,mood));
CREATE TABLE IF NOT EXISTS imported_games(id TEXT PRIMARY KEY,rom_path TEXT NOT NULL,source_xml TEXT NOT NULL,metadata TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS editorial(game_id TEXT PRIMARY KEY,metadata TEXT NOT NULL);CREATE TABLE IF NOT EXISTS activity(game_id TEXT NOT NULL,started_utc INTEGER NOT NULL,duration_seconds INTEGER,PRIMARY KEY(game_id,started_utc));
CREATE INDEX IF NOT EXISTS activity_recent ON activity(started_utc DESC);
CREATE TABLE IF NOT EXISTS launch_log(attempt_id TEXT PRIMARY KEY,game_id TEXT NOT NULL,outcome TEXT NOT NULL,data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS progress(game_id TEXT PRIMARY KEY,completed INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS atlas_links(game_id TEXT PRIMARY KEY,atlas_id TEXT NOT NULL,provenance TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS continuity_preferences(game_id TEXT PRIMARY KEY,data TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
PRAGMA user_version=4;COMMIT;")?;
        Ok(Self(db))
    }
    pub fn load(&self, games: &mut [Game]) -> rusqlite::Result<()> {
        let indices = games
            .iter()
            .enumerate()
            .map(|(i, g)| (g.id.clone(), i))
            .collect::<std::collections::HashMap<_, _>>();
        let mut q = self.0.prepare("SELECT game_id,metadata FROM editorial")?;
        let rows = q.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (id, data) = row?;
            if let Some(&i) = indices.get(&id) {
                let profile: emotional_core::editorial::Profile =
                    toml::from_str(&data).map_err(|_| rusqlite::Error::InvalidQuery)?;
                profile
                    .validate()
                    .map_err(|_| rusqlite::Error::InvalidQuery)?;
                games[i].editorial = profile;
            }
        }
        let mut q = self
            .0
            .prepare("SELECT id,choices,recent FROM preferences")?;
        let rows = q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, u32>(1)?,
                r.get::<_, u32>(2)?,
            ))
        })?;
        for row in rows {
            let (id, choices, recent) = row?;
            if let Some(&i) = indices.get(&id) {
                games[i].choices = choices;
                games[i].recent_recommendations = recent;
            }
        }
        let mut q = self.0.prepare("SELECT id,mood,choices FROM habits")?;
        let rows = q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, usize>(1)?,
                r.get::<_, u16>(2)?,
            ))
        })?;
        for row in rows {
            let (id, mood, count) = row?;
            if let Some(&i) = indices.get(&id) {
                if mood < 4 {
                    games[i].choices_by_mood[mood] = count;
                }
            }
        }
        Ok(())
    }
    #[cfg(test)]
    pub fn save(&mut self, games: &[Game]) -> rusqlite::Result<()> {
        self.save_state(
            &games.iter().map(Preference::from).collect::<Vec<_>>(),
            None,
        )
    }
    fn save_state(
        &mut self,
        games: &[Preference],
        session: Option<(&str, &[String])>,
    ) -> rusqlite::Result<()> {
        let tx = self.0.transaction()?;
        {
            // Compile these statements once per snapshot, not five times per game.
            // Keep their lifetime inside the transaction before committing.
            let mut preference = tx.prepare("INSERT INTO preferences VALUES (?1,?2,?3) ON CONFLICT(id) DO UPDATE SET choices=excluded.choices,recent=excluded.recent WHERE preferences.choices<>excluded.choices OR preferences.recent<>excluded.recent")?;
            let mut habit = tx.prepare("INSERT INTO habits VALUES(?1,?2,?3) ON CONFLICT(id,mood) DO UPDATE SET choices=excluded.choices WHERE habits.choices<>excluded.choices")?;
            for g in games {
                preference.execute(params![g.id, g.choices, g.recent_recommendations])?;
                for (mood, count) in g.choices_by_mood.iter().enumerate() {
                    habit.execute(params![g.id, mood, *count])?;
                }
            }
        }
        if let Some((slot, ids)) = session {
            Self::write_session(&tx, slot, ids)?;
        }
        tx.commit()
    }
    pub fn import(
        &mut self,
        games: &[emotional_services::emulationstation::ImportedGame],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self
            .0
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        for g in games {
            tx.execute("INSERT INTO imported_games VALUES(?1,?2,?3,?4) ON CONFLICT(id) DO UPDATE SET rom_path=excluded.rom_path,source_xml=excluded.source_xml,metadata=excluded.metadata",params![g.id,g.rom.to_string_lossy(),g.source.to_string_lossy(),toml::to_string(&g.metadata)?])?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Read our SQLite copy only. Revalidate each path against an explicit authorized root.
    pub fn catalog(&self, root: &Path) -> Result<Vec<Game>, Box<dyn std::error::Error>> {
        let root = root.canonicalize()?;
        if !root.is_dir() {
            return Err("Library root must be a directory".into());
        }
        let mut query = self
            .0
            .prepare("SELECT id,rom_path,source_xml,metadata FROM imported_games ORDER BY id")?;
        let rows = query.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        let mut games = Vec::new();
        for row in rows {
            let (id, rom, source, metadata) = row?;
            let imported = emotional_services::emulationstation::ImportedGame {
                id: id.clone(),
                rom: rom.into(),
                source: source.into(),
                metadata: toml::from_str(&metadata)?,
            };
            if !imported.available(&root) {
                continue;
            }
            let field = |key: &str, fallback: &str| {
                imported
                    .metadata
                    .get(key)
                    .map(|v| v.split_whitespace().collect::<Vec<_>>().join(" "))
                    .filter(|v| !v.is_empty())
                    .unwrap_or_else(|| fallback.into())
                    .chars()
                    .take(512)
                    .collect::<String>()
            };
            games.push(Game {
                id,
                title: field("name", "Sin título"),
                subtitle: field("desc", "Un camino de tu colección."),
                genre: field("genre", "Sin clasificar"),
                available: true,
                // Original play count is not proof of a resumable save or an EmotionalUI choice.
                started: false,
                favorite: field("favorite", "").eq_ignore_ascii_case("true"),
                session_minutes: 30,
                choices: 0,
                editorial: emotional_core::editorial::Profile::default(),
                recent_recommendations: 0,
                choices_by_mood: [0; 4],
                // Unknown emotions remain neutral, independent of genre.
                emotions: emotional_core::library::Emotions {
                    calm: 50,
                    intensity: 50,
                    nostalgia: 50,
                    mystery: 50,
                    exploration: 50,
                    tension: 50,
                    joy: 50,
                    concentration: 50,
                    immersion: 50,
                },
            });
        }
        Ok(games)
    }
    /// References only: never copy or rewrite local artwork. Reject paths outside the root.
    pub fn art_paths(
        &self,
        root: &Path,
    ) -> Result<
        std::collections::BTreeMap<String, Vec<crate::artwork::Resource>>,
        Box<dyn std::error::Error>,
    > {
        let root = root.canonicalize()?;
        let mut q = self
            .0
            .prepare("SELECT id,rom_path,source_xml,metadata FROM imported_games ORDER BY id")?;
        let rows = q.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        let mut result = std::collections::BTreeMap::new();
        for row in rows {
            let (id, rom, source, text) = row?;
            if !Path::new(&rom)
                .canonicalize()
                .ok()
                .is_some_and(|p| p.is_file() && p.starts_with(&root))
            {
                continue;
            }
            let metadata: std::collections::BTreeMap<String, String> = toml::from_str(&text)?;
            // Typed fields are optional. Unknown fields remain in original metadata.
            let mut resources = Vec::new();
            for (field, role) in [
                ("emotionalui_hero", crate::artwork::Role::Hero),
                ("fanart", crate::artwork::Role::Hero),
                ("boxart", crate::artwork::Role::Boxart),
                ("image", crate::artwork::Role::Generic),
                ("screenshot", crate::artwork::Role::Screenshot),
                ("titleshot", crate::artwork::Role::Title),
                ("marquee", crate::artwork::Role::Logo),
                ("thumbnail", crate::artwork::Role::Generic),
            ] {
                if let Some(raw) = metadata.get(field).filter(|s| !s.trim().is_empty()) {
                    let path = Path::new(&source).parent().unwrap_or(&root).join(raw);
                    if let Ok(path) = path.canonicalize() {
                        if path.is_file()
                            && path.starts_with(&root)
                            && !resources
                                .iter()
                                .any(|r: &crate::artwork::Resource| r.path == path)
                        {
                            resources.push(crate::artwork::Resource {
                                path,
                                role,
                                root: root.clone(),
                            });
                        }
                    }
                }
            }
            if !resources.is_empty() {
                result.insert(id, resources);
            }
        }
        Ok(result)
    }
    pub fn session(&self, slot: &str) -> rusqlite::Result<Option<Vec<String>>> {
        use rusqlite::OptionalExtension;
        let mut q = self
            .0
            .prepare("SELECT id FROM session_items WHERE slot=?1 ORDER BY position")?;
        let ids = q
            .query_map([slot], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if !ids.is_empty() {
            return Ok(Some(ids));
        }
        self.0
            .query_row("SELECT ids FROM session WHERE slot=?1", [slot], |r| {
                r.get::<_, String>(0)
            })
            .optional()
            .map(|v| {
                v.map(|s| {
                    s.split(',')
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect()
                })
            })
    }
    fn write_session(db: &Connection, slot: &str, ids: &[String]) -> rusqlite::Result<()> {
        db.execute("DELETE FROM session_items WHERE slot=?1", [slot])?;
        for (pos, id) in ids.iter().enumerate() {
            db.execute(
                "INSERT INTO session_items VALUES(?1,?2,?3)",
                params![slot, pos, id],
            )?;
        }
        // Legacy table is retained for v1 compatibility; no new IDs are delimiter-encoded.
        db.execute(
            "INSERT INTO session VALUES(?1,'') ON CONFLICT(slot) DO UPDATE SET ids='';",
            [slot],
        )?;
        Ok(())
    }
    pub fn save_session(&mut self, slot: &str, ids: &[String]) -> rusqlite::Result<()> {
        let tx = self.0.transaction()?;
        Self::write_session(&tx, slot, ids)?;
        tx.commit()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn library_systems_use_import_sources_not_rom_subfolders() {
        let store = Store::open(Path::new(":memory:")).unwrap();
        for (id, source) in [
            ("a", "/collection/snes/gamelist.xml"),
            ("b", "/collection/gba/gamelist.xml"),
            ("c", "/external/gamelist.xml"),
        ] {
            store
                .0
                .execute(
                    "INSERT INTO imported_games VALUES(?1,?2,?3,'')",
                    params![id, "/collection/snes/rpg/game.sfc", source],
                )
                .unwrap();
        }
        let systems = store.library_systems(Path::new("/collection")).unwrap();
        assert_eq!(systems["a"], "snes");
        assert_eq!(systems["b"], "gba");
        assert_eq!(systems["c"], "unknown");
        assert_eq!(
            store
                .library_systems(Path::new("/collection/snes"))
                .unwrap()["a"],
            "snes"
        );
    }
    #[cfg(unix)]
    #[test]
    fn library_systems_resolve_an_authorized_symlinked_root_once() {
        let base =
            std::env::temp_dir().join(format!("emotional-library-roots-{}", std::process::id()));
        let real = base.join("real");
        let alias = base.join("alias");
        std::fs::create_dir_all(real.join("snes")).unwrap();
        let real = real.canonicalize().unwrap();
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        let store = Store::open(Path::new(":memory:")).unwrap();
        store
            .0
            .execute(
                "INSERT INTO imported_games VALUES('a','game.fixture',?1,'')",
                [real.join("snes/gamelist.xml").to_string_lossy().to_string()],
            )
            .unwrap();
        assert_eq!(store.library_systems(&alias).unwrap()["a"], "snes");
        std::fs::remove_dir_all(base).unwrap();
    }
    #[test]
    fn persistence_roundtrip() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut g = emotional_core::library::demo_catalog();
        g[0].choices = 4;
        s.save(&g).unwrap();
        let mut restored = emotional_core::library::demo_catalog();
        s.load(&mut restored).unwrap();
        assert_eq!(restored[0].choices, 4);
        s.save_session("day", &["demo-0".into()]).unwrap();
        assert_eq!(s.session("day").unwrap().unwrap(), vec!["demo-0"]);
        assert!(s.session("absent").unwrap().is_none());
    }
}

/// Bounded queue: filesystem work never runs inside a navigation callback.
pub struct Persistence {
    audio_pending: std::sync::Arc<std::sync::Mutex<Option<emotional_core::audio::Preferences>>>,
    sender: Option<std::sync::mpsc::SyncSender<Command>>,
    results: std::sync::mpsc::Receiver<Result<(), String>>,
    thread: Option<std::thread::JoinHandle<()>>,
}
enum Command {
    Clock(chrono::NaiveDateTime),
    Save(Save),
    Privacy(emotional_core::product::Privacy),
    Clear,
    Audio,
    Reserve(emotional_core::reserve::Reserve),
    Continuity(String, emotional_core::continuity::Preference),
}
struct Preference {
    id: String,
    choices: u32,
    recent_recommendations: u32,
    choices_by_mood: [u16; 4],
}
impl From<&Game> for Preference {
    fn from(g: &Game) -> Self {
        Self {
            id: g.id.clone(),
            choices: g.choices,
            recent_recommendations: g.recent_recommendations,
            choices_by_mood: g.choices_by_mood,
        }
    }
}
struct Save {
    games: Vec<Preference>,
    slot: String,
    ids: Vec<String>,
}
impl Persistence {
    pub fn clock(&self, value: chrono::NaiveDateTime) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Clock(value))?;
        Ok(())
    }
    pub fn start(mut store: Store) -> Self {
        let (sender, rx) = std::sync::mpsc::sync_channel::<Command>(8);
        let (tx, results) = std::sync::mpsc::channel();
        let audio_pending = std::sync::Arc::new(std::sync::Mutex::new(
            None::<emotional_core::audio::Preferences>,
        ));
        let pending = audio_pending.clone();
        let thread = std::thread::spawn(move || {
            while let Ok(save) = rx.recv() {
                let result = match save {
                    Command::Clock(value) => {
                        store.save_last_valid_time(value).map_err(|e| e.to_string())
                    }
                    Command::Save(save) => store
                        .save_state(&save.games, Some((&save.slot, &save.ids)))
                        .map_err(|e| e.to_string()),
                    Command::Privacy(value) => {
                        store.save_privacy(&value).map_err(|e| e.to_string())
                    }
                    Command::Clear => store.clear_history().map_err(|e| e.to_string()),
                    Command::Audio => Ok(()),
                    Command::Reserve(value) => {
                        store.save_reserve(&value).map_err(|e| e.to_string())
                    }
                    Command::Continuity(id, p) => {
                        store.set_continuity(&id, &p).map_err(|e| e.to_string())
                    }
                };
                let _ = tx.send(result);
                let audio = pending.lock().unwrap().take();
                if let Some(value) = audio {
                    let _ = tx.send(store.save_audio(&value).map_err(|e| e.to_string()));
                }
            }
        });
        Self {
            audio_pending,
            sender: Some(sender),
            results,
            thread: Some(thread),
        }
    }
    pub fn save(
        &self,
        games: &[Game],
        slot: &str,
        ids: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Almacenamiento cerrado")?
            .try_send(Command::Save(Save {
                games: games.iter().map(Preference::from).collect(),
                slot: slot.into(),
                ids,
            }))?;
        Ok(())
    }
    pub fn reserve(
        &self,
        value: &emotional_core::reserve::Reserve,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Reserve(value.clone()))?;
        Ok(())
    }
    pub fn continuity(
        &self,
        id: &str,
        p: &emotional_core::continuity::Preference,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Continuity(id.into(), p.clone()))?;
        Ok(())
    }
    pub fn audio(
        &self,
        value: &emotional_core::audio::Preferences,
    ) -> Result<(), Box<dyn std::error::Error>> {
        value.validate()?;
        *self.audio_pending.lock().unwrap() = Some(value.clone());
        match self
            .sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Audio)
        {
            Ok(()) | Err(std::sync::mpsc::TrySendError::Full(_)) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
    pub fn privacy(
        &self,
        value: &emotional_core::product::Privacy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Privacy(value.clone()))?;
        Ok(())
    }
    pub fn clear_history(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.sender
            .as_ref()
            .ok_or("Storage closed")?
            .try_send(Command::Clear)?;
        Ok(())
    }
    pub fn error(&self) -> Option<String> {
        self.results.try_iter().filter_map(Result::err).last()
    }
}
impl Persistence {
    pub fn finish(&mut self) -> Result<(), String> {
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| "Falló el hilo de almacenamiento".to_owned())?;
        }
        if let Some(error) = self.error() {
            return Err(error);
        }
        Ok(())
    }
}
impl Drop for Persistence {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("Persistence failed during shutdown: {error}");
        }
    }
}

#[cfg(test)]
mod audit_tests {
    use super::*;
    #[test]
    fn last_valid_time_roundtrips_and_ignores_corrupt_or_invalid_values() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        assert_eq!(s.last_valid_time().unwrap(), None);
        let date =
            chrono::NaiveDateTime::parse_from_str("2026-10-20T09:30:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap();
        s.save_last_valid_time(date).unwrap();
        assert_eq!(s.last_valid_time().unwrap(), Some(date));
        s.0.execute(
            "UPDATE settings SET value='not a date' WHERE key='last_valid_local_time'",
            [],
        )
        .unwrap();
        assert_eq!(s.last_valid_time().unwrap(), None);
        s.0.execute(
            "UPDATE settings SET value='1970-01-01T00:00:00' WHERE key='last_valid_local_time'",
            [],
        )
        .unwrap();
        assert_eq!(s.last_valid_time().unwrap(), None);
    }
    #[test]
    fn clock_checkpoint_survives_writer_and_reopen() {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "emotional-clock-{}-{unique}.sqlite",
            std::process::id()
        ));
        let date =
            chrono::NaiveDateTime::parse_from_str("2026-10-20T09:30:00", "%Y-%m-%dT%H:%M:%S")
                .unwrap();
        let mut writer = Persistence::start(Store::open(&path).unwrap());
        writer.clock(date).unwrap();
        writer.finish().unwrap();
        drop(writer);
        let reopened = Store::open(&path).unwrap();
        assert_eq!(reopened.last_valid_time().unwrap(), Some(date));
        drop(reopened);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn imported_catalog_feeds_gui_and_revalidates_availability() {
        let root = std::env::temp_dir().join(format!("emotional-catalog-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let rom = root.join("fiction.test");
        std::fs::write(&rom, "original synthetic fixture").unwrap();
        let xml = root.join("gamelist.xml");
        let original="<gameList><game><path>fiction.test</path><name>Lago de papel</name><favorite>true</favorite><playcount>9</playcount><genre>Action</genre></game></gameList>";
        std::fs::write(&xml, original).unwrap();
        let report = emotional_services::emulationstation::import(&xml, &root).unwrap();
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        s.import(&report.games).unwrap();
        let mut games = s.catalog(&root).unwrap();
        assert_eq!(games[0].title, "Lago de papel");
        assert!(games[0].favorite);
        assert!(!games[0].started);
        assert_eq!(games[0].emotions.intensity, 50);
        games[0].choices = 3;
        s.save(&games).unwrap();
        let mut reloaded = s.catalog(&root).unwrap();
        s.load(&mut reloaded).unwrap();
        assert_eq!(reloaded[0].choices, 3);
        assert_eq!(std::fs::read_to_string(&xml).unwrap(), original);
        std::fs::remove_file(&rom).unwrap();
        assert!(s.catalog(&root).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn habits_roundtrip() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut g = emotional_core::library::demo_catalog();
        g[0].choices_by_mood = [1, 2, 3, 4];
        s.save(&g).unwrap();
        g[0].choices_by_mood = [0; 4];
        s.load(&mut g).unwrap();
        assert_eq!(g[0].choices_by_mood, [1, 2, 3, 4]);
    }
    #[test]
    fn identical_snapshots_do_not_rewrite_preferences_or_habits() {
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        let mut games = emotional_core::library::demo_catalog();
        store.save(&games).unwrap();
        let changes = store.0.total_changes();
        store.save(&games).unwrap();
        assert_eq!(store.0.total_changes(), changes);
        games[0].choices += 1;
        games[0].choices_by_mood[2] += 1;
        store.save(&games).unwrap();
        assert_eq!(store.0.total_changes(), changes + 2);
        let mut reloaded = emotional_core::library::demo_catalog();
        store.load(&mut reloaded).unwrap();
        assert_eq!(reloaded[0].choices, games[0].choices);
        assert_eq!(reloaded[0].choices_by_mood, games[0].choices_by_mood);
    }
    #[test]
    fn import_does_not_overwrite_preferences() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let mut g = emotional_core::library::demo_catalog();
        g[0].choices = 7;
        s.save(&g).unwrap();
        let i = emotional_services::emulationstation::ImportedGame {
            id: g[0].id.clone(),
            rom: "fixture.test".into(),
            source: "gamelist.xml".into(),
            metadata: std::collections::BTreeMap::new(),
        };
        s.import(&[i]).unwrap();
        g[0].choices = 0;
        s.load(&mut g).unwrap();
        assert_eq!(g[0].choices, 7);
    }
    #[test]
    fn future_schema_is_rejected() {
        let path =
            std::env::temp_dir().join(format!("emotional-future-{}.sqlite", std::process::id()));
        let db = Connection::open(&path).unwrap();
        db.pragma_update(None, "user_version", 99).unwrap();
        drop(db);
        assert!(Store::open(&path).is_err());
        let db = Connection::open(&path).unwrap();
        assert_eq!(
            db.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
                .unwrap(),
            99
        );
        drop(db);
        let _ = std::fs::remove_file(path);
    }
    #[test]
    fn worker_flushes_before_exit() {
        let path =
            std::env::temp_dir().join(format!("emotional-worker-{}.sqlite", std::process::id()));
        {
            let worker = Persistence::start(Store::open(&path).unwrap());
            let mut g = emotional_core::library::demo_catalog();
            g[0].choices = 9;
            worker.save(&g, "slot", vec!["demo-0".into()]).unwrap();
        }
        let s = Store::open(&path).unwrap();
        let mut g = emotional_core::library::demo_catalog();
        s.load(&mut g).unwrap();
        assert_eq!(g[0].choices, 9);
        drop(s);
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod session_tests {
    use super::*;
    #[test]
    fn arbitrary_identifiers_and_empty_session() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let ids = vec!["es:/roms/A,B.test".into(), "a\nb".into()];
        s.save_session("test", &ids).unwrap();
        assert_eq!(s.session("test").unwrap(), Some(ids));
        s.save_session("test", &[]).unwrap();
        assert_eq!(s.session("test").unwrap(), Some(vec![]));
    }
    #[test]
    fn v1_session_and_preferences_survive() {
        let path = std::env::temp_dir().join(format!("emotional-v1-{}.sqlite", std::process::id()));
        let db = Connection::open(&path).unwrap();
        db.execute_batch("PRAGMA user_version=1;CREATE TABLE session(slot TEXT PRIMARY KEY,ids TEXT NOT NULL);INSERT INTO session VALUES('old','demo-1,demo-0');CREATE TABLE preferences(id TEXT PRIMARY KEY,choices INTEGER NOT NULL,recent INTEGER NOT NULL);INSERT INTO preferences VALUES('demo-0',3,2);").unwrap();
        drop(db);
        let s = Store::open(&path).unwrap();
        assert_eq!(s.session("old").unwrap().unwrap(), ["demo-1", "demo-0"]);
        let mut g = emotional_core::library::demo_catalog();
        s.load(&mut g).unwrap();
        assert_eq!(g[0].choices, 3);
        drop(s);
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod phase7_tests {
    use super::*;
    #[test]
    fn truthful_history_privacy_and_erasure() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        assert!(s.history().unwrap().sessions.is_empty());
        let record = emotional_core::product::Session {
            game_id: "demo-0".into(),
            started_utc: 123,
            duration_seconds: None,
        };
        s.record_session(&record).unwrap();
        s.complete("demo-1").unwrap();
        let h = s.history().unwrap();
        assert_eq!(h.sessions, [record]);
        assert_eq!(h.completed, ["demo-1"]);
        let p = emotional_core::product::Privacy {
            personalized: false,
            statistics: false,
            ..Default::default()
        };
        s.save_privacy(&p).unwrap();
        assert_eq!(s.privacy().unwrap(), p);
        s.clear_history().unwrap();
        assert!(s.history().unwrap().sessions.is_empty());
        assert_eq!(s.history().unwrap().completed, ["demo-1"]);
        assert_eq!(s.privacy().unwrap(), p);
    }
    #[test]
    fn editorial_survives_history_delete_and_reimport() {
        let mut s = Store::open(Path::new(":memory:")).unwrap();
        let c: emotional_core::editorial::Catalog =
            toml::from_str(include_str!("../../assets/editorial/catalog.toml")).unwrap();
        s.import_editorial(&c).unwrap();
        s.clear_history().unwrap();
        let mut g = emotional_core::library::demo_catalog();
        s.load(&mut g).unwrap();
        assert!(g[0].editorial.calm.is_some());
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;
    #[test]
    fn audio_preferences_roundtrip_and_clear_preserves_them() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        assert_eq!(
            s.audio().unwrap().mode,
            emotional_core::audio::Mode::Minimal
        );
        let p = emotional_core::audio::Preferences {
            mode: emotional_core::audio::Mode::Silence,
            master: 17,
            interface_enabled: false,
            ..Default::default()
        };
        s.save_audio(&p).unwrap();
        s.clear_history().unwrap();
        assert_eq!(s.audio().unwrap(), p);
    }
    #[test]
    fn rapid_audio_updates_persist_latest_without_full_queue_errors() {
        let path = std::env::temp_dir().join(format!(
            "emotional-audio-prefs-{}.sqlite",
            std::process::id()
        ));
        {
            let writer = Persistence::start(Store::open(&path).unwrap());
            let mut p = emotional_core::audio::Preferences::default();
            for n in 0..200 {
                p.master = (n % 101) as u8;
                writer.audio(&p).unwrap();
            }
        }
        let s = Store::open(&path).unwrap();
        assert_eq!(s.audio().unwrap().master, 98);
        drop(s);
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn invalid_audio_configuration_is_not_saved() {
        let s = Store::open(Path::new(":memory:")).unwrap();
        let p = emotional_core::audio::Preferences {
            master: 101,
            ..Default::default()
        };
        assert!(s.save_audio(&p).is_err());
        assert_eq!(s.audio().unwrap().master, 40);
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;
    use emotional_services::launcher::configured::LaunchRecord;
    fn record() -> LaunchRecord {
        LaunchRecord {
            id: "attempt-1".into(),
            game_id: "fixture-1".into(),
            profile: "explicit".into(),
            started_utc: None,
            elapsed_ms: 2500,
            outcome: "exited-success".into(),
            exit_code: Some(0),
            session_registered: false,
        }
    }
    #[test]
    fn process_success_without_clock_does_not_invent_a_dated_session() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        db.record_launch(&record()).unwrap();
        assert_eq!(db.last_launched().unwrap().as_deref(), Some("fixture-1"));
        assert!(db.history().unwrap().sessions.is_empty());
        assert!(db.history().unwrap().completed.is_empty());
        db.clear_usage().unwrap();
        assert!(db.last_launched().unwrap().is_none());
    }
    #[test]
    fn only_confirmed_sustained_process_can_add_activity() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let mut r = record();
        r.started_utc = Some(1792317600);
        r.session_registered = true;
        db.record_launch(&r).unwrap();
        let h = db.history().unwrap();
        assert_eq!(h.sessions.len(), 1);
        assert_eq!(h.sessions[0].duration_seconds, Some(2));
        assert!(h.completed.is_empty());
    }
    #[test]
    fn failures_and_invalid_clock_never_become_activity() {
        let mut db = Store::open(Path::new(":memory:")).unwrap();
        let mut r = record();
        r.outcome = "exit-error".into();
        db.record_launch(&r).unwrap();
        assert!(db.last_launched().unwrap().is_none());
        assert!(db.history().unwrap().sessions.is_empty());
        r.id = "attempt-2".into();
        r.outcome = "exited-success".into();
        r.session_registered = true;
        assert!(db.record_launch(&r).is_err());
        assert!(db.last_launched().unwrap().is_none());
    }
}

#[cfg(test)]
mod phase12_contention_tests {
    use super::*;
    #[test]
    fn import_waits_for_a_slow_existing_writer_without_dropping_the_system() {
        let path = std::env::temp_dir().join(format!(
            "emotional-phase12-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let mut first = Store::open(&path.join("test.sqlite")).unwrap();
        let mut importer = Store::open(&path.join("test.sqlite")).unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            let hold = first
                .0
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .unwrap();
            tx.send(()).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(400));
            hold.commit().unwrap();
        });
        rx.recv().unwrap();
        let game = emotional_services::emulationstation::ImportedGame {
            id: "owned-fixture".into(),
            rom: path.join("game.fixture"),
            source: path.join("gamelist.xml"),
            metadata: Default::default(),
        };
        importer.import(&[game]).unwrap();
        writer.join().unwrap();
        assert_eq!(
            importer
                .0
                .query_row("SELECT count(*) FROM imported_games", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            1
        );
        drop(importer);
        std::fs::remove_dir_all(path).unwrap();
    }
}

#[cfg(test)]
mod artwork_metadata_tests {
    use super::*;
    #[test]
    fn imported_roles_roundtrip_without_source_changes_or_filename_matching() {
        let root = std::env::temp_dir().join(format!("editorial-sqlite-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("owned.fixture"), b"owned text, not ROM").unwrap();
        for name in ["hero.png", "box.png", "screen.png"] {
            image::RgbImage::from_pixel(100, 80, image::Rgb([10, 20, 30]))
                .save(root.join(name))
                .unwrap();
        }
        let xml = root.join("gamelist.xml");
        let source="<gameList><game><path>owned.fixture</path><name>Una casa</name><fanart>hero.png</fanart><boxart>box.png</boxart><screenshot>screen.png</screenshot><marquee>../outside.png</marquee></game></gameList>";
        std::fs::write(&xml, source).unwrap();
        let imported = emotional_services::emulationstation::import(&xml, &root).unwrap();
        let mut store = Store::open(Path::new(":memory:")).unwrap();
        store.import(&imported.games).unwrap();
        let art = store.art_paths(&root).unwrap();
        let resources = art.get(&imported.games[0].id).unwrap();
        assert_eq!(resources.len(), 3);
        assert_eq!(resources[0].role, crate::artwork::Role::Hero);
        assert_eq!(resources[1].role, crate::artwork::Role::Boxart);
        assert_eq!(resources[2].role, crate::artwork::Role::Screenshot);
        assert_eq!(std::fs::read_to_string(xml).unwrap(), source);
        std::fs::remove_file(root.join("owned.fixture")).unwrap();
        assert!(store.art_paths(&root).unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
}
