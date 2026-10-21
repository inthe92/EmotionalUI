// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: GPL-3.0-only OR LicenseRef-Slint-Royalty-free-2.0 OR LicenseRef-Slint-Software-3.0

use std::{cell::RefCell, num::NonZeroU32, rc::Rc, time::Instant};

use i_slint_core::item_rendering::ItemRenderer;
use i_slint_core::platform::PlatformError;
use i_slint_renderer_femtovg::FemtoVGRendererExt;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use glutin::{
    context::{ContextApi, ContextAttributesBuilder},
    display::GetGlDisplay,
    prelude::*,
    surface::{SurfaceAttributesBuilder, WindowSurface},
};

use crate::display::{gbmdisplay::GbmDisplay, Presenter, RenderingRotation};
use crate::drmoutput::DrmOutput;

pub struct FemtoVGRendererAdapter {
    renderer:
        i_slint_renderer_femtovg::FemtoVGRenderer<i_slint_renderer_femtovg::opengl::OpenGLBackend>,
    gbm_display: Rc<GbmDisplay>,
    timings: Option<Rc<RefCell<GpuTimings>>>,
}

struct GlContextWrapper {
    glutin_context: glutin::context::PossiblyCurrentContext,
    glutin_surface: glutin::surface::Surface<glutin::surface::WindowSurface>,
    gbm_display: Rc<GbmDisplay>,
    timings: Option<Rc<RefCell<GpuTimings>>>,
}

// Opt-in CPU wall times. Neither a draw call nor eglSwapBuffers proves GPU
// completion; the shared DRM diagnostics independently count kernel flips.
struct GpuTimings {
    samples: [Vec<f64>; 5],
    completed: u64,
    dropped: u64,
    last_report: Instant,
    stages: Option<Vec<[u64; 6]>>,
    current_stage: [u64; 6],
    pending_request: Option<u64>,
    redraws: u64,
    coalesced: u64,
    stage_dropped: u64,
    finished: bool,
}
impl GpuTimings {
    fn new() -> Self {
        Self {
            samples: Default::default(),
            completed: 0,
            dropped: 0,
            last_report: Instant::now(),
            stages: (std::env::var("EMOTIONAL_PERF_TRACE").as_deref() == Ok("1"))
                .then(Vec::new),
            current_stage: [0; 6],
            pending_request: None,
            redraws: 0,
            coalesced: 0,
            stage_dropped: 0,
            finished: false,
        }
    }
    fn redraw(&mut self, coalesced: bool) {
        self.redraws += 1;
        self.coalesced += u64::from(coalesced);
        if self.stages.is_some() && self.pending_request.is_none() {
            self.pending_request = Some(monotonic_us());
        }
    }
    fn stage(&mut self, index: usize) {
        if self.stages.is_none() {
            return;
        }
        if index == 1 {
            self.current_stage = [0; 6];
            self.current_stage[0] = self.pending_request.take().unwrap_or(0);
        }
        self.current_stage[index] = monotonic_us();
        if index == 5 {
            let rows = self.stages.as_mut().unwrap();
            if rows.len() < 16384 {
                rows.push(self.current_stage);
            } else {
                self.stage_dropped += 1;
            }
        }
    }
    fn finish(&mut self) {
        if self.finished {
            return;
        }
        self.finished = true;
        if let Some(rows) = &self.stages {
            // One bounded write at shutdown, no frame-by-frame disk output.
            println!("KMS_GPU_STAGES format=request:begin:swap:waited:swapped:submitted unit=monotonic_us redraws={} coalesced={} dropped={} rows={rows:?}", self.redraws,self.coalesced,self.stage_dropped);
        }
    }
    fn record(&mut self, index: usize, at: Instant) {
        if self.samples[index].len() < 16384 {
            self.samples[index].push(at.elapsed().as_secs_f64() * 1000.);
        } else {
            self.dropped = self.dropped.saturating_add(1);
        }
    }
    fn report(&self) {
        println!("KMS_GPU completed_render_calls={} dropped_samples={} gpu_execution_time=UNMEASURED gpu_utilization=UNMEASURED input_to_photon=UNMEASURED",self.completed,self.dropped);
        let names = [
            "render_cpu_including_swap_and_previous_flip_wait_ms",
            "drm_submit_cpu_ms",
            "total_render_and_submit_cpu_ms",
            "previous_flip_wait_cpu_ms",
            "egl_swap_cpu_ms_NOT_gpu_finish",
        ];
        for (name, samples) in names.into_iter().zip(&self.samples) {
            if samples.is_empty() {
                continue;
            }
            let mut sorted = samples.clone();
            sorted.sort_by(f64::total_cmp);
            let n = sorted.len();
            let percentile = |p: usize| sorted[(n * p).div_ceil(100).saturating_sub(1)];
            let p99 = if n >= 100 {
                format!("{:.3}", percentile(99))
            } else {
                "UNAVAILABLE_LT100".into()
            };
            println!(
                "KMS_GPU metric={name} n={n} median={:.3} p95={:.3} p99={p99} max={:.3}",
                percentile(50),
                percentile(95),
                sorted[n - 1]
            );
        }
    }
}

