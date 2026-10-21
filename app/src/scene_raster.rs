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
