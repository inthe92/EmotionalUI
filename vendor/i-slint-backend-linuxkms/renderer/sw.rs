// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0

//! Delegate the rendering to the [`i_slint_core::software_renderer::SoftwareRenderer`]

use i_slint_core::api::PhysicalSize as PhysicalWindowSize;
use i_slint_core::platform::PlatformError;
pub use i_slint_core::software_renderer::SoftwareRenderer;
use i_slint_core::software_renderer::{PremultipliedRgbaColor, RepaintBufferType, TargetPixel};
use std::cell::RefCell;
use std::sync::Arc;
use std::time::Instant;

use crate::display::RenderingRotation;

pub struct SoftwareRendererAdapter {
    renderer: SoftwareRenderer,
    display: Arc<dyn crate::display::swdisplay::SoftwareBufferDisplay>,
    presenter: Arc<dyn crate::display::Presenter>,
    size: PhysicalWindowSize,
    direct_mmap: bool,
    timings: Option<RefCell<KmsTimings>>,
    shadow_xrgb: RefCell<Vec<DumbBufferPixelXrgb888>>,
    shadow_rgb565: RefCell<Vec<i_slint_core::software_renderer::Rgb565Pixel>>,
}

const SOFTWARE_RENDER_SUPPORTED_DRM_FOURCC_FORMATS: &[drm::buffer::DrmFourcc] = &[
    // Preferred formats
    drm::buffer::DrmFourcc::Xrgb8888,
    drm::buffer::DrmFourcc::Argb8888,
    // drm::buffer::DrmFourcc::Bgra8888,
    // drm::buffer::DrmFourcc::Rgba8888,

    // 16-bit formats
    drm::buffer::DrmFourcc::Rgb565,
    // drm::buffer::DrmFourcc::Bgr565,

    // // 4444 formats
    // drm::buffer::DrmFourcc::Argb4444,
    // drm::buffer::DrmFourcc::Abgr4444,
    // drm::buffer::DrmFourcc::Rgba4444,
    // drm::buffer::DrmFourcc::Bgra4444,

    // // Single channel formats
    // drm::buffer::DrmFourcc::Gray8,
    // drm::buffer::DrmFourcc::C8,
    // drm::buffer::DrmFourcc::R8,
    // drm::buffer::DrmFourcc::R16,

    // // Dual channel formats
    // drm::buffer::DrmFourcc::Gr88,
    // drm::buffer::DrmFourcc::Rg88,
    // drm::buffer::DrmFourcc::Gr1616,
    // drm::buffer::DrmFourcc::Rg1616,

    // // 10-bit formats
    // drm::buffer::DrmFourcc::Xrgb2101010,
    // drm::buffer::DrmFourcc::Argb2101010,
    // drm::buffer::DrmFourcc::Abgr2101010,
    // drm::buffer::DrmFourcc::Rgba1010102,
    // drm::buffer::DrmFourcc::Bgra1010102,
    // drm::buffer::DrmFourcc::Rgbx1010102,
    // drm::buffer::DrmFourcc::Bgrx1010102,
];

#[repr(transparent)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct DumbBufferPixelXrgb888(pub u32);

impl From<DumbBufferPixelXrgb888> for PremultipliedRgbaColor {
    #[inline]
    fn from(pixel: DumbBufferPixelXrgb888) -> Self {
        let v = pixel.0;
        PremultipliedRgbaColor {
            red: (v >> 16) as u8,
            green: (v >> 8) as u8,
            blue: (v >> 0) as u8,
            alpha: (v >> 24) as u8,
        }
    }
}

impl From<PremultipliedRgbaColor> for DumbBufferPixelXrgb888 {
    #[inline]
    fn from(pixel: PremultipliedRgbaColor) -> Self {
        Self(
            (pixel.alpha as u32) << 24
                | ((pixel.red as u32) << 16)
                | ((pixel.green as u32) << 8)
                | (pixel.blue as u32),
        )
    }
}

impl TargetPixel for DumbBufferPixelXrgb888 {
    #[inline]
    fn blend(&mut self, color: PremultipliedRgbaColor) {
        // Large room masks contain transparent areas; opaque texels replace
        // the destination. Avoid unpacking/blending/repacking either case.
        if color.alpha == 0 {
            return;
        }
        if color.alpha == 255 {
            *self = color.into();
            return;
        }
        let mut x = PremultipliedRgbaColor::from(*self);
        x.blend(color);
        *self = x.into();
    }

    fn from_rgb(r: u8, g: u8, b: u8) -> Self {
        Self(0xff000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32))
    }

    fn background() -> Self {
        Self(0)
    }
}