fn monotonic_us() -> u64 {
    #[repr(C)]
    struct Timespec { seconds: i64, nanos: i64 }
    unsafe extern "C" { fn clock_gettime(clock: i32, value: *mut Timespec) -> i32; }
    let mut value = Timespec { seconds: 0, nanos: 0 };
    if unsafe { clock_gettime(1, &mut value) } == 0 {
        value.seconds as u64 * 1_000_000 + value.nanos as u64 / 1000
    } else { 0 }
}

fn log_gl_identity(display: &glutin::display::Display) {
    // EGL context is current here. Query strings once, without glFinish or a
    // driver override. A successfully created EGL context alone is insufficient.
    let symbol = std::ffi::CString::new("glGetString").unwrap();
    let address = display.get_proc_address(&symbol);
    if address.is_null() {
        eprintln!(
            "KMS_GL identity=UNAVAILABLE glGetString_missing=true hardware_acceleration=UNVERIFIED"
        );
        return;
    }
    let get_string: unsafe extern "system" fn(u32) -> *const u8 =
        unsafe { std::mem::transmute(address) };
    let query = |name| {
        let value = unsafe { get_string(name) };
        if value.is_null() {
            "UNAVAILABLE".to_owned()
        } else {
            unsafe { std::ffi::CStr::from_ptr(value.cast()) }
                .to_string_lossy()
                .into_owned()
        }
    };
    let vendor = query(0x1F00);
    let renderer = query(0x1F01);
    let version = query(0x1F02);
    let lower = renderer.to_ascii_lowercase();
    let known_software = [
        "llvmpipe",
        "softpipe",
        "swrast",
        "swiftshader",
        "software rasterizer",
    ]
    .iter()
    .any(|name| lower.contains(name));
    eprintln!("KMS_GL vendor={vendor:?} renderer={renderer:?} version={version:?} known_software_renderer={known_software} gpu_finish_timing=UNMEASURED");
}

