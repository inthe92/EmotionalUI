//! Ambient-only GPU adapter. Slint owns all content, geometry and input.
//! No SDL window, renderer replacement, per-frame readback or continuous timer.
use crate::{DaycycleAssets, FocusDesign, GpuMaterialDesign, Home};
use glow::HasContext;
use slint::{ComponentHandle, GraphicsAPI, Image, RenderingState};
use std::{path::PathBuf, rc::Rc};

thread_local! {
    static LAUNCH_FRAME: std::cell::RefCell<Option<Vec<u8>>> = const { std::cell::RefCell::new(None) };
}
pub fn take_launch_frame() -> Option<Vec<u8>> {
    LAUNCH_FRAME.with(|frame| frame.borrow_mut().take())
}

type Result<T> = std::result::Result<T, String>;
const VERT: &str = include_str!("../ui/gpu-quad.vert");
const AMBIENT: &str = include_str!("../ui/gpu-ambient.frag");
const BLUR: &str = include_str!("../ui/gpu-blur.frag");
const MATERIAL: &str = "in vec2 v_uv;out vec4 color;uniform sampler2D u_tex;uniform sampler2D u_soft;void main(){vec2 uv=vec2(v_uv.x,1.0-v_uv.y);vec3 c=mix(texture(u_tex,uv).rgb,texture(u_soft,uv).rgb,0.80);color=vec4(mix(c,vec3(0.055,0.078,0.11),0.48),1.0);}";

#[derive(Clone, Copy, PartialEq)]
struct Environment {
    night: f32,
    tint: [f32; 3],
    pan: f32,
}
fn night(index: i32) -> f32 {
    match index {
        0 | 6 => 1.0,
        1 => 0.45,
        2 => 0.05,
        3 | 7 => 0.0,
        4 => 0.15,
        _ => 0.65,
    }
}
fn environment(ui: &Home) -> Environment {
    let day = ui.global::<DaycycleAssets>();
    let mix = day.get_day_mix().clamp(0., 1.);
    let season = ui.get_scene_season();
    let tint = match season {
        0 => [0.86, 0.93, 1.0],
        1 => [0.91, 1.0, 0.93],
        2 => [1.0, 0.97, 0.87],
        3 => [1.0, 0.91, 0.84],
        _ => [1.; 3],
    };
    Environment {
        night: night(day.get_palette_index()) * (1. - mix)
            + night(day.get_palette_next_index()) * mix,
        tint,
        pan: if ui.get_reduced() || !ui.get_ambient_motion() {
            0.
        } else {
            ui.get_pan_position() * 6.
        },
    }
}

pub fn select_renderer() -> std::result::Result<(), slint::PlatformError> {
    if std::env::var_os("EMOTIONAL_MATERIAL_GPU").as_deref() != Some(std::ffi::OsStr::new("1"))
        || std::env::var("SLINT_BACKEND")
            .unwrap_or_default()
            .contains("software")
        || cfg!(feature = "simulator")
    {
        return Ok(());
    }
    let selector = slint::BackendSelector::new().renderer_name("femtovg".into());
    if cfg!(target_os = "linux") {
        // Slint 1.13 linuxkms creates its EGL context itself and rejects the
        // graphics-API requirement API. Validate its actual version at setup.
        selector.select()
    } else {
        selector.require_opengl_with_version(3, 2).select()
    }
}

