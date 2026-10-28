use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};
static SERIAL: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "editorial-art-{}-{}",
            std::process::id(),
            SERIAL.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        fs::create_dir(p.join("library")).unwrap();
        Self(p)
    }
    fn image(&self, name: &str, w: u32, h: u32, role: Role) -> Resource {
        let p = self.0.join("library").join(name);
        let image = RgbaImage::from_fn(w, h, |x, y| {
            Rgba([(x % 256) as u8, (y % 256) as u8, 90, 255])
        });
        if name.ends_with(".jpg") {
            DynamicImage::ImageRgba8(image).to_rgb8().save(&p).unwrap();
        } else {
            image.save(&p).unwrap();
        }
        Resource {
            path: p,
            root: self.0.join("library"),
            role,
        }
    }
    fn request(&self, resources: Vec<Resource>) -> Request {
        Request {
            id: "request".into(),
            game_id: "game:1".into(),
            resources,
            width: 282,
            height: 224,
            presentation: Presentation::Hero,
        }
    }
    fn cache(&self) -> Cache {
        Cache::new(Some(self.0.join("cache")), 1024 * 1024)
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
#[test]
fn missing_art_is_optional() {
    assert!(decode(Path::new("/emotional-ui-missing.png")).is_none());
}
#[test]
fn changing_one_cover_preserves_other_choices_and_never_changes_original_art() {
    let f=Fixture::new();let art=f.image("original.png",600,900,Role::Selected);
    let original=fs::read(&art.path).unwrap();let config=f.0.join("choices.json");
    save_choice_at(&config,"game:a",Some(&art)).unwrap();
    save_choice_at(&config,"game:b",Some(&art)).unwrap();
    save_choice_at(&config,"game:a",None).unwrap();
    let choices=read_choices(&config).unwrap();
    assert_eq!(choices.games.len(),1);assert!(choices.games.contains_key("game:b"));
    assert_eq!(fs::read(&art.path).unwrap(),original);
    fs::write(&config,b"user content that cannot be parsed").unwrap();
    assert!(save_choice_at(&config,"game:b",None).is_err());
    assert_eq!(fs::read(&config).unwrap(),b"user content that cannot be parsed");
}
#[test]
fn provider_download_is_normalized_once_and_corrupted_derivatives_are_not_applied() {
    let f=Fixture::new();let art=f.image("download.png",600,900,Role::Selected);
    let original=fs::read(&art.path).unwrap();
    let (selected,prepared)=prepare_download(&art.path,&art.root).unwrap();
    assert_eq!((prepared.width,prepared.height),(240,360));
    assert!(prepare_download(&art.path,&art.root).is_some());
    fs::write(&selected.path,b"corrupt cache").unwrap();
    assert!(prepare_download(&art.path,&art.root).is_none());
    assert_eq!(fs::read(&art.path).unwrap(),original);
    let wide=f.image("wide.png",900,600,Role::Selected);
    assert!(prepare_download(&wide.path,&wide.root).is_none());
}
#[test]
fn portrait_catalog_spans_systems_and_is_bounded() {
    let mut text = String::new();
    for i in 0..10_000 {
        text.push_str(&format!(
            "[games.\"es:/roms2/system-{}/title-{}.fixture\"]\npath=\"/art/portraits/{}.png\"\nroot=\"/art\"\n",
            i % 20, i, i
        ));
    }
    assert!(text.len() > 256 * 1024);
    assert_eq!(read_overrides(text.as_bytes()).unwrap().games.len(), 10_000);
    assert!(read_overrides(vec![b' '; MAX_OVERRIDE_BYTES as usize + 1].as_slice()).is_none());
    assert!(read_overrides(b"[games.g]\npath='a'\nroot='b'\nunknown=true".as_slice()).is_none());
}
#[test]
fn provider_portrait_wins_both_presentations_and_missing_file_keeps_existing_art() {
    let f = Fixture::new();
    let portrait = f.image("provider.png", 240, 360, Role::Curated);
    let original = f.image("original.png", 640, 360, Role::Hero);
    let mut cache = f.cache();
    let mut request = f.request(vec![portrait.clone(), original]);
    for presentation in [Presentation::Hero, Presentation::Tile] {
        request.presentation = presentation;
        assert_eq!(cache.prepare(&request).unwrap().mode, Mode::Boxart);
    }
    fs::remove_file(portrait.path).unwrap();
    assert_eq!(cache.prepare(&request).unwrap().mode, Mode::Landscape);
}
#[test]
fn original_jpeg_thumbnail_is_decoded_without_new_framework() {
    let f = Fixture::new();
    let r = f.image("test.jpg", 320, 240, Role::Generic);
    let (w, h, data) = decode(&r.path).unwrap();
    assert!(w <= 292 && h <= 232);
    assert_eq!(data.len(), (w * h * 4) as usize);
}
#[test]
fn published_png_is_bounded() {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/materials/gpu/ambient-day.png");
    let (w, h, p) = decode(&p).unwrap();
    assert!(w <= 292 && h <= 232);
    assert_eq!(p.len(), (w * h * 4) as usize);
}
#[test]
fn all_four_states_and_priority() {
    let f = Fixture::new();
    let hero = f.image("hero.png", 640, 360, Role::Hero);
    let boxart = f.image("box.png", 100, 200, Role::Boxart);
    let screen = f.image("screen.png", 320, 240, Role::Screenshot);
    let mut c = f.cache();
    let mut r = f.request(vec![screen.clone(), boxart.clone(), hero]);
    assert_eq!(c.prepare(&r).unwrap().mode, Mode::Landscape);
    r.resources.pop();
    assert_eq!(c.prepare(&r).unwrap().mode, Mode::Boxart);
    r.presentation = Presentation::Tile;
    assert_eq!(c.prepare(&r).unwrap().mode, Mode::Screenshot);
    r.resources = vec![];
    assert!(c.prepare(&r).is_none());
}
#[test]
fn full_boxart_survives_and_stays_opaque() {
    let f = Fixture::new();
    let r = f.request(vec![f.image("narrow.png", 20, 200, Role::Boxart)]);
    let p = f.cache().prepare(&r).unwrap();
    assert_eq!((p.width, p.height), (282, 224));
    assert!(p.pixels.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    let image = RgbaImage::from_raw(p.width, p.height, p.pixels).unwrap();
    assert_eq!(image.get_pixel(130, 8)[0], 0);
    assert_eq!(image.get_pixel(150, 215)[0], 19);
}
#[test]
fn display_corners_are_antialiased_without_mutating_cached_artwork() {
    let f = Fixture::new();
    let r = f.request(vec![f.image("mask.png", 100, 200, Role::Boxart)]);
    let mut cache = f.cache();
    let mut display = cache.prepare(&r).unwrap();
    let original = display.pixels.clone();
    round_display_corners(&mut display, 8);
    assert_eq!(display.pixels[3], 0);
    assert_eq!(
        display.pixels
            [(((display.height / 2) * display.width + display.width / 2) * 4 + 3) as usize],
        255
    );
    assert!(display
        .pixels
        .chunks_exact(4)
        .any(|p| p[3] > 0 && p[3] < 255));
    for (shown, cached) in display.pixels.chunks_exact(4).zip(original.chunks_exact(4)) {
        assert_eq!(&shown[..3], &cached[..3]);
    }
    assert_eq!(cache.prepare(&r).unwrap().pixels, original);
}
#[test]
fn screenshot_uses_nearest_and_bands() {
    let image = DynamicImage::ImageRgba8(RgbaImage::from_fn(2, 2, |x, y| {
        Rgba([
            if x == 0 { 0 } else { 255 },
            if y == 0 { 0 } else { 255 },
            0,
            255,
        ])
    }));
    let p = compose(image, Mode::Screenshot, 10, 8);
    assert_eq!(p.get_pixel(0, 0), &Rgba([32, 38, 34, 255]));
    assert!(p.pixels().all(|p| [0, 32, 255].contains(&p[0])));
}
#[test]
fn cache_miss_memory_hit_disk_hit_and_content_invalidation() {
    let f = Fixture::new();
    let resource = f.image("cover.png", 200, 100, Role::Hero);
    let r = f.request(vec![resource.clone()]);
    let mut c = f.cache();
    let first = c.prepare(&r).unwrap();
    assert!(!first.cache_hit);
    assert!(c.prepare(&r).unwrap().cache_hit);
    let mut c = f.cache();
    assert!(c.prepare(&r).unwrap().cache_hit);
    RgbaImage::from_pixel(200, 100, Rgba([200, 20, 1, 255]))
        .save(&resource.path)
        .unwrap();
    let changed = c.prepare(&r).unwrap();
    assert!(!changed.cache_hit);
    assert_ne!(first.pixels, changed.pixels);
    fs::remove_file(resource.path).unwrap();
    assert!(c.prepare(&r).is_none());
}
#[test]
fn corrupt_or_renamed_non_image_falls_back() {
    let f = Fixture::new();
    let r = f.image("corrupt.png", 100, 200, Role::Generic);
    fs::write(&r.path, b"not png").unwrap();
    assert!(f.cache().prepare(&f.request(vec![r])).is_none());
}
#[test]
fn oversized_dimensions_are_rejected() {
    let f = Fixture::new();
    let r = f.image("wide.png", 2049, 1, Role::Generic);
    assert!(f.cache().prepare(&f.request(vec![r])).is_none());
}
#[test]
fn oversized_file_and_output_are_rejected() {
    let f = Fixture::new();
    let r = f.image("large.png", 10, 10, Role::Generic);
    fs::OpenOptions::new()
        .write(true)
        .open(&r.path)
        .unwrap()
        .set_len(INPUT + 1)
        .unwrap();
    let mut req = f.request(vec![r]);
    assert!(f.cache().prepare(&req).is_none());
    req.width = 641;
    assert!(f.cache().prepare(&req).is_none());
}
#[test]
fn outside_root_is_never_read() {
    let f = Fixture::new();
    let other = Fixture::new();
    let mut r = other.image("foreign.png", 100, 200, Role::Generic);
    r.root = f.0.clone();
    assert!(f.cache().prepare(&f.request(vec![r])).is_none());
}
#[cfg(unix)]
#[test]
fn symlink_escape_is_rejected() {
    let f = Fixture::new();
    let other = Fixture::new();
    let r = other.image("foreign.png", 100, 200, Role::Generic);
    let link = f.0.join("escape.png");
    std::os::unix::fs::symlink(r.path, &link).unwrap();
    assert!(f
        .cache()
        .prepare(&f.request(vec![Resource {
            path: link,
            root: f.0.clone(),
            role: Role::Generic
        }]))
        .is_none());
}
#[test]
fn partial_files_are_not_cache_hits_or_cleanup_targets() {
    let f = Fixture::new();
    let mut c = f.cache();
    fs::create_dir(c.directory.as_ref().unwrap()).unwrap();
    let partial = c.directory.as_ref().unwrap().join(".interrupted.part");
    fs::write(&partial, b"not a valid result").unwrap();
    let r = f.request(vec![f.image("hero.png", 160, 90, Role::Hero)]);
    assert!(!c.prepare(&r).unwrap().cache_hit);
    assert!(partial.exists());
}
#[test]
fn dimensions_and_identity_do_not_alias() {
    let f = Fixture::new();
    let mut r = f.request(vec![f.image("hero.png", 160, 90, Role::Hero)]);
    let mut c = f.cache();
    c.prepare(&r).unwrap();
    r.width = 64;
    r.height = 42;
    assert!(!c.prepare(&r).unwrap().cache_hit);
    r.game_id = "game:2".into();
    assert!(!c.prepare(&r).unwrap().cache_hit);
}
#[test]
fn memory_cache_is_bounded() {
    let f = Fixture::new();
    let mut c = f.cache();
    let mut r = f.request(vec![f.image("hero.png", 160, 90, Role::Hero)]);
    for i in 0..24 {
        r.game_id = i.to_string();
        c.prepare(&r).unwrap();
    }
    assert!(c.memory_bytes <= 4 * 1024 * 1024);
}
#[test]
fn manual_exact_identity_resource_has_priority() {
    let f = Fixture::new();
    let r = f.request(vec![
        f.image("hero.png", 160, 90, Role::Hero),
        f.image("chosen.png", 100, 200, Role::Curated),
    ]);
    assert_eq!(f.cache().prepare(&r).unwrap().mode, Mode::Boxart);
}
#[test]
fn corrupt_preferred_source_does_not_hide_valid_fallback() {
    let f = Fixture::new();
    let bad = f.image("bad.png", 100, 200, Role::Boxart);
    fs::write(&bad.path, b"bad").unwrap();
    let r = f.request(vec![bad, f.image("screen.png", 320, 240, Role::Screenshot)]);
    assert_eq!(f.cache().prepare(&r).unwrap().mode, Mode::Screenshot);
}
#[test]
fn logo_only_retains_typographic_rendering() {
    let f = Fixture::new();
    let r = f.request(vec![f.image("logo.png", 200, 80, Role::Logo)]);
    assert!(f.cache().prepare(&r).is_none());
}
#[test]
fn disk_budget_removes_only_own_regular_cache_files() {
    let f = Fixture::new();
    let c = f.cache();
    let dir = c.directory.as_ref().unwrap();
    fs::create_dir(dir).unwrap();
    fs::write(dir.join("personal.png"), vec![1; 1024 * 1024]).unwrap();
    fs::write(
        dir.join("editorial-v1-0000000000000001.png"),
        vec![1; 800000],
    )
    .unwrap();
    fs::write(
        dir.join("editorial-v1-0000000000000002.png"),
        vec![1; 800000],
    )
    .unwrap();
    c.prune();
    assert!(dir.join("personal.png").exists());
    let total = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().starts_with("editorial-v1-"))
        .map(|e| e.metadata().unwrap().len())
        .sum::<u64>();
    assert!(total <= 1024 * 1024);
}
#[test]
fn misconfigured_cache_never_writes_into_resource_root() {
    let f = Fixture::new();
    let r = f.request(vec![f.image("hero.png", 160, 90, Role::Hero)]);
    let cache = f.0.join("library/cache");
    let mut c = Cache::new(Some(cache.clone()), 1024 * 1024);
    assert!(c.prepare(&r).is_some());
    assert!(!cache.exists());
}
#[test]
fn smaller_named_hero_does_not_win_over_a_valid_resource() {
    let f = Fixture::new();
    let r = f.request(vec![
        f.image("hero.png", 160, 20, Role::Hero),
        f.image("screen.png", 320, 240, Role::Screenshot),
    ]);
    assert_eq!(f.cache().prepare(&r).unwrap().mode, Mode::Screenshot);
}
