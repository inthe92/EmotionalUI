//! Temporary real-library mode. No service/process handoff, synthetic games or shell.
use crate::store::Store;
use emotional_core::library::Game;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::mpsc,
};
pub struct Update {
    pub games: Vec<Game>,
    pub systems: crate::library_browser::Systems,
    pub art: BTreeMap<String, Vec<crate::artwork::Resource>>,
    pub message: String,
    pub explanations: BTreeMap<String, String>,
}
/// Merge only in-memory activity by stable ID in linear time. Imported metadata
/// remains current, and user choices made while XML loads are never overwritten.
pub fn preserve_activity(previous: &[Game], incoming: &mut [Game]) {
    let previous = previous
        .iter()
        .map(|g| (g.id.as_str(), g))
        .collect::<std::collections::HashMap<_, _>>();
    for game in incoming {
        if let Some(old) = previous.get(game.id.as_str()) {
            game.choices = old.choices;
            game.choices_by_mood = old.choices_by_mood;
            game.recent_recommendations = old.recent_recommendations;
            game.started = old.started;
        }
    }
}
pub fn root(explicit: Option<PathBuf>) -> Option<PathBuf> {
    // /roms is documented upstream, but use it only when it exists on this machine.
    explicit.or_else(|| Path::new("/roms").is_dir().then(|| PathBuf::from("/roms")))
}
pub fn sources(root: &Path) -> std::io::Result<Vec<(PathBuf, PathBuf)>> {
    let root = root.canonicalize()?;
    let mut dirs = vec![root.clone()];
    for entry in std::fs::read_dir(&root)?.take(512) {
        let path = entry?.path();
        if let Ok(path) = path.canonicalize() {
            if path.is_dir() && path.starts_with(&root) {
                dirs.push(path);
            }
        }
    }
    dirs.sort();
    dirs.dedup();
    Ok(dirs
        .into_iter()
        .filter_map(|d| {
            let xml = d.join("gamelist.xml").canonicalize().ok()?;
            (xml.is_file() && xml.starts_with(&root)).then_some((xml, d))
        })
        .collect())
}
fn stamp(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    use std::hash::{Hash, Hasher};
    use std::io::Read;
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err("Trial XML exceeds 8 MiB".into());
    }
    let mut h = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut h);
    Ok(format!("import-v1:{:016x}", h.finish()))
}
pub fn start(
    db: PathBuf,
    root: Option<PathBuf>,
    atlas_path: Option<PathBuf>,
    experimental: bool,
) -> mpsc::Receiver<Result<Update, String>> {
    let (tx, rx) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let Some(root) = root.filter(|r| r.is_dir()) else {
                tx.send(Ok(Update {
                    games: vec![],
                    systems: Default::default(),
                    art: BTreeMap::new(),
                    explanations: BTreeMap::new(),
                    message: "Modo prueba · No encontramos una biblioteca disponible.".into(),
                }))?;
                return Ok(());
            };
            let mut store = Store::open(&db)?;
            let atlas = atlas_path
                .as_deref()
                .map(|p| crate::atlas::Atlas::open_channel(p, experimental))
                .transpose()?;
            let explanations = atlas
                .as_ref()
                .map(|a| a.explanations(&store))
                .transpose()?
                .unwrap_or_default();
            let files = sources(&root)?;
            let total = files.len();
            let mut warnings = 0;
            for (index, (xml, dir)) in files.iter().enumerate() {
                let mut changed = false;
                let key = format!("trial-source:{}", xml.to_string_lossy());
                let signature = stamp(xml);
                let previous = store
                    .0
                    .query_row("SELECT value FROM settings WHERE key=?1", [&key], |r| {
                        r.get::<_, String>(0)
                    })
                    .ok();
                if signature.as_ref().ok() != previous.as_ref() || previous.is_none() {
                    match signature.and_then(|signature| {
                        let report=emotional_services::emulationstation::import(xml,dir)?;
                        store.import(&report.games)?;
                        store.0.execute("INSERT INTO settings VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",rusqlite::params![key,signature])?;
                        Ok(report.games.len())
                    }) {
                        Ok(n)=>{changed=true;println!("TRIAL xml_imported={n}");},
                        Err(error)=>{warnings+=1;eprintln!("TRIAL import skipped: {error}");}
                    }
                } else {
                    println!("TRIAL xml_unchanged=true");
                }
                // Cached systems need no identical full-catalog snapshot/ranking.
                // Always publish the first usable model and final scan status.
                if index != 0 && index + 1 != total && !changed {
                    continue;
                }
                let mut games = store.catalog(&root)?;
                store.load(&mut games)?;
                if let Some(a) = &atlas {
                    a.apply(&store, &mut games)?;
                }
                let art = store.art_paths(&root)?;
                tx.send(Ok(Update {
                    games,
                    systems: store.library_systems(&root)?,
                    explanations: explanations.clone(),
                    art,
                    message: format!(
                        "Modo prueba · Biblioteca {}/{} · {} sin leer",
                        index + 1,
                        total,
                        warnings
                    ),
                }))?;
            }
            if total == 0 {
                let mut games = store.catalog(&root)?;
                store.load(&mut games)?;
                if let Some(a) = &atlas {
                    a.apply(&store, &mut games)?;
                }
                tx.send(Ok(Update {systems:store.library_systems(&root)?,games,explanations,art:store.art_paths(&root)?,message:"Modo prueba · Sin gamelist.xml disponibles; puedes volver cuando conectes tu colección.".into()}))?;
            }
            Ok(())
        })();
        if let Err(e) = result {
            let _ = tx.send(Err(e.to_string()));
        }
    });
    rx
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progressive_refresh_keeps_choices_by_id_without_reverting_metadata() {
        let mut previous = emotional_core::library::demo_catalog();
        previous[0].choices = 19;
        previous[0].choices_by_mood = [1, 2, 3, 4];
        previous[0].recent_recommendations = 7;
        previous[0].started = true;
        let mut incoming = previous.iter().rev().cloned().collect::<Vec<_>>();
        let last = incoming.last_mut().unwrap();
        last.choices = 0;
        last.choices_by_mood = [0; 4];
        last.recent_recommendations = 0;
        last.started = false;
        last.title = "New source title".into();
        let mut new = previous[0].clone();
        new.id = "new-local-id".into();
        new.choices = 0;
        incoming.push(new);
        preserve_activity(&previous, &mut incoming);
        let old = incoming.iter().find(|g| g.id == previous[0].id).unwrap();
        assert_eq!(old.title, "New source title");
        assert_eq!(old.choices, 19);
        assert_eq!(old.choices_by_mood, [1, 2, 3, 4]);
        assert_eq!(old.recent_recommendations, 7);
        assert!(old.started);
        assert_eq!(incoming.last().unwrap().choices, 0);
    }
    #[test]
    fn no_implicit_demo_or_system_write_for_disconnected_root() {
        let rx = start(PathBuf::from(":memory:"), None, None, false);
        let u = rx.recv().unwrap().unwrap();
        assert!(u.games.is_empty());
        assert!(u.art.is_empty());
    }
    #[test]
    fn local_sources_are_bounded_and_preserve_xml_and_images() {
        let root = std::env::temp_dir().join(format!("trial-fixture-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let game = root.join("owned.fixture");
        std::fs::write(&game, "not a ROM").unwrap();
        let original="<gameList><game><path>owned.fixture</path><name>Original</name><image>../outside.png</image></game></gameList>";
        let xml = root.join("gamelist.xml");
        std::fs::write(&xml, original).unwrap();
        let before = stamp(&xml).unwrap();
        assert_eq!(sources(&root).unwrap().len(), 1);
        let db = root.join("own.sqlite");
        let rx = start(db.clone(), Some(root.clone()), None, false);
        let update = rx.recv().unwrap().unwrap();
        assert_eq!(update.games.len(), 1);
        assert!(update.art.is_empty());
        assert_eq!(std::fs::read_to_string(&xml).unwrap(), original);
        drop(rx);
        let rx = start(db, Some(root.clone()), None, false);
        assert_eq!(rx.recv().unwrap().unwrap().games.len(), 1);
        drop(rx);
        assert_eq!(stamp(&xml).unwrap(), before);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn explicit_disconnected_path_is_not_replaced() {
        let p = PathBuf::from("/emotional-ui-deliberately-absent");
        assert_eq!(root(Some(p.clone())), Some(p));
    }
    #[test]
    fn cached_systems_publish_first_and_final_without_repeated_catalogs() {
        let root = std::env::temp_dir().join(format!("trial-cached-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        for name in ["one", "two", "three"] {
            let dir = root.join(name);
            std::fs::create_dir(&dir).unwrap();
            std::fs::write(dir.join("owned.fixture"), "not a ROM").unwrap();
            std::fs::write(dir.join("gamelist.xml"), format!("<gameList><game><path>owned.fixture</path><name>{name}</name></game></gameList>")).unwrap();
        }
        let db = root.join("own.sqlite");
        let collect = || {
            start(db.clone(), Some(root.clone()), None, false)
                .into_iter()
                .map(Result::unwrap)
                .collect::<Vec<_>>()
        };
        let first = collect();
        assert_eq!(first.len(), 3);
        assert_eq!(first.last().unwrap().games.len(), 3);
        let cached = collect();
        assert_eq!(cached.len(), 2);
        assert!(cached.iter().all(|u| u.games.len() == 3));
        assert!(cached.last().unwrap().message.contains("3/3"));
        // A changed middle system must still publish its new metadata immediately.
        std::fs::write(
            root.join("three/gamelist.xml"),
            "<gameList><game><path>owned.fixture</path><name>Updated</name></game></gameList>",
        )
        .unwrap();
        let changed = collect();
        assert_eq!(changed.len(), 3);
        assert!(changed[1].games.iter().any(|g| g.title == "Updated"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