impl GlContextWrapper {
    fn new(
        gbm_display: &Rc<GbmDisplay>,
        timings: Option<Rc<RefCell<GpuTimings>>>,
    ) -> Result<Self, PlatformError> {
        let (width, height) = gbm_display.drm_output.size();
        let width: std::num::NonZeroU32 = width.try_into().map_err(|_| {
            format!(
                "Attempting to create window surface with an invalid width: {}",
                width
            )
        })?;
        let height: std::num::NonZeroU32 = height.try_into().map_err(|_| {
            format!(
                "Attempting to create window surface with an invalid height: {}",
                height
            )
        })?;

        let display_handle = gbm_display.display_handle().unwrap();
        let window_handle = gbm_display.window_handle().unwrap();

        let gl_display = unsafe {
            glutin::display::Display::new(
                display_handle.as_raw(),
                glutin::display::DisplayApiPreference::Egl,
            )
            .map_err(|e| format!("Error creating EGL display: {e}"))?
        };

        let config_template = gbm_display.config_template_builder().build();

        let config = unsafe {
            gl_display
                .find_configs(config_template)
                .map_err(|e| format!("Error locating EGL configs: {e}"))?
                .filter(|config| gbm_display.filter_gl_config(config))
                .reduce(|accum, config| {
                    let transparency_check = config.supports_transparency().unwrap_or(false)
                        & !accum.supports_transparency().unwrap_or(false);

                    if transparency_check || config.num_samples() < accum.num_samples() {
                        config
                    } else {
                        accum
                    }
                })
                .ok_or("Unable to find suitable GL config")?
        };

        let gles_context_attributes = ContextAttributesBuilder::new()
            .with_context_api(ContextApi::Gles(Some(glutin::context::Version {
                major: 2,
                minor: 0,
            })))
            .build(Some(window_handle.as_raw()));

        let fallback_context_attributes =
            ContextAttributesBuilder::new().build(Some(window_handle.as_raw()));

        let not_current_gl_context = unsafe {
            gl_display
                .create_context(&config, &gles_context_attributes)
                .or_else(|_| gl_display.create_context(&config, &fallback_context_attributes))
                .map_err(|e| format!("Error creating EGL context: {e}"))?
        };

        let attrs = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window_handle.as_raw(),
            width,
            height,
        );

        let surface = unsafe {
            config
                .display()
                .create_window_surface(&config, &attrs)
                .map_err(|e| format!("Error creating EGL window surface: {e}"))?
        };

        let context = not_current_gl_context.make_current(&surface)
        .map_err(|glutin_error: glutin::error::Error| -> PlatformError {
            format!("FemtoVG Renderer: Failed to make newly created OpenGL context current: {glutin_error}")
            .into()
    })?;

        log_gl_identity(&gl_display);
        // Attempt A is opt-in for a measured trial. DRM EVENT pageflips and
        // their buffer-lifetime wait stay unchanged; no async/tearing flip.
        match std::env::var("EMOTIONAL_GPU_SWAP").as_deref() {
            Ok("immediate") if timings.is_some() => {
                surface.set_swap_interval(&context, glutin::surface::SwapInterval::DontWait)
                    .map_err(|error| format!("GPU trial swap interval unavailable: {error}"))?;
                eprintln!("KMS_GPU_SWAP requested=0 api_accepted=true effective_interval=DRIVER_DEPENDENT drm_vblank_flip=UNCHANGED");
            }
            Err(std::env::VarError::NotPresent) => {
                eprintln!("KMS_GPU_SWAP requested=UPSTREAM_DEFAULT drm_vblank_flip=UNCHANGED");
            }
            _ => return Err("EMOTIONAL_GPU_SWAP only accepts immediate in a performance trial".into()),
        }
        Ok(Self {
            glutin_context: context,
            glutin_surface: surface,
            gbm_display: gbm_display.clone(),
            timings,
        })
    }
}

unsafe impl i_slint_renderer_femtovg::opengl::OpenGLInterface for GlContextWrapper {
    fn ensure_current(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.glutin_context.is_current() {
            self.glutin_context
                .make_current(&self.glutin_surface)
                .map_err(|glutin_error| -> PlatformError {
                    format!("FemtoVG: Error making context current: {glutin_error}").into()
                })?;
        }
        Ok(())
    }

    fn swap_buffers(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Make sure the in-flight font-buffer from the previous swap_buffers call has been
        // posted to the screen.
        if let Some(timings) = &self.timings { timings.borrow_mut().stage(2); }
        let at = self.timings.as_ref().map(|_| Instant::now());
        self.gbm_display.drm_output.wait_for_page_flip();
        if let (Some(timings), Some(at)) = (&self.timings, at) {
            timings.borrow_mut().record(3, at);
            timings.borrow_mut().stage(3);
        }
        let at = self.timings.as_ref().map(|_| Instant::now());
        self.glutin_surface
            .swap_buffers(&self.glutin_context)
            .map_err(|glutin_error| -> PlatformError {
                format!("FemtoVG: Error swapping buffers: {glutin_error}").into()
            })?;
        if let (Some(timings), Some(at)) = (&self.timings, at) {
            timings.borrow_mut().record(4, at);
            timings.borrow_mut().stage(4);
        }
        Ok(())
    }

