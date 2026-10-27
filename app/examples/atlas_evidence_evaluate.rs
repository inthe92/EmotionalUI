//! Deterministic before/after worksheet using the production ranking policy.
//! The installed collection and usage are synthetic, not the owner's library.
use emotional_core::{
    editorial::{Profile, Signals},
    library::{demo_catalog, Emotions, Game},
    mood::Mood,
    recommendation::{rank_policy, Context},
};
use rusqlite::{Connection, OpenFlags};
use std::{collections::BTreeMap, path::Path};

fn catalog(path: &Path) -> (String, Vec<Game>) {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let version: String = db
        .query_row("SELECT value FROM meta WHERE key='version'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let mut q = db.prepare("SELECT g.id,g.title,e.profile FROM games g JOIN profiles_experimental e ON e.game_id=g.id ORDER BY g.id").unwrap();
    let games = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .map(|r| {
            let (id, title, profile) = r.unwrap();
            let mut g = demo_catalog()[0].clone();
            g.id = id;
            g.title = title;
            g.genre = "unknown".into();
            g.available = true;
            g.started = false;
            g.favorite = false;
            g.choices = 0;
            g.choices_by_mood = [0; 4];
            g.recent_recommendations = 0;
            // Identical fixed legacy controls, not researched game attributes or duration.
            g.session_minutes = 30;
            g.emotions = Emotions {
                calm: 50,
                intensity: 50,
                nostalgia: 50,
                mystery: 50,
                exploration: 50,
                tension: 50,
                joy: 50,
                concentration: 50,
                immersion: 50,
            };
            g.editorial = toml::from_str::<Profile>(&profile).unwrap();
            g.editorial.validate().unwrap();
            g
        })
        .collect();
    (version, games)
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    assert_eq!(args.len(), 2, "expected BEFORE.db AFTER.db");
    let (before_version, before) = catalog(Path::new(&args[0]));
    let (after_version, after) = catalog(Path::new(&args[1]));
    assert_eq!(
        before.iter().map(|g| &g.id).collect::<Vec<_>>(),
        after.iter().map(|g| &g.id).collect::<Vec<_>>()
    );
    let cases = [
        "rain-quiet",
        "brief-10",
        "high-energy",
        "low-energy",
        "returning-metroid",
        "limited-installed",
        "repeat-penalty",
        "unpersonalized",
    ];
    println!("{{\"before_version\":{before_version:?},\"after_version\":{after_version:?},\"synthetic_installed_collection\":true,\"human_review\":null,\"scenarios\":[");
    for (case, name) in cases.into_iter().enumerate() {
        let ctx = Context {
            mood: if case == 0 {
                Mood::Shelter
            } else {
                Mood::Horizon
            },
            minutes: if case == 1 { 10 } else { 30 },
            raining: case == 0,
            slot: 17,
            refresh: 0,
        };
        let signals = Signals {
            date: chrono::NaiveDate::from_ymd_opt(2026, 10, 24),
            energy_preference: if case == 2 {
                Some(90)
            } else if case == 3 {
                Some(20)
            } else {
                None
            },
            ..Default::default()
        };
        let select = |input: &[Game]| {
            let mut games = input.to_vec();
            for g in &mut games {
                if case == 4 && g.id == "atlas:snes:000005" {
                    g.started = true;
                }
                if case == 5 {
                    g.available = [
                        "atlas:snes:000001",
                        "atlas:snes:000007",
                        "atlas:snes:000046",
                        "atlas:snes:000047",
                    ]
                    .contains(&g.id.as_str());
                }
                if case == 6 && ["atlas:snes:000001", "atlas:snes:000005"].contains(&g.id.as_str())
                {
                    g.recent_recommendations = 2;
                }
            }
            let ranked = rank_policy(&games, ctx, &signals, case != 7);
            let chosen = ranked
                .iter()
                .take(2)
                .map(|i| games[*i].id.clone())
                .collect::<Vec<_>>();
            assert!(chosen
                .iter()
                .all(|id| games.iter().any(|g| g.available && &g.id == id)));
            (chosen, games.iter().filter(|g| g.available).count())
        };
        let (old, installed) = select(&before);
        let (new, _) = select(&after);
        let input = BTreeMap::from([
            ("minutes", ctx.minutes.to_string()),
            ("raining", ctx.raining.to_string()),
            (
                "energy",
                signals
                    .energy_preference
                    .map_or("unknown".into(), |v| v.to_string()),
            ),
        ]);
        println!("{}{{\"scenario\":{name:?},\"installed_count\":{installed},\"inputs\":{input:?},\"before\":{old:?},\"after\":{new:?}}}", if case>0 {","} else {""});
    }
    println!("]}}");
}