/// Explicit opt-in preserves the installed/software path and renderer selection.
pub fn install(ui: &Home) {
    if std::env::var_os("EMOTIONAL_MATERIAL_GPU").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let weak = ui.as_weak();
    let mut engine: Option<Engine> = None;
    let mut capture = std::env::var_os("EMOTIONAL_GLASS_CAPTURE").map(PathBuf::from);
    let focus_sequence = std::env::var_os("EMOTIONAL_FOCUS_SEQUENCE").map(PathBuf::from);
    let mut frames = 0_u32;
    let mut contexts = 0_i32;
    let focus_timer = slint::Timer::default();
    let focus_epoch = std::time::Instant::now();
    let result = ui.window().set_rendering_notifier(move |state, api| {
        let Some(ui) = weak.upgrade() else { return };
        match state {
            RenderingState::RenderingSetup => {
                focus_timer.stop();
                // A recreated context must never expose IDs from the old context,
                // including when initialization of the replacement fails.
                let style = ui.global::<GpuMaterialDesign>();
                style.set_enabled(false);
                style.set_backdrop(Image::default());
                style.set_material(Image::default());
                let GraphicsAPI::NativeOpenGL { get_proc_address } = api else {
                    eprintln!("GPU_MATERIAL unsupported graphics API; original materials retained");
                    return;
                };
                // SAFETY: Slint invokes the notifier with this window's context current.
                let gl =
                    Rc::new(unsafe { glow::Context::from_loader_function_cstr(get_proc_address) });
                let saved = unsafe { GlState::capture(&gl) };
                unsafe { gl.active_texture(glow::TEXTURE0) };
                let created = unsafe { Engine::new(gl.clone()) };
                unsafe { saved.restore(&gl) };
                match created {
                    Ok(value) => {
                        contexts += 1;
                        ui.global::<GpuMaterialDesign>().set_context_count(contexts);
                        eprintln!(
                            "GPU_MATERIAL ready textures=8 payload_bytes=7987200; fps=unmeasured"
                        );
                        engine = Some(value);
                    }
                    Err(error) => {
                        eprintln!("GPU_MATERIAL init failed: {error}; original materials retained")
                    }
                }
            }
            RenderingState::BeforeRendering => {
                let focus = ui.global::<FocusDesign>();
                focus.set_gpu(engine.is_some());
                if ui.get_reduced() || engine.is_none() {
                    focus_timer.stop();
                    if focus.get_rotation() != 110.0 { focus.set_rotation(110.0); }
                } else if !focus_timer.running() {
                    let weak = ui.as_weak();
                    focus_timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(33), move || {
                        if let Some(ui) = weak.upgrade() {
                            // Only a native gradient angle changes. Backdrop/FBO/textures
                            // remain cached; reduced motion stops this timer at rendering.
                            ui.global::<FocusDesign>().set_rotation((110.0 + focus_epoch.elapsed().as_secs_f32()*45.0).rem_euclid(360.0));
                        }
                    });
                }
                if let Some(renderer) = engine.as_mut() {
                    let params = environment(&ui);
                    if renderer.key == Some(params) {
                        return;
                    }
                    let saved = unsafe { GlState::capture(&renderer.gl) };
                    let images = unsafe { renderer.update(params) };
                    unsafe { saved.restore(&renderer.gl) };
                    match images {
                        Ok((backdrop, material)) => {
                            let style = ui.global::<GpuMaterialDesign>();
                            style.set_backdrop(backdrop);
                            style.set_material(material);
                            style.set_environment_builds(renderer.builds as i32);
                            style.set_enabled(true);
                        }
                        Err(error) => {
                            ui.global::<GpuMaterialDesign>().set_enabled(false);
                            eprintln!(
                                "GPU_MATERIAL draw failed: {error}; original materials retained"
                            );
                            // Do not retry a failed graphics operation on every frame.
                            renderer.key = Some(params);
                        }
                    }
                }
            }
            RenderingState::AfterRendering => {
                frames += 1;
                if ui.get_launch_frame_requested() && !ui.get_launch_frame_ready() {
                    if let Some(renderer) = engine.as_ref() {
                        match unsafe { renderer.frame() } {
                            Ok(image) => LAUNCH_FRAME.with(|frame| *frame.borrow_mut() = Some(image.into_raw())),
                            Err(error) => eprintln!("LAUNCH_FRAME capture unavailable: {error}"),
                        }
                    }
                    ui.set_launch_frame_ready(true);
                }
                if matches!(frames, 16 | 46) {
                    if let (Some(renderer), Some(folder)) = (engine.as_ref(), focus_sequence.as_ref()) {
                        let mut viewport = [0; 4];
                        unsafe { renderer.gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport); }
                        if viewport[2] == 640 && viewport[3] == 480 {
                            if let Err(error) = unsafe { renderer.capture(&folder.join(format!("focus-{frames}.png"))) } {
                                eprintln!("GPU_MATERIAL focus capture failed: {error}");
                            }
                        }
                    }
                }
                ui.global::<GpuMaterialDesign>()
                    .set_rendered_frames(frames as i32);
                if ui.global::<GpuMaterialDesign>().get_capture_ready() {
                    if let (Some(renderer), Some(path)) = (engine.as_ref(), capture.take()) {
                        // Diagnostic-only readback, never part of normal navigation.
                        let mut viewport = [0; 4];
                        unsafe {
                            renderer
                                .gl
                                .get_parameter_i32_slice(glow::VIEWPORT, &mut viewport)
                        };
                        if viewport[2] == 640 && viewport[3] == 480 {
                            if let Err(error) = unsafe { renderer.capture(&path) } {
                                eprintln!("GPU_MATERIAL capture failed: {error}");
                            }
                        } else {
                            eprintln!("GPU_MATERIAL capture requires 640x480 physical pixels");
                        }
                    }
                }
            }
            RenderingState::RenderingTeardown => {
                focus_timer.stop();
                // Do not request property-driven redraws while Winit tears down its
                // mutably borrowed native window. Borrowed Images do not own GL IDs;
                // a recreated context replaces both images in BeforeRendering before
                // Slint can sample them. No rendering occurs after this notification.
                if let Some(renderer) = engine.take() {
                    eprintln!(
                        "GPU_MATERIAL teardown frames={frames} environment_builds={}",
                        renderer.builds
                    );
                    drop(renderer);
                }
            }
            _ => {}
        }
    });
    if let Err(error) = result {
        eprintln!("GPU_MATERIAL unavailable: {error}; original materials retained");
    }
}