    fn resize(
        &self,
        _width: NonZeroU32,
        _height: NonZeroU32,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ignore resize requests
        Ok(())
    }

    fn get_proc_address(&self, name: &std::ffi::CStr) -> *const std::ffi::c_void {
        self.glutin_context.display().get_proc_address(name)
    }
}

impl FemtoVGRendererAdapter {
    pub fn new(
        device_opener: &crate::DeviceOpener,
    ) -> Result<Box<dyn crate::fullscreenwindowadapter::FullscreenRenderer>, PlatformError> {
        let drm_output = DrmOutput::new(device_opener)?;
        let egl_display = Rc::new(crate::display::gbmdisplay::GbmDisplay::new(drm_output)?);
        let timings = matches!(
            std::env::var("EMOTIONAL_PERF").as_deref(),
            Ok("full" | "static" | "simple")
        )
        .then(|| Rc::new(RefCell::new(GpuTimings::new())));

        let renderer = Box::new(Self {
            renderer: i_slint_renderer_femtovg::FemtoVGRenderer::new(GlContextWrapper::new(
                &egl_display,
                timings.clone(),
            )?)?,
            gbm_display: egl_display,
            timings,
        });

        eprintln!("Using FemtoVG OpenGL renderer");

        Ok(renderer)
    }
}

impl crate::fullscreenwindowadapter::FullscreenRenderer for FemtoVGRendererAdapter {
    fn diagnostic_redraw_request(&self, coalesced: bool) {
        if let Some(timings) = &self.timings { timings.borrow_mut().redraw(coalesced); }
    }
    fn as_core_renderer(&self) -> &dyn i_slint_core::renderer::Renderer {
        &self.renderer
    }

    fn render_and_present(
        &self,
        rotation: RenderingRotation,
        draw_mouse_cursor_callback: &dyn Fn(&mut dyn ItemRenderer),
    ) -> Result<(), PlatformError> {
        let size = self.size();
        if let Some(timings) = &self.timings { timings.borrow_mut().stage(1); }
        let total = self.timings.as_ref().map(|_| Instant::now());
        let at = self.timings.as_ref().map(|_| Instant::now());
        self.renderer.render_transformed_with_post_callback(
            rotation.degrees(),
            rotation.translation_after_rotation(size),
            size,
            Some(&|item_renderer| {
                draw_mouse_cursor_callback(item_renderer);
            }),
        )?;
        if let (Some(timings), Some(at)) = (&self.timings, at) {
            timings.borrow_mut().record(0, at);
        }
        let at = self.timings.as_ref().map(|_| Instant::now());
        self.gbm_display.present()?;
        if let Some(timings) = &self.timings {
            let mut timings = timings.borrow_mut();
            timings.stage(5);
            timings.completed = timings.completed.saturating_add(1);
            if let Some(at) = at {
                timings.record(1, at);
            }
            if let Some(total) = total {
                timings.record(2, total);
            }
            if timings.last_report.elapsed().as_secs() >= 2 {
                timings.report();
                self.gbm_display
                    .drm_output
                    .report_presentation_diagnostics();
                timings.last_report = Instant::now();
            }
        }
        Ok(())
    }
    fn size(&self) -> i_slint_core::api::PhysicalSize {
        let (width, height) = self.gbm_display.drm_output.size();
        i_slint_core::api::PhysicalSize::new(width, height)
    }
    fn diagnostic_session_end(&self) {
        if let Some(timings) = &self.timings {
            timings.borrow().report();
            timings.borrow_mut().finish();
            self.gbm_display
                .drm_output
                .finish_presentation_diagnostics();
        }
    }
}

impl Drop for FemtoVGRendererAdapter {
    fn drop(&mut self) {
        if let Some(timings) = &self.timings {
            timings.borrow().report();
            timings.borrow_mut().finish();
            self.gbm_display
                .drm_output
                .finish_presentation_diagnostics();
        }
    }
}
