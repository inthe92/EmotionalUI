//! Reuse the active room's trusted SVG masks at their exact display size.
//! Software rendering otherwise rasterizes these vectors again on every draw.
use crate::{AtmosphereDesign, DaycycleAssets, Home};
use slint::{ComponentHandle, Model};

/// Two small derived foreground strips, prepared only on a lighting-key change.
/// They reuse the approved painting; no invented scenery or continuous filter.
pub fn refresh_near(ui: &Home) {
    let design = ui.global::<AtmosphereDesign>();
    let day = ui.global::<DaycycleAssets>();
    let keys = [day.get_palette_index(), day.get_palette_next_index()];
    if design.get_near_key() == keys[0] && design.get_near_next_key() == keys[1] {
        return;
    }
    let backgrounds = day.get_backgrounds();
    let mut result = Vec::with_capacity(2);
    for key in keys {
        let prepared = backgrounds
            .row_data(key.max(0) as usize)
            .and_then(|image| image.to_rgba8_premultiplied());
        let Some(buffer) = prepared else {
            result.push(slint::Image::default());
            continue;
        };
        let rgba =
            image::RgbaImage::from_raw(buffer.width(), buffer.height(), buffer.as_bytes().to_vec())
                .unwrap();
        let resized =
            image::imageops::resize(&rgba, 664, 498, image::imageops::FilterType::Triangle);
        let strip = foreground_strip(&resized);
        let pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::clone_from_slice(
            strip.as_raw(),
            strip.width(),
            strip.height(),
        );
        result.push(slint::Image::from_rgba8_premultiplied(pixels));
    }
    design.set_near_rasters(slint::ModelRc::new(slint::VecModel::from(result)));
    design.set_near_key(keys[0]);
    design.set_near_next_key(keys[1]);
}

fn foreground_strip(resized: &image::RgbaImage) -> image::RgbaImage {
    // In the fixed room composition the lower third contains the near table,
    // cushion and floor. A feathered join avoids an arbitrary rectangular seam.
    let mut strip = image::imageops::crop_imm(resized, 0, 345, 664, 153).to_image();
    for y in 0..24 {
        for x in 0..strip.width() {
            for c in &mut strip.get_pixel_mut(x, y).0 {
                *c = (u32::from(*c) * y / 24) as u8;
            }
        }
    }
    strip
}