/// Arm after application setup, so timing is relative to the event loop.
pub fn arm_capture(ui: &Home) {
    if std::env::var_os("EMOTIONAL_GLASS_CAPTURE").is_some() {
        capture_when_settled(ui.as_weak(), 0);
    }
}

fn capture_when_settled(weak: slint::Weak<Home>, attempt: u32) {
    slint::Timer::single_shot(
        std::time::Duration::from_millis(if attempt == 0 { 1800 } else { 100 }),
        move || {
            use slint::Model;
            let Some(ui) = weak.upgrade() else { return };
            let section = ui.get_motion_section().max(0) as usize;
            let settled = ui.get_page_offsets().row_data(section) == Some(0.0)
                && (ui.get_nav_position() - section as f32).abs() < 0.001;
            if !settled && attempt < 80 {
                capture_when_settled(weak, attempt + 1);
                return;
            }
            if !settled {
                eprintln!("GPU_MATERIAL capture refused: navigation did not settle");
                return;
            }
            eprintln!(
                "GPU_MATERIAL capture settled: section={section} pages={:?}",
                ui.get_page_offsets().iter().collect::<Vec<_>>()
            );
            ui.global::<GpuMaterialDesign>().set_capture_ready(true);
            ui.window().request_redraw();
        },
    );
}

