//! One bounded base-painting composition. No UI text, masks or parallax are baked.
use crate::{AtmosphereDesign, DaycycleAssets, Home};
use slint::{ComponentHandle, Image, Model, Rgb8Pixel, SharedPixelBuffer};
use std::cell::RefCell;

#[derive(Clone, PartialEq)]
struct Key {
    first: Image,
    second: Image,
    alpha: u8,
    viewport: (u32, u32),
    scale: u32,
}
#[derive(Default)]
struct Cache {
    entry: Option<(Key, Image)>,
    builds: i32,
    hits: i32,
}
impl Cache {
    fn resolve(&mut self, key: Key) -> Option<Image> {
        if let Some((old, image)) = &self.entry {
            if *old == key {
                self.hits += 1;
                return Some(image.clone());
            }
        }
        let _timing = crate::performance::span("scene_cache_prepare_ms");
        let size = key.first.size();
        // Only the trusted 4:3 base paintings, never arbitrary library artwork.
        if size != key.second.size() || size.width != 640 || size.height != 480 {
            self.entry = None;
            return None;
        }
        self.entry = None;
        let image = if key.alpha == 0 {
            key.first.clone()
        } else if key.alpha == 255 {
            key.second.clone()
        } else {
            let first = key.first.to_rgb8()?;
            let second = key.second.to_rgb8()?;
            let mut output = SharedPixelBuffer::<Rgb8Pixel>::new(640, 480);
            for ((a, b), pixel) in first
                .as_slice()
                .iter()
                .zip(second.as_slice())
                .zip(output.make_mut_slice())
            {
                *pixel = Rgb8Pixel::new(
                    mix(a.r, b.r, key.alpha),
                    mix(a.g, b.g, key.alpha),
                    mix(a.b, b.b, key.alpha),
                );
            }
            Image::from_rgb8(output)
        };
        self.builds += 1;
        self.entry = Some((key, image.clone()));
        Some(image)
    }
}

// Match the existing RGB source-over operation, including its two integer
// roundings. Precompose at SOURCE size: the renderer keeps its existing scaling.
fn mix(first: u8, second: u8, alpha: u8) -> u8 {
    (u16::from(first) * u16::from(255 - alpha) / 255 + u16::from(second) * u16::from(alpha) / 255)
        as u8
}