const SOURCES: [&str; 9] = [
    include_str!("../../assets/scenes/daycycle/interior-light-mask.svg"),
    include_str!("../../assets/scenes/daycycle/vegetation-mask.svg"),
    include_str!("../../assets/scenes/daycycle/weather-window.svg"),
    include_str!("../../assets/scenes/daycycle/rain-window.svg"),
    include_str!("../../assets/scenes/daycycle/snow-window.svg"),
    include_str!("../../assets/scenes/daycycle/halloween-table.svg"),
    include_str!("../../assets/scenes/daycycle/christmas-table.svg"),
    include_str!("../../assets/scenes/daycycle/new-year-table.svg"),
    include_str!("../../assets/scenes/lofi-room-v2/reading-mask.svg"),
];
fn active(index: usize, weather: i32, event: i32) -> bool {
    match index {
        3 => weather == 2,
        4 => weather == 4,
        5..=7 => event == index as i32 - 4,
        _ => true,
    }
}
fn raster(index: usize) -> Result<slint::Image, slint::LoadImageError> {
    // The viewBox remains unchanged; this matches the renderer's 664x498 SVG
    // target, not an upscaled 640x480 bitmap. The reading veil stays 640x480.
    let source = if index == 8 {
        SOURCES[index].to_string()
    } else {
        SOURCES[index]
            .replacen("width=\"640\"", "width=\"664\"", 1)
            .replacen("height=\"480\"", "height=\"498\"", 1)
    };
    let svg = slint::Image::load_from_svg_data(source.as_bytes())?;
    Ok(slint::Image::from_rgba8_premultiplied(
        svg.to_rgba8_premultiplied().expect("trusted SVG raster"),
    ))
}
pub fn refresh(ui: &Home) {
    // A diagnostic baseline only; never stored as a user preference.
    if crate::performance::enabled()
        && std::env::var("EMOTIONAL_PERF_RESOURCES").as_deref() == Ok("svg")
    {
        return;
    }
    let assets = ui.global::<AtmosphereDesign>();
    let weather = ui.get_scene_weather();
    let event = ui.get_scene_decoration();
    if assets.get_scene_raster_ready()
        && assets.get_cached_weather() == weather
        && assets.get_cached_event() == event
    {
        return;
    }
    let previous = assets.get_scene_rasters();
    let mut images = Vec::with_capacity(SOURCES.len());
    for index in 0..SOURCES.len() {
        if !active(index, weather, event) {
            images.push(slint::Image::default());
            continue;
        }
        use slint::Model;
        let existing = previous.row_data(index).unwrap_or_default();
        if existing.size().width > 0 {
            images.push(existing);
            continue;
        }
        match raster(index) {
            Ok(image) => images.push(image),
            Err(error) => {
                eprintln!("Scene mask cache unavailable: {error}");
                assets.set_scene_raster_ready(false);
                return;
            }
        }
    }
    assets.set_scene_rasters(slint::ModelRc::new(slint::VecModel::from(images)));
    assets.set_cached_weather(weather);
    assets.set_cached_event(event);
    assets.set_scene_raster_ready(true);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exterior_layers_do_not_paint_the_foreground_cup_or_window_sill() {
        // Coordinates refer to the approved 640x480 painting. Exercise the
        // actual 664x498 cached SVG raster used by the physical renderer.
        let alpha = |index, x: u32, y: u32| {
            let pixels = raster(index).unwrap().to_rgba8_premultiplied().unwrap();
            let px = x * 664 / 640;
            let py = y * 498 / 480;
            pixels.as_bytes()[((py * pixels.width() + px) * 4 + 3) as usize]
        };
        for index in [1, 2, 3, 4] {
            for (x, y) in [(497, 301), (502, 305), (510, 308), (525, 320), (480, 312)] {
                assert_eq!(
                    alpha(index, x, y),
                    0,
                    "exterior layer {index} covers foreground at ({x},{y})"
                );
            }
        }
        assert!(alpha(1, 400, 250) > 0, "seasonal exterior retained");
        assert!(alpha(2, 400, 250) > 0, "weather exterior retained");
        assert!(
            alpha(0, 502, 305) > 0,
            "cup keeps the room's interior light"
        );
        assert_eq!(
            alpha(0, 400, 250),
            0,
            "glass remains outside the interior tint"
        );
    }
    #[test]
    fn foreground_is_bounded_and_feathered_without_changing_the_source() {
        let source = image::RgbaImage::from_pixel(664, 498, image::Rgba([100, 80, 60, 255]));
        let strip = foreground_strip(&source);
        assert_eq!(strip.dimensions(), (664, 153));
        assert_eq!(strip.get_pixel(0, 0).0, [0, 0, 0, 0]);
        assert_eq!(strip.get_pixel(0, 24).0, [100, 80, 60, 255]);
        assert_eq!(source.get_pixel(0, 345).0, [100, 80, 60, 255]);
        assert!(strip.get_pixel(0, 12)[3] > 0 && strip.get_pixel(0, 12)[3] < 255);
    }
    #[test]
    fn bounded_active_set_and_exact_target_dimensions() {
        for weather in 0..5 {
            for event in 0..4 {
                assert!((0..9).filter(|i| active(*i, weather, event)).count() <= 6);
            }
        }
        for index in 0..9 {
            let size = raster(index).unwrap().size();
            assert_eq!(
                (size.width, size.height),
                if index == 8 { (640, 480) } else { (664, 498) }
            );
        }
    }
}

