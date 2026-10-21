// SPDX-License-Identifier: GPL-3.0-only
//! Isolated KMS renderer probe: no library, artwork, audio or emulator access.
//! Run only inside an explicitly authorised, externally recoverable trial.
use slint::ComponentHandle;
use std::time::Duration;

slint::slint! {
    export component GpuProbe inherits Window {
        width:640px; height:480px; background:rgb(24,33,30);
        default-font-family:"DejaVu Sans";
        in-out property <bool> moved:false;
        Text {x:32px;y:32px;width:576px;height:40px;text:"EmotionalUI · prueba de renderizado";
            font-size:24px;font-weight:700;color:#eee6d5;}
        Text {x:32px;y:88px;width:576px;height:48px;
            text:"La identidad GL y las presentaciones DRM se registran por separado.";
            wrap:word-wrap;font-size:16px;color:#bac4bd;}
        Rectangle {x:root.moved?416px:32px;y:184px;width:192px;height:160px;
            border-radius:16px;border-width:2px;border-color:#b3c6bb;
            background:@linear-gradient(110deg,#293e38cc,#392f29ee);
            animate x {duration:360ms;easing:ease-in-out;}
            Text {x:16px;y:16px;width:160px;height:128px;text:"Continuidad\n360 ms";
                font-size:22px;font-weight:600;color:#eee6d5;wrap:word-wrap;
                vertical-alignment:center;horizontal-alignment:center;}
        }
        Text {x:32px;y:392px;width:576px;height:40px;
            text:"Ensayo temporal · cierre automático · sin juegos ni audio";
            font-size:15px;color:#bac4bd;wrap:word-wrap;}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = std::env::var("SLINT_BACKEND").unwrap_or_default();
    if !matches!(backend.as_str(), "linuxkms-femtovg" | "linuxkms-software") {
        return Err("Set SLINT_BACKEND explicitly to linuxkms-femtovg or linuxkms-software".into());
    }
    let mut seconds = 10_u64;
    let mut args = std::env::args().skip(1);
    if let Some(arg) = args.next() {
        if arg != "--seconds" {
            return Err("Usage: gpu-probe [--seconds 2..30]".into());
        }
        seconds = args.next().ok_or("Missing seconds")?.parse()?;
        if !(2..=30).contains(&seconds) || args.next().is_some() {
            return Err("Usage: gpu-probe [--seconds 2..30]".into());
        }
    }
    eprintln!(
        "GPU_PROBE backend={backend} seconds={seconds} requested_transition_ms=360 fps=UNMEASURED"
    );
    let ui = GpuProbe::new()?;
    let weak = ui.as_weak();
    let motion = slint::Timer::default();
    motion.start(
        slint::TimerMode::Repeated,
        Duration::from_millis(900),
        move || {
            if let Some(ui) = weak.upgrade() {
                ui.set_moved(!ui.get_moved());
            }
        },
    );
    slint::Timer::single_shot(Duration::from_secs(seconds), || {
        let _ = slint::quit_event_loop();
    });
    ui.run()?;
    motion.stop();
    eprintln!("GPU_PROBE exit=clean");
    Ok(())
}
