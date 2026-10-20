//! Home is an opaque window. Some software snapshots contain RGB with zero alpha.
//! Export RGB explicitly so normal image viewers do not hide the entire screenshot.
use slint::{Rgba8Pixel, SharedPixelBuffer};
use std::path::Path;

pub fn save(path: &Path, buffer: &SharedPixelBuffer<Rgba8Pixel>) -> image::ImageResult<()> {
    let (pixels, _) = buffer.as_bytes().as_chunks::<4>();
    let rgb = pixels
        .iter()
        .flat_map(|pixel| pixel[..3].iter().copied())
        .collect::<Vec<_>>();
    image::save_buffer(
        path,
        &rgb,
        buffer.width(),
        buffer.height(),
        image::ColorType::Rgb8,
    )
}

/// A Retina window is not a 640x480 capture. Refuse it rather than silently resize.
pub fn save_ui(
    path: &Path,
    buffer: &SharedPixelBuffer<Rgba8Pixel>,
) -> Result<(), Box<dyn std::error::Error>> {
    if buffer.width() != 640 || buffer.height() != 480 {
        return Err(
            "Snapshot must be 640x480 physical pixels; use SLINT_SCALE_FACTOR=1 or the simulator"
                .into(),
        );
    }
    save(path, buffer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshot_with_zero_alpha_keeps_colors_and_becomes_opaque() {
        let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(2, 1);
        buffer
            .make_mut_bytes()
            .copy_from_slice(&[12, 34, 56, 0, 200, 150, 100, 0]);
        let path = std::env::temp_dir().join(format!(
            "emotional-opaque-capture-{}.png",
            std::process::id()
        ));
        save(&path, &buffer).unwrap();
        let decoded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(decoded.as_raw(), &[12, 34, 56, 255, 200, 150, 100, 255]);
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(test)]
mod dimensions_tests {
    use super::*;
    #[test]
    fn retina_export_is_rejected_before_writing() {
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::new(1280, 960);
        let path = std::env::temp_dir().join(format!(
            "emotional-retina-rejected-{}.png",
            std::process::id()
        ));
        assert!(save_ui(&path, &buffer).is_err());
        assert!(!path.exists());
    }
    #[test]
    fn native_size_export_is_opaque_rgb() {
        let buffer = SharedPixelBuffer::<Rgba8Pixel>::new(640, 480);
        let path =
            std::env::temp_dir().join(format!("emotional-native-size-{}.png", std::process::id()));
        save_ui(&path, &buffer).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes[25], 2);
        assert_eq!(image::open(&path).unwrap().width(), 640);
        std::fs::remove_file(path).unwrap();
    }
}