impl SoftwareRendererAdapter {
    pub fn new(
        device_opener: &crate::DeviceOpener,
    ) -> Result<Box<dyn crate::fullscreenwindowadapter::FullscreenRenderer>, PlatformError> {
        let display = crate::display::swdisplay::new(
            device_opener,
            SOFTWARE_RENDER_SUPPORTED_DRM_FOURCC_FORMATS,
        )?;

        let (width, height) = display.size();
        let size = i_slint_core::api::PhysicalSize::new(width, height);

        let diagnostics = matches!(
            std::env::var("EMOTIONAL_PERF").as_deref(),
            Ok("full" | "static" | "simple")
        );
        let direct_mmap =
            diagnostics && std::env::var("EMOTIONAL_KMS_DIRECT").as_deref() == Ok("1");
        let renderer = Box::new(Self {
            renderer: SoftwareRenderer::new(),
            display: display.clone(),
            presenter: display.as_presenter(),
            size,
            direct_mmap,
            timings: diagnostics.then(|| RefCell::new(KmsTimings::default())),
            shadow_xrgb: RefCell::new(Vec::new()),
            shadow_rgb565: RefCell::new(Vec::new()),
        });

        eprintln!("Using Software renderer");

        Ok(renderer)
    }
}

impl crate::fullscreenwindowadapter::FullscreenRenderer for SoftwareRendererAdapter {
    fn as_core_renderer(&self) -> &dyn i_slint_core::renderer::Renderer {
        &self.renderer
    }

    fn render_and_present(
        &self,
        rotation: RenderingRotation,
        _draw_mouse_cursor_callback: &dyn Fn(&mut dyn i_slint_core::item_rendering::ItemRenderer),
    ) -> Result<(), PlatformError> {
        let total_at = self.timings.as_ref().map(|_| Instant::now());
        if let Some(timings) = &self.timings {
            let mut timings = timings.borrow_mut();
            timings.render_attempts = timings.render_attempts.saturating_add(1);
        }
        self.display.map_back_buffer(&mut |pixels, age, format| {
            if self.direct_mmap {
                self.renderer.set_repaint_buffer_type(match age {
                    1 => RepaintBufferType::ReusedBuffer,
                    2 => RepaintBufferType::SwappedBuffers,
                    _ => RepaintBufferType::NewBuffer,
                });
            }

            self.renderer.set_rendering_rotation(match rotation {
                RenderingRotation::NoRotation => {
                    i_slint_core::software_renderer::RenderingRotation::NoRotation
                }
                RenderingRotation::Rotate90 => {
                    i_slint_core::software_renderer::RenderingRotation::Rotate90
                }
                RenderingRotation::Rotate180 => {
                    i_slint_core::software_renderer::RenderingRotation::Rotate180
                }
                RenderingRotation::Rotate270 => {
                    i_slint_core::software_renderer::RenderingRotation::Rotate270
                }
            });

            match format {
                drm::buffer::DrmFourcc::Xrgb8888 | drm::buffer::DrmFourcc::Argb8888 => {
                    let buffer: &mut [DumbBufferPixelXrgb888] =
                        bytemuck::cast_slice_mut(pixels.as_mut());
                    if self.direct_mmap {
                        let at = self.timings.as_ref().map(|_| Instant::now());
                        self.renderer.render(buffer, self.size.width as usize);
                        self.record(0, at);
                        return Ok(());
                    }
                    let mut shadow = self.shadow_xrgb.borrow_mut();
                    let fresh = shadow.len() != buffer.len();
                    if fresh {
                        shadow.resize(buffer.len(), DumbBufferPixelXrgb888(0));
                    }
                    self.renderer.set_repaint_buffer_type(if fresh {
                        RepaintBufferType::NewBuffer
                    } else {
                        RepaintBufferType::ReusedBuffer
                    });
                    let at = self.timings.as_ref().map(|_| Instant::now());
                    self.renderer
                        .render(shadow.as_mut_slice(), self.size.width as usize);
                    self.record(0, at);
                    let at = self.timings.as_ref().map(|_| Instant::now());
                    buffer.copy_from_slice(shadow.as_slice());
                    self.record(1, at);
                }
                drm::buffer::DrmFourcc::Rgb565 => {
                    let buffer: &mut [i_slint_core::software_renderer::Rgb565Pixel] =
                        bytemuck::cast_slice_mut(pixels.as_mut());
                    if self.direct_mmap {
                        let at = self.timings.as_ref().map(|_| Instant::now());
                        self.renderer.render(buffer, self.size.width as usize);
                        self.record(0, at);
                        return Ok(());
                    }
                    let mut shadow = self.shadow_rgb565.borrow_mut();
                    let fresh = shadow.len() != buffer.len();
                    if fresh {
                        shadow.resize(buffer.len(), Default::default());
                    }
                    self.renderer.set_repaint_buffer_type(if fresh {
                        RepaintBufferType::NewBuffer
                    } else {
                        RepaintBufferType::ReusedBuffer
                    });
                    let at = self.timings.as_ref().map(|_| Instant::now());
                    self.renderer
                        .render(shadow.as_mut_slice(), self.size.width as usize);
                    self.record(0, at);
                    let at = self.timings.as_ref().map(|_| Instant::now());
                    buffer.copy_from_slice(shadow.as_slice());
                    self.record(1, at);
                }
                _ => {
                    return Err(format!(
                        "Unsupported frame buffer format {format} used with software renderer"
                    )
                    .into())
                }
            }

            Ok(())
        })?;
        let at = self.timings.as_ref().map(|_| Instant::now());
        self.presenter.present()?;
        self.record(2, at);
        self.record(3, total_at);
        if let Some(timings) = &self.timings {
            let due = timings
                .borrow()
                .last_report
                .is_none_or(|t| t.elapsed().as_secs() >= 2);
            if due {
                timings.borrow_mut().last_report = Some(Instant::now());
                self.report_timings();
            }
        }
        Ok(())
    }

