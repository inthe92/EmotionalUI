//! CPU/cache userspace measurements with owned procedural pixels. No microSD/GPU claims.
#[path = "../src/artwork.rs"]
#[allow(dead_code)]
mod artwork;
use artwork::*;
use std::{collections::BTreeMap, fs, time::Instant};
fn median(mut f: impl FnMut(), n: usize) -> f64 {
    let mut samples = (0..n)
        .map(|_| {
            let t = Instant::now();
            f();
            t.elapsed().as_secs_f64() * 1000.0
        })
        .collect::<Vec<_>>();
    samples.sort_by(f64::total_cmp);
    samples[n / 2]
}
fn main() {
    let root = std::env::temp_dir().join(format!("emotional-art-bench-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let library = root.join("library");
    fs::create_dir(&library).unwrap();
    let path = library.join("owned.png");
    image::RgbImage::from_fn(600, 900, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 70])
    })
    .save(&path)
    .unwrap();
    let r = Request {
        id: "visible".into(),
        game_id: "owned:1".into(),
        resources: vec![Resource {
            path,
            root: library,
            role: Role::Boxart,
        }],
        width: 282,
        height: 224,
        presentation: Presentation::Hero,
    };
    let cold = median(
        || {
            assert!(Cache::new(None, 1024 * 1024).prepare(&r).is_some());
        },
        7,
    );
    let dir = root.join("cache");
    Cache::new(Some(dir.clone()), 1024 * 1024)
        .prepare(&r)
        .unwrap();
    let disk = median(
        || {
            assert!(
                Cache::new(Some(dir.clone()), 1024 * 1024)
                    .prepare(&r)
                    .unwrap()
                    .cache_hit
            );
        },
        15,
    );
    let mut c = Cache::new(Some(dir), 1024 * 1024);
    c.prepare(&r).unwrap();
    let memory = median(
        || {
            assert!(c.prepare(&r).unwrap().cache_hit);
        },
        31,
    );
    let mut scenarios = Vec::new();
    for count in [100, 1000, 5000, 10000] {
        let index = (0..count)
            .map(|i| (format!("owned:{i:05}"), r.resources.clone()))
            .collect::<BTreeMap<_, _>>();
        let key = format!("owned:{:05}", count / 2);
        let lookup = median(
            || {
                std::hint::black_box(index.get(&key).unwrap());
            },
            101,
        );
        scenarios.push(format!(
            "{{\"games\":{count},\"visible_lookup_ms\":{lookup},\"queued_images_max\":1}}"
        ));
    }
    println!("{{\"scope\":\"local CPU and filesystem, not RG353V or microSD\",\"source\":[600,900],\"output\":[282,224],\"cold_composition_median_ms\":{cold},\"disk_hit_median_ms\":{disk},\"memory_hit_including_source_validation_median_ms\":{memory},\"memory_cache_cap_bytes\":4194304,\"decoded_source_limit_bytes\":25165824,\"scenarios\":[{}]}}",scenarios.join(","));
    fs::remove_dir_all(root).unwrap();
}
