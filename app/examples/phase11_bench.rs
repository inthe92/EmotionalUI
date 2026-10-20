//! Synthetic, reproducible algorithm comparison; not an editorial or RG353V validation.
#[path = "../src/artwork.rs"]
#[allow(dead_code)]
mod artwork;
#[path = "../src/store.rs"]
#[allow(dead_code)]
mod store;
use emotional_core::{
    editorial::{Profile, Signals},
    library::demo_catalog,
    mood::Mood,
    recommendation::{rank, rank_policy, Context},
    reserve::{fingerprint, Reserve},
};
use std::time::Instant;
fn ms(mut f: impl FnMut()) -> f64 {
    let mut x = (0..7)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect::<Vec<_>>();
    x.sort_by(f64::total_cmp);
    x[3]
}
fn main() {
    println!("{{\"scope\":\"synthetic local IDs with 52 real experimental profile records repeated, not installed games or physical/human evaluation\",\"cases\":[");
    for (case, n) in [100, 1000, 5000, 10000].into_iter().enumerate() {
        let demo = demo_catalog();
        let games = (0..n)
            .map(|i| {
                let mut g = demo[i % 6].clone();
                g.id = format!("fixture-{i:06}");
                g.started = false;
                g.favorite = false;
                g.editorial = Profile::default();
                g
            })
            .collect::<Vec<_>>();
        let mut enriched = games.clone();
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../assets/atlas/emotional-atlas-0.2.0-experimental.1.db");
        let db = rusqlite::Connection::open_with_flags(
            source,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut q=db.prepare("SELECT COALESCE(e.profile,p.profile) FROM profiles p LEFT JOIN profiles_experimental e ON e.game_id=p.game_id ORDER BY p.game_id").unwrap();
        let profiles = q
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|s| toml::from_str::<Profile>(&s.unwrap()).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(profiles.len(), 52);
        for (i, g) in enriched.iter_mut().enumerate() {
            g.editorial = profiles[i % profiles.len()].clone();
        }
        let ctx = Context {
            mood: Mood::Shelter,
            minutes: 30,
            raining: true,
            slot: 42,
            refresh: 0,
        };
        let signals = Signals {
            date: chrono::NaiveDate::from_ymd_opt(2026, 10, 18),
            ..Default::default()
        };
        let legacy = ms(|| {
            std::hint::black_box(rank(&games, ctx));
        });
        let atlas = ms(|| {
            std::hint::black_box(rank_policy(&enriched, ctx, &signals, true));
        });
        let prep = ms(|| {
            std::hint::black_box(Reserve::prepare(
                &enriched,
                "experimental-profile-multiplicity",
                Some(1),
                true,
            ));
        });
        let mut cache = Reserve::prepare(
            &enriched,
            "experimental-profile-multiplicity",
            Some(1),
            true,
        );
        let selected = ms(|| {
            std::hint::black_box(cache.select(&enriched, ctx, &signals, true, "night-rain", false));
        });
        let key = ms(|| {
            std::hint::black_box(fingerprint(
                &enriched,
                "experimental-profile-multiplicity",
                true,
            ));
        });
        let db = store::Store::open(std::path::Path::new(":memory:")).unwrap();
        db.save_reserve(&cache).unwrap();
        let query = ms(|| {
            std::hint::black_box(db.reserve().unwrap());
        });
        let clone = ms(|| {
            std::hint::black_box(enriched.clone());
        });
        let mut prefs = std::collections::BTreeMap::new();
        prefs.insert(enriched[n - 1].id.clone(), 1);
        let personal = ms(|| {
            std::hint::black_box(emotional_core::recommendation::rank_affinity(
                &enriched, ctx, &signals, true, &prefs,
            ));
        });
        println!("{}{{\"games\":{n},\"legacy_rank_ms\":{legacy:.4},\"atlas_rank_ms\":{atlas:.4},\"personal_rank_ms\":{personal:.4},\"prepare_ms\":{prep:.4},\"prepared_select_ms\":{selected:.4},\"fingerprint_ms\":{key:.4},\"sqlite_cache_read_ms\":{query:.4},\"catalog_clone_ms\":{clone:.4},\"pool_size\":{},\"pool_id_payload_bytes\":{}}}",if case>0{","}else{""},cache.pool.len(),cache.pool.iter().map(|s|s.len()).sum::<usize>());
    }
    println!("]}}");
}