// Rounded child clipping is rectangular in Slint's software backend. Bake only
// this alpha mask at geometry/scene changes, and reuse the same image while moving.
#[derive(PartialEq)]
struct FrostKey(slint::Image, i32, i32, u32, u32, u32);
thread_local! {
    static FROST: std::cell::RefCell<Vec<(FrostKey, slint::Image)>> = const { std::cell::RefCell::new(Vec::new()) };
}
pub fn install_glass(ui: &Home) {
    ui.global::<AtmosphereDesign>().on_rounded_frost(|source, x, y, width, height, radius| {
        if !(1..=640).contains(&width) || !(1..=480).contains(&height) { return slint::Image::default(); }
        let key = FrostKey(source, x, y, width as u32, height as u32, radius.max(0) as u32);
        FROST.with(|cache| {
            let mut cache = cache.borrow_mut();
            if let Some(index) = cache.iter().position(|(old, _)| old == &key) {
                let entry = cache.remove(index);
                let image = entry.1.clone();
                cache.push(entry);
                return image;
            }
            let Some(rgb) = key.0.to_rgb8() else { return slint::Image::default(); };
            let Some(source) = image::RgbImage::from_raw(rgb.width(), rgb.height(), rgb.as_bytes().to_vec()) else { return slint::Image::default(); };
            let cw = ((width as f32 / 664.0 * source.width() as f32).round() as u32).clamp(1,source.width());
            let ch = ((height as f32 / 498.0 * source.height() as f32).round() as u32).clamp(1,source.height());
            let sx = (((x+12) as f32 / 664.0 * source.width() as f32).round() as i32).clamp(0,(source.width()-cw) as i32) as u32;
            let sy = (((y+9) as f32 / 498.0 * source.height() as f32).round() as i32).clamp(0,(source.height()-ch) as i32) as u32;
            let crop = image::imageops::crop_imm(&source,sx,sy,cw,ch).to_image();
            let pixels = image::imageops::resize(&crop,key.3,key.4,image::imageops::FilterType::Triangle);
            let mut output = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(key.3,key.4);
            for (i,(src,dst)) in pixels.pixels().zip(output.make_mut_slice()).enumerate() {
                let a = rounded_alpha(i as u32 % key.3,i as u32 / key.3,key.3,key.4,key.5);
                *dst = slint::Rgba8Pixel::new(src[0],src[1],src[2],a);
            }
            let image = slint::Image::from_rgba8(output);
            // Bound both count and worst-case RGBA allocation (8 MiB). This is
            // a geometry cache, not a history of animated positions or frames.
            let bytes = |k: &FrostKey| k.3 as usize*k.4 as usize*4;
            while !cache.is_empty() && (cache.len()>=24 || cache.iter().map(|(k,_)| bytes(k)).sum::<usize>()+bytes(&key)>8*1024*1024) { cache.remove(0); }
            cache.push((key,image.clone()));
            image
        })
    });
}
fn rounded_alpha(x:u32,y:u32,width:u32,height:u32,radius:u32)->u8 {
    let r = radius.min(width/2).min(height/2) as f32;
    if r==0.0 { return 255; }
    let mut inside=0;
    for sy in 0..4 { for sx in 0..4 {
        let px=x as f32+(sx as f32+0.5)/4.0;
        let py=y as f32+(sy as f32+0.5)/4.0;
        let dx=px-px.clamp(r,width as f32-r);
        let dy=py-py.clamp(r,height as f32-r);
        if dx*dx+dy*dy<=r*r { inside+=1; }
    }}
    (inside*255/16) as u8
}
#[cfg(test)]
mod glass_tests {
    #[test]
    fn rounded_frost_has_no_square_corner_and_preserves_interior() {
        for (w,h,r) in [(216,56,12),(176,28,14),(448,192,16)] {
            assert_eq!(super::rounded_alpha(0,0,w,h,r),0);
            assert_eq!(super::rounded_alpha(w-1,h-1,w,h,r),0);
            assert_eq!(super::rounded_alpha(w/2,h/2,w,h,r),255);
            for y in 0..h { for x in 0..w {
                assert_eq!(super::rounded_alpha(x,y,w,h,r),super::rounded_alpha(w-x-1,h-y-1,w,h,r));
            }}
        }
    }
}
