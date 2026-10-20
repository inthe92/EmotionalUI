//! Pilot evaluation worksheet. Sparse real Atlas metadata, no invented human ratings.
use emotional_core::{
    editorial::{Profile, Signals},
    library::demo_catalog,
    mood::Mood,
    recommendation::{rank_policy, Context},
};
use rusqlite::Connection;
fn main() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/atlas/emotional-atlas-0.2.0-experimental.1.db");
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut q=db.prepare("SELECT g.id,g.title,COALESCE(e.profile,p.profile) FROM games g JOIN profiles p ON p.game_id=g.id LEFT JOIN profiles_experimental e ON e.game_id=g.id ORDER BY g.id").unwrap();
    let games = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .map(|row| {
            let (id, title, text) = row.unwrap();
            let mut g = demo_catalog()[0].clone();
            g.id = id;
            g.title = title;
            g.genre = "unknown".into();
            g.started = false;
            g.favorite = false;
            g.choices = 0;
            g.choices_by_mood = [0; 4];
            g.editorial = toml::from_str::<Profile>(&text).unwrap();
            g.emotions = demo_catalog()[0].emotions.clone();
            g
        })
        .collect::<Vec<_>>();
    let names = [
        "autumn-rain-sunday",
        "summer-afternoon",
        "october-cloud-night",
        "winter-morning",
        "brief-before-leaving",
        "adventure-preference-fixture",
        "arcade-preference-fixture",
        "little-history",
        "returns-to-old-title",
        "several-trials-no-continuation",
    ];
    println!("{{\"scope\":\"Automatic worksheet; not human validation. Popularity baseline is an explicit fixed proxy, not measured popularity. Opt-in source-assisted proposals, not human-reviewed. Identical legacy fixtures and weights.\",\"atlas_version\":\"0.2.0-experimental.1\",\"scenarios\":[");
    for (case, name) in names.into_iter().enumerate() {
        let mut current = games.clone();
        if case == 5 {
            current[3].choices = 10;
        }
        if case == 6 {
            current[6].choices = 10;
        }
        if case == 8 {
            current[12].started = true;
        }
        if case == 9 {
            for g in current.iter_mut().take(5) {
                g.choices = 1;
                g.recent_recommendations = 2;
            }
        }
        let month = if case == 1 {
            7
        } else if case == 3 {
            1
        } else {
            10
        };
        let signals = Signals {
            date: chrono::NaiveDate::from_ymd_opt(2026, month, 18),
            ..Default::default()
        };
        let ctx = Context {
            mood: if case == 3 {
                Mood::Dawn
            } else if case < 2 {
                Mood::Horizon
            } else {
                Mood::Shelter
            },
            minutes: if case == 4 { 10 } else { 30 },
            raining: case == 0,
            slot: case as u64,
            refresh: 0,
        };
        let mut plain = current.clone();
        for g in &mut plain {
            g.editorial = Profile::default();
        }
        let old = rank_policy(&plain, ctx, &signals, true);
        let atlas = rank_policy(&current, ctx, &signals, true);
        let ids = |indices: &[usize]| {
            indices
                .iter()
                .take(2)
                .map(|i| current[*i].id.clone())
                .collect::<Vec<_>>()
        };
        let random = [
            (case * 17 + 7) % current.len(),
            (case * 17 + 19) % current.len(),
        ];
        let proxy = [0, 3];
        let active = current
            .iter()
            .filter(|g| {
                g.editorial.calm.is_some()
                    || g.editorial.intensity.is_some()
                    || !g.editorial.seasons.is_empty()
            })
            .count();
        println!("{}{{\"scenario\":{name:?},\"random_seeded\":{:?},\"popularity_proxy\":{:?},\"existing_rank\":{:?},\"atlas_rank\":{:?},\"active_editorial_games\":{active},\"same_as_existing\":{},\"human_review\":null}}",if case>0{","}else{""},ids(&random),ids(&proxy),ids(&old),ids(&atlas),ids(&old)==ids(&atlas));
    }
    println!("]}}");
}