    fn size(&self) -> i_slint_core::api::PhysicalSize {
        self.size
    }
    fn diagnostic_redraw_request(&self, coalesced: bool) {
        if let Some(timings) = &self.timings {
            let mut timings = timings.borrow_mut();
            timings.redraw_requests = timings.redraw_requests.saturating_add(1);
            if coalesced {
                timings.redraw_coalesced = timings.redraw_coalesced.saturating_add(1);
            }
        }
    }
    fn diagnostic_session_end(&self) {
        self.report_timings();
        self.display.finish_diagnostics();
    }
}

// Bounded opt-in diagnostics, aggregated during rendering and on adapter drop.
// These durations measure CPU raster/copy/present scheduling, never LCD scanout.
const METRIC_LIMIT: usize = 16384;
#[derive(Default)]
struct KmsTimings {
    samples: [Vec<f64>; 4],
    redraw_requests: u64,
    redraw_coalesced: u64,
    render_attempts: u64,
    raster_completed: u64,
    dropped: usize,
    last_report: Option<Instant>,
}
impl SoftwareRendererAdapter {
    fn record(&self, index: usize, at: Option<Instant>) {
        if let (Some(timings), Some(at)) = (&self.timings, at) {
            let elapsed = at.elapsed().as_secs_f64() * 1000.;
            let mut timings = timings.borrow_mut();
            if index == 0 {
                timings.raster_completed = timings.raster_completed.saturating_add(1);
            }
            if timings.samples[index].len() < METRIC_LIMIT {
                timings.samples[index].push(elapsed);
            } else {
                timings.dropped += 1;
            }
        }
    }
}
impl SoftwareRendererAdapter {
    fn report_timings(&self) {
        let Some(timings) = &self.timings else {
            return;
        };
        let timings = timings.borrow();
        println!(
            "KMS_PERF target={} physical_frame_times=UNMEASURED input_to_photon=UNMEASURED",
            if self.direct_mmap {
                "direct_mmap"
            } else {
                "shadow_ram"
            }
        );
        println!("KMS_PERF redraw_requests={} coalesced_requests={} backend_render_attempts={} raster_completed={} scope=WindowAdapter_requests_not_all_property_invalidations",timings.redraw_requests,timings.redraw_coalesced,timings.render_attempts,timings.raster_completed);
        for (index, name) in [
            "raster_ms",
            "copy_to_mmap_ms",
            "present_scheduling_ms_NOT_LCD",
            "backend_draw_copy_submit_ms_NOT_LCD",
        ]
        .iter()
        .enumerate()
        {
            let mut samples = timings.samples[index].clone();
            if samples.is_empty() {
                continue;
            }
            samples.sort_by(f64::total_cmp);
            let n = samples.len();
            let median = samples[(n * 50).div_ceil(100).saturating_sub(1)];
            let p95 = samples[(n * 95).div_ceil(100).saturating_sub(1)];
            let p99 = if n >= 100 {
                format!("{:.3}", samples[(n * 99).div_ceil(100).saturating_sub(1)])
            } else {
                "UNAVAILABLE_LT100".into()
            };
            println!("KMS_PERF metric={name} n={n} median={median:.3} p95={p95:.3} max={:.3} total={:.3} p99={p99}", samples[n-1], samples.iter().sum::<f64>());
        }
        println!("KMS_PERF dropped_samples={}", timings.dropped);
        self.display.report_presentation_diagnostics();
    }
}

impl Drop for SoftwareRendererAdapter {
    fn drop(&mut self) {
        self.report_timings();
        self.display.finish_diagnostics();
    }
}