thread_local! { static CACHE: RefCell<Cache> = RefCell::new(Cache::default()); }
#[derive(Default)]
struct NearCache {
    entry: Option<(Key, Image)>,
}
impl NearCache {
    fn resolve(&mut self, key: Key) -> Option<Image> {
        if let Some((old, image)) = &self.entry {
            if old == &key {
                return Some(image.clone());
            }
        }
        self.entry = None;
        let _timing = crate::performance::span("near_cache_prepare_ms");
        let first = key.first.to_rgba8_premultiplied()?;
        let second = key.second.to_rgba8_premultiplied()?;
        if (first.width(), first.height()) != (664, 153)
            || (second.width(), second.height()) != (664, 153)
        {
            return None;
        }
        let mut output = SharedPixelBuffer::<Rgb8Pixel>::new(664, 129);
        for ((a, b), pixel) in first.as_slice()[664 * 24..]
            .iter()
            .zip(second.as_slice()[664 * 24..].iter())
            .zip(output.make_mut_slice())
        {
            if a.a != 255 || b.a != 255 {
                return None;
            }
            *pixel = Rgb8Pixel::new(
                mix(a.r, b.r, key.alpha),
                mix(a.g, b.g, key.alpha),
                mix(a.b, b.b, key.alpha),
            );
        }
        let image = Image::from_rgb8(output);
        self.entry = Some((key, image.clone()));
        Some(image)
    }
}
thread_local! { static NEAR: RefCell<NearCache> = RefCell::new(NearCache::default()); }
pub fn refresh(ui: &Home) {
    let design = ui.global::<AtmosphereDesign>();
    if crate::performance::enabled()
        && std::env::var("EMOTIONAL_SCENE_CACHE").as_deref() == Ok("off")
    {
        design.set_base_cache_ready(false);
        design.set_near_cache_ready(false);
        return;
    }
    let day = ui.global::<DaycycleAssets>();
    let sources = day.get_backgrounds();
    let Some(first) = sources.row_data(day.get_palette_index().clamp(0, 7) as usize) else {
        return;
    };
    let Some(second) = sources.row_data(day.get_palette_next_index().clamp(0, 7) as usize) else {
        return;
    };
    let size = ui.window().size();
    let key = Key {
        first,
        second,
        alpha: (day.get_day_mix().clamp(0., 1.) * 255.) as u8,
        viewport: (size.width, size.height),
        scale: ui.window().scale_factor().to_bits(),
    };
    let alpha = key.alpha;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let image = cache.resolve(key);
        design.set_base_cache_ready(image.is_some());
        if let Some(image) = image {
            design.set_base_cache(image);
        } else {
            design.set_base_cache(Image::default());
        }
        design.set_base_cache_builds(cache.builds);
        design.set_base_cache_hits(cache.hits);
    });
    if crate::performance::enabled()
        && std::env::var("EMOTIONAL_SCENE_CACHE").as_deref() == Ok("base")
    {
        design.set_near_cache_ready(false);
        return;
    }
    let near = design.get_near_rasters();
    let near_key = Key {
        first: near.row_data(0).unwrap_or_default(),
        second: near.row_data(1).unwrap_or_default(),
        alpha,
        viewport: (size.width, size.height),
        scale: ui.window().scale_factor().to_bits(),
    };
    NEAR.with(|cache| {
        let result = cache.borrow_mut().resolve(near_key);
        design.set_near_cache_ready(result.is_some());
        design.set_near_cache(result.unwrap_or_default());
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn painting(rgb: [u8; 3]) -> Image {
        let mut buffer = SharedPixelBuffer::<Rgb8Pixel>::new(640, 480);
        buffer
            .make_mut_slice()
            .fill(Rgb8Pixel::new(rgb[0], rgb[1], rgb[2]));
        Image::from_rgb8(buffer)
    }
    fn key() -> Key {
        Key {
            first: painting([20, 60, 100]),
            second: painting([200, 180, 160]),
            alpha: 127,
            viewport: (640, 480),
            scale: 1f32.to_bits(),
        }
    }
    #[test]
    fn base_generation_reuse_and_resource_palette_scale_invalidation() {
        let mut cache = Cache::default();
        let original = key();
        let image = cache.resolve(original.clone()).unwrap();
        assert_eq!((image.size().width, image.size().height), (640, 480));
        assert_eq!(
            image.to_rgb8().unwrap().as_slice()[0],
            Rgb8Pixel::new(mix(20, 200, 127), mix(60, 180, 127), mix(100, 160, 127))
        );
        assert!(cache.resolve(original.clone()).unwrap() == image);
        assert_eq!((cache.builds, cache.hits), (1, 1));
        let mut changed = original.clone();
        changed.alpha = 128;
        cache.resolve(changed.clone()).unwrap();
        changed.first = painting([2, 4, 6]);
        cache.resolve(changed.clone()).unwrap();
        changed.viewport = (1280, 960);
        cache.resolve(changed.clone()).unwrap();
        changed.scale = 2f32.to_bits();
        cache.resolve(changed).unwrap();
        assert_eq!(cache.builds, 5); // One live entry; no growing library of scenes.
    }
    #[test]
    fn exact_endpoints_and_failure_never_returns_stale_scene() {
        let mut cache = Cache::default();
        let mut k = key();
        k.alpha = 0;
        assert!(cache.resolve(k.clone()).unwrap() == k.first);
        k.alpha = 255;
        assert!(cache.resolve(k.clone()).unwrap() == k.second);
        k.second = Image::default();
        assert!(cache.resolve(k).is_none());
        assert!(cache.entry.is_none());
    }
    #[test]
    fn opaque_near_layer_reuses_and_rejects_translucent_or_missing_pixels() {
        fn strip(alpha: u8) -> Image {
            let mut b = SharedPixelBuffer::<slint::Rgba8Pixel>::new(664, 153);
            b.make_mut_slice()
                .fill(slint::Rgba8Pixel::new(30, 40, 50, alpha));
            Image::from_rgba8_premultiplied(b)
        }
        let mut cache = NearCache::default();
        let mut k = key();
        k.first = strip(255);
        k.second = strip(255);
        let a = cache.resolve(k.clone()).unwrap();
        assert_eq!((a.size().width, a.size().height), (664, 129));
        assert!(cache.resolve(k.clone()).unwrap() == a);
        k.alpha = 50;
        assert!(cache.resolve(k.clone()).unwrap() != a);
        k.second = strip(254);
        assert!(cache.resolve(k.clone()).is_none());
        assert!(cache.entry.is_none());
        k.first = Image::default();
        assert!(cache.resolve(k).is_none());
    }
    #[test]
    fn opaque_mix_matches_software_target_for_all_opacities() {
        use slint::platform::software_renderer::{PremultipliedRgbaColor, TargetPixel};
        for a in 0..=255 {
            let mut target = Rgb8Pixel::new(19, 120, 239);
            target.blend(PremultipliedRgbaColor {
                red: (201u16 * a as u16 / 255) as u8,
                green: (72u16 * a as u16 / 255) as u8,
                blue: (7u16 * a as u16 / 255) as u8,
                alpha: a,
            });
            assert_eq!(
                target,
                Rgb8Pixel::new(mix(19, 201, a), mix(120, 72, a), mix(239, 7, a))
            );
        }
    }
}
