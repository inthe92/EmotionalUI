//! Reproducible local CPU/SQLite benchmark. Never a hardware FPS measurement.
#[path = "../src/artwork.rs"]
#[allow(dead_code)]
mod artwork;
#[path = "../src/store.rs"]
#[allow(dead_code)]
mod store;
use emotional_core::{
    library::demo_catalog,
    mood::Mood,
    recommendation::{rank, restore, Context},
};
use std::time::Instant;
fn median(mut values: Vec<f64>) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}
fn measure(mut run: impl FnMut(), repeats: usize) -> f64 {
    let values = (0..repeats)
        .map(|_| {
            let t = Instant::now();
            run();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect::<Vec<_>>();
    median(values)
}
fn main() {
    println!("{{\"scope\":\"local-linux-userspace-not-rk3566\",\"cases\":[");
    for (case, n) in [1000, 5000, 10000].into_iter().enumerate() {
        let demo = demo_catalog();
        let games = (0..n)
            .map(|i| {
                let mut g = demo[i % 6].clone();
                g.id = format!("synthetic-{i:06}");
                g.title = format!("Synthetic game {i:06}");
                g
            })
            .collect::<Vec<_>>();
        let ctx = Context {
            mood: Mood::Shelter,
            minutes: 30,
            raining: true,
            slot: 42,
            refresh: 0,
        };
        let ranked = rank(&games, ctx);
        let saved = ranked
            .iter()
            .map(|i| games[*i].id.clone())
            .collect::<Vec<_>>();
        let rank_ms = measure(
            || {
                std::hint::black_box(rank(&games, ctx));
            },
            15,
        );
        let restore_ms = measure(
            || {
                std::hint::black_box(restore(&games, &saved, ctx));
            },
            7,
        );
        let legacy_ms = measure(
            || {
                let ranked = rank(&games, ctx);
                let found = saved
                    .iter()
                    .filter_map(|id| ranked.iter().copied().find(|i| games[*i].id == *id))
                    .collect::<Vec<_>>();
                std::hint::black_box(found);
            },
            3,
        );
        let clone_ms = measure(
            || {
                std::hint::black_box(games.clone());
            },
            15,
        );
        let mut changed = games.clone();
        changed[0].choices += 1;
        let worker = store::Persistence::start(
            store::Store::open(std::path::Path::new(":memory:")).unwrap(),
        );
        let t = Instant::now();
        worker.save(&changed, "benchmark", saved.clone()).unwrap();
        let enqueue_ms = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        drop(worker);
        let flush_ms = t.elapsed().as_secs_f64() * 1000.0;
        let db_path =
            std::env::temp_dir().join(format!("emotional-bench-{}-{n}.sqlite", std::process::id()));
        {
            let worker = store::Persistence::start(store::Store::open(&db_path).unwrap());
            worker.save(&games, "bench", saved.clone()).unwrap();
        }
        let db = store::Store::open(&db_path).unwrap();
        let sqlite_load_ms = measure(
            || {
                let mut loaded = games.clone();
                db.load(&mut loaded).unwrap();
                std::hint::black_box(loaded);
            },
            7,
        );
        let history = emotional_core::product::History {
            sessions: games
                .iter()
                .enumerate()
                .map(|(i, g)| emotional_core::product::Session {
                    game_id: g.id.clone(),
                    started_utc: i as i64,
                    duration_seconds: None,
                })
                .collect(),
            completed: vec![],
        };
        let history_ms = measure(
            || {
                std::hint::black_box(history.latest(&games));
            },
            3,
        );
        let visible_rows_ms = measure(
            || {
                std::hint::black_box(
                    games
                        .iter()
                        .skip(n / 2)
                        .take(6)
                        .map(|g| g.title.clone())
                        .collect::<Vec<_>>(),
                );
            },
            100,
        );
        drop(db);
        std::fs::remove_file(db_path).unwrap();
        println!("{}{{\"games\":{n},\"rank_median_ms\":{rank_ms:.4},\"restore_median_ms\":{restore_ms:.4},\"legacy_restore_median_ms\":{legacy_ms:.4},\"full_snapshot_clone_median_ms\":{clone_ms:.4},\"enqueue_ms\":{enqueue_ms:.4},\"worker_flush_ms\":{flush_ms:.4},\"sqlite_load_with_clone_median_ms\":{sqlite_load_ms:.4},\"history_latest_median_ms\":{history_ms:.4},\"six_visible_rows_median_ms\":{visible_rows_ms:.6}}}",if case==0 {""}else{","});
    }
    println!("]}}");
}
