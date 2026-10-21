// SPDX-License-Identifier: GPL-3.0-only
//! Diagnostic A: one prepared opaque RGB image, no product UI or library.
//! Same KMS/FemtoVG backend and DRM diagnostics. No driver/swap overrides.
use slint::{ComponentHandle, Image, Rgb8Pixel, SharedPixelBuffer};
use std::{cell::Cell, rc::Rc, time::Duration};

slint::slint! {
    export component PipelineProbe inherits Window {
        width:640px; height:480px; background:rgb(24,33,30);
        in property <image> prepared;
        in-out property <length> offset:-192px;
        animate offset {duration:360ms;easing:ease-in-out;}
        Image {x:root.offset;y:160px;width:192px;height:160px;
            source:root.prepared;image-fit:fill;}
    }
}
fn monotonic_us() -> u64 {
    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Timespec {
            seconds: i64,
            nanos: i64,
        }
        unsafe extern "C" {
            fn clock_gettime(clock: i32, value: *mut Timespec) -> i32;
        }
        let mut value = Timespec {
            seconds: 0,
            nanos: 0,
        };
        if unsafe { clock_gettime(1, &mut value) } == 0 {
            return value.seconds as u64 * 1_000_000 + value.nanos as u64 / 1000;
        }
    }
    0
}
fn seconds(args: impl Iterator<Item = String>) -> Result<u64, Box<dyn std::error::Error>> {
    let args: Vec<_> = args.collect();
    if args.is_empty() {
        return Ok(24);
    }
    if args.len() != 2 || args[0] != "--seconds" {
        return Err("Usage: gpu-pipeline-probe [--seconds 2..30]".into());
    }
    let seconds = args[1].parse()?;
    if !(2..=30).contains(&seconds) {
        return Err("Duration must be 2..30 seconds".into());
    }
    Ok(seconds)
}
fn prepared_surface() -> Image {
    let mut pixels = SharedPixelBuffer::<Rgb8Pixel>::new(192, 160);
    for (i, pixel) in pixels.make_mut_slice().iter_mut().enumerate() {
        let x = (i % 192) as u8;
        let y = (i / 192) as u8;
        *pixel = Rgb8Pixel::new(70 + x / 4, 85 + y / 4, 80 + x / 8);
    }
    Image::from_rgb8(pixels)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seconds = seconds(std::env::args().skip(1))?;
    if std::env::var("SLINT_BACKEND").as_deref() != Ok("linuxkms-femtovg") {
        return Err("Diagnostic A requires explicit SLINT_BACKEND=linuxkms-femtovg".into());
    }
    if std::env::var_os("EMOTIONAL_GPU_SWAP").is_some() {
        return Err("Diagnostic A must retain upstream swap policy".into());
    }
    let ui = PipelineProbe::new()?;
    ui.set_prepared(prepared_surface());
    let active = Rc::new(Cell::new(None::<(u32, f32)>));
    let render_active = active.clone();
    let render_ui = ui.as_weak();
    ui.window().set_rendering_notifier(move |phase, _| {
        if matches!(phase, slint::RenderingState::AfterRendering) {
            if let (Some((id, target)), Some(ui)) = (render_active.get(), render_ui.upgrade()) {
                if ui.get_offset() == target {
                    println!("PIPELINE_MOVE_END id={id} kernel_us={} offset={target} source=after_render_before_submit physical_presentation=BACKEND_ONLY", monotonic_us());
                    render_active.set(None);
                }
            }
        }
    })?;
    let weak = ui.as_weak();
    let generation = Rc::new(Cell::new(0u32));
    let motion = slint::Timer::default();
    motion.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(900),
        move || {
            let Some(ui) = weak.upgrade() else {
                return;
            };
            let n = generation.get() + 1;
            generation.set(n);
            let target = if n % 2 == 1 { 640.0 } else { -192.0 };
            println!(
                "PIPELINE_MOVE_BEGIN id={n} kernel_us={} duration_ms=360 from={} target={target}",
                monotonic_us(),
                ui.get_offset()
            );
            active.set(Some((n, target)));
            ui.set_offset(target);
        },
    );
    println!("PIPELINE_PROBE kind=A one_rgb_surface=192x160 resolution=640x480 seconds={seconds} nominal_motion_ms=360 driver_swap_policy=UNCHANGED measurement=DRM_CONFIRMATIONS_ONLY");
    slint::Timer::single_shot(Duration::from_secs(seconds), || {
        let _ = slint::quit_event_loop();
    });
    ui.run()?;
    motion.stop();
    println!("PIPELINE_PROBE exit=clean");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duration_is_bounded_and_rejects_other_arguments() {
        assert_eq!(seconds(Vec::<String>::new().into_iter()).unwrap(), 24);
        assert!(seconds(["--seconds".into(), "31".into()].into_iter()).is_err());
        assert!(seconds(["--trial".into()].into_iter()).is_err());
    }
}