#[derive(Clone, Copy)]
struct Target {
    tex: glow::Texture,
    fbo: glow::Framebuffer,
    w: i32,
    h: i32,
}
struct Engine {
    gl: Rc<glow::Context>,
    programs: Vec<glow::Program>,
    textures: Vec<glow::Texture>,
    targets: Vec<Target>,
    vao: Option<glow::VertexArray>,
    buffer: Option<glow::Buffer>,
    key: Option<Environment>,
    output: usize,
    builds: u32,
}
impl Engine {
    unsafe fn new(gl: Rc<glow::Context>) -> Result<Self> {
        if gl.version().major < 3 {
            return Err("GPU materials require OpenGL ES 3.0 or desktop OpenGL 3.2".into());
        }
        eprintln!(
            "GPU_MATERIAL context {}.{} embedded={}",
            gl.version().major,
            gl.version().minor,
            gl.version().is_embedded
        );
        let mut this = Self {
            gl,
            programs: vec![],
            textures: vec![],
            targets: vec![],
            vao: None,
            buffer: None,
            key: None,
            output: 0,
            builds: 0,
        };
        for frag in [AMBIENT, BLUR, MATERIAL] {
            this.programs.push(program(&this.gl, frag)?);
        }
        let gl = &this.gl;
        this.vao = Some(gl.create_vertex_array()?);
        this.buffer = Some(gl.create_buffer()?);
        gl.bind_vertex_array(this.vao);
        gl.bind_buffer(glow::ARRAY_BUFFER, this.buffer);
        let vertices = [0_f32, 0., 1., 0., 0., 1., 0., 1., 1., 0., 1., 1.];
        let bytes =
            std::slice::from_raw_parts(vertices.as_ptr().cast(), std::mem::size_of_val(&vertices));
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, bytes, glow::STATIC_DRAW);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 8, 0);
        for bytes in [
            include_bytes!("../../assets/materials/gpu/ambient-day.png").as_slice(),
            include_bytes!("../../assets/materials/gpu/ambient-night.png").as_slice(),
        ] {
            let pixels = image::load_from_memory(bytes)
                .map_err(|e| e.to_string())?
                .resize_exact(640, 480, image::imageops::FilterType::Lanczos3)
                .to_rgba8();
            let tex = texture(gl, 640, 480, Some(pixels.as_raw()))?;
            this.textures.push(tex);
        }
        // Double-buffered published images avoid stale Slint image/layer caches.
        for (w, h) in [
            (640, 480),
            (640, 480),
            (640, 480),
            (640, 480),
            (320, 240),
            (320, 240),
        ] {
            let tex = texture(gl, w, h, None)?;
            this.textures.push(tex);
            let fbo = gl.create_framebuffer()?;
            let target = Target { tex, fbo, w, h };
            this.targets.push(target);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(tex),
                0,
            );
            if gl.check_framebuffer_status(glow::FRAMEBUFFER) != glow::FRAMEBUFFER_COMPLETE {
                return Err("Incomplete material framebuffer".into());
            }
        }
        Ok(this)
    }
    unsafe fn pass(&self, p: glow::Program, t: Target) {
        let gl = &self.gl;
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(t.fbo));
        gl.viewport(0, 0, t.w, t.h);
        gl.use_program(Some(p));
        gl.bind_vertex_array(self.vao);
        gl.uniform_4_f32(
            gl.get_uniform_location(p, "u_rect").as_ref(),
            0.,
            0.,
            t.w as f32,
            t.h as f32,
        );
        gl.uniform_2_f32(
            gl.get_uniform_location(p, "u_res").as_ref(),
            t.w as f32,
            t.h as f32,
        );
    }
    unsafe fn sampler(&self, p: glow::Program, name: &str, t: glow::Texture, unit: u32) {
        self.gl.active_texture(glow::TEXTURE0 + unit);
        self.gl.bind_texture(glow::TEXTURE_2D, Some(t));
        self.gl
            .uniform_1_i32(self.gl.get_uniform_location(p, name).as_ref(), unit as i32);
    }
    unsafe fn update(&mut self, key: Environment) -> Result<(Image, Image)> {
        let gl = &self.gl;
        gl.disable(glow::BLEND);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::DEPTH_TEST);
        gl.disable(glow::CULL_FACE);
        self.output = 1 - self.output;
        let back = self.targets[self.output];
        let material = self.targets[2 + self.output];
        let p = self.programs[0];
        self.pass(p, back);
        self.sampler(p, "u_day", self.textures[0], 0);
        self.sampler(p, "u_night", self.textures[1], 1);
        gl.uniform_1_f32(gl.get_uniform_location(p, "u_mix").as_ref(), key.night);
        gl.uniform_1_f32(gl.get_uniform_location(p, "u_pan").as_ref(), key.pan);
        gl.uniform_3_f32(
            gl.get_uniform_location(p, "u_tint").as_ref(),
            key.tint[0],
            key.tint[1],
            key.tint[2],
        );
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
        let p = self.programs[1];
        for (i, t) in self.targets[4..6].iter().copied().enumerate() {
            self.pass(p, t);
            self.sampler(
                p,
                "u_tex",
                if i == 0 {
                    back.tex
                } else {
                    self.targets[4].tex
                },
                0,
            );
            let step = 2.0;
            gl.uniform_2_f32(
                gl.get_uniform_location(p, "u_step").as_ref(),
                if i == 0 { step / t.w as f32 } else { 0. },
                if i == 1 { step / t.h as f32 } else { 0. },
            );
            gl.draw_arrays(glow::TRIANGLES, 0, 6);
        }
        let p = self.programs[2];
        self.pass(p, material);
        self.sampler(p, "u_tex", back.tex, 0);
        self.sampler(p, "u_soft", self.targets[5].tex, 1);
        gl.draw_arrays(glow::TRIANGLES, 0, 6);
        let error = gl.get_error();
        if error != glow::NO_ERROR {
            return Err(format!("OpenGL error {error:#x}"));
        }
        self.key = Some(key);
        self.builds += 1;
        Ok((borrow(back.tex), borrow(material.tex)))
    }
    unsafe fn frame(&self) -> Result<image::RgbaImage> {
        let mut viewport = [0;4];
        self.gl.get_parameter_i32_slice(glow::VIEWPORT,&mut viewport);
        if viewport != [0,0,640,480] { return Err("Launch frame requires 640x480 pixels".into()); }
        let mut bytes = vec![0_u8; 640 * 480 * 4];
        self.gl.read_pixels(
            0,
            0,
            640,
            480,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            glow::PixelPackData::Slice(Some(&mut bytes)),
        );
        if self.gl.get_error() != glow::NO_ERROR { return Err("Framebuffer readback failed".into()); }
        let mut image =
            image::RgbaImage::from_raw(640, 480, bytes).ok_or("Invalid capture size")?;
        image::imageops::flip_vertical_in_place(&mut image);
        Ok(image)
    }
    unsafe fn capture(&self, path: &std::path::Path) -> Result<()> {
        if path.exists() {
            return Err("Refusing to overwrite capture".into());
        }
        let image=self.frame()?;
        image.save(path).map_err(|e| e.to_string())
    }
}
unsafe fn borrow(t: glow::Texture) -> Image {
    slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(t.0, (640, 480).into())
        .origin(slint::BorrowedOpenGLTextureOrigin::BottomLeft)
        .build()
}
unsafe fn texture(
    gl: &glow::Context,
    w: i32,
    h: i32,
    bytes: Option<&[u8]>,
) -> Result<glow::Texture> {
    let tex = gl.create_texture()?;
    gl.bind_texture(glow::TEXTURE_2D, Some(tex));
    for p in [glow::TEXTURE_MIN_FILTER, glow::TEXTURE_MAG_FILTER] {
        gl.tex_parameter_i32(glow::TEXTURE_2D, p, glow::LINEAR as i32);
    }
    for p in [glow::TEXTURE_WRAP_S, glow::TEXTURE_WRAP_T] {
        gl.tex_parameter_i32(glow::TEXTURE_2D, p, glow::CLAMP_TO_EDGE as i32);
    }
    gl.tex_image_2d(
        glow::TEXTURE_2D,
        0,
        glow::RGBA8 as i32,
        w,
        h,
        0,
        glow::RGBA,
        glow::UNSIGNED_BYTE,
        glow::PixelUnpackData::Slice(bytes),
    );
    Ok(tex)
}
unsafe fn program(gl: &glow::Context, frag: &str) -> Result<glow::Program> {
    let prefix = if gl.version().is_embedded {
        "#version 300 es\nprecision highp float;\n"
    } else {
        "#version 150\n"
    };
    let p = gl.create_program()?;
    let mut shaders = Vec::new();
    for (kind, source) in [(glow::VERTEX_SHADER, VERT), (glow::FRAGMENT_SHADER, frag)] {
        let s = match gl.create_shader(kind) {
            Ok(s) => s,
            Err(e) => {
                for s in shaders {
                    gl.delete_shader(s);
                }
                gl.delete_program(p);
                return Err(e);
            }
        };
        gl.shader_source(s, &format!("{prefix}{source}"));
        gl.compile_shader(s);
        if !gl.get_shader_compile_status(s) {
            let e = gl.get_shader_info_log(s);
            gl.delete_shader(s);
            for s in shaders {
                gl.delete_shader(s);
            }
            gl.delete_program(p);
            return Err(e);
        }
        gl.attach_shader(p, s);
        shaders.push(s);
    }
    gl.bind_attrib_location(p, 0, "a_pos");
    gl.link_program(p);
    for s in shaders {
        gl.detach_shader(p, s);
        gl.delete_shader(s);
    }
    if !gl.get_program_link_status(p) {
        let e = gl.get_program_info_log(p);
        gl.delete_program(p);
        return Err(e);
    }
    Ok(p)
}
impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            for t in &self.targets {
                self.gl.delete_framebuffer(t.fbo);
            }
            for t in &self.textures {
                self.gl.delete_texture(*t);
            }
            for p in &self.programs {
                self.gl.delete_program(*p);
            }
            if let Some(v) = self.vao {
                self.gl.delete_vertex_array(v);
            }
            if let Some(b) = self.buffer {
                self.gl.delete_buffer(b);
            }
        }
    }
}

/// Preserve every binding/capability touched by our passes before returning to Slint.
struct GlState {
    draw: Option<glow::Framebuffer>,
    read: Option<glow::Framebuffer>,
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    buffer: Option<glow::Buffer>,
    viewport: [i32; 4],
    active: u32,
    textures: [Option<glow::Texture>; 2],
    enabled: [bool; 4],
}
impl GlState {
    unsafe fn capture(gl: &glow::Context) -> Self {
        let active = gl.get_parameter_i32(glow::ACTIVE_TEXTURE) as u32;
        let mut textures = [None; 2];
        for (i, t) in textures.iter_mut().enumerate() {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            *t = gl.get_parameter_texture(glow::TEXTURE_BINDING_2D);
        }
        gl.active_texture(active);
        let mut viewport = [0; 4];
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
        Self {
            draw: gl.get_parameter_framebuffer(glow::DRAW_FRAMEBUFFER_BINDING),
            read: gl.get_parameter_framebuffer(glow::READ_FRAMEBUFFER_BINDING),
            program: gl.get_parameter_program(glow::CURRENT_PROGRAM),
            vao: gl.get_parameter_vertex_array(glow::VERTEX_ARRAY_BINDING),
            buffer: gl.get_parameter_buffer(glow::ARRAY_BUFFER_BINDING),
            viewport,
            active,
            textures,
            enabled: [
                glow::BLEND,
                glow::SCISSOR_TEST,
                glow::DEPTH_TEST,
                glow::CULL_FACE,
            ]
            .map(|v| gl.is_enabled(v)),
        }
    }
    unsafe fn restore(self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, self.draw);
        gl.bind_framebuffer(glow::READ_FRAMEBUFFER, self.read);
        gl.use_program(self.program);
        gl.bind_vertex_array(self.vao);
        gl.bind_buffer(glow::ARRAY_BUFFER, self.buffer);
        gl.viewport(
            self.viewport[0],
            self.viewport[1],
            self.viewport[2],
            self.viewport[3],
        );
        for (i, t) in self.textures.into_iter().enumerate() {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            gl.bind_texture(glow::TEXTURE_2D, t);
        }
        gl.active_texture(self.active);
        for (cap, enabled) in [
            glow::BLEND,
            glow::SCISSOR_TEST,
            glow::DEPTH_TEST,
            glow::CULL_FACE,
        ]
        .into_iter()
        .zip(self.enabled)
        {
            if enabled {
                gl.enable(cap)
            } else {
                gl.disable(cap)
            }
        }
    }
}
