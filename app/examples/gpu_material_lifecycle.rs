//! GPU regression using the actual Home and adapter, without library/device I/O.
//! Each step waits for a rendered frame before changing the next condition.
#[path = "../src/gpu_material.rs"]
mod gpu_material;
#[path = "../src/focus.rs"]
mod focus;
use slint::ComponentHandle;
use std::{cell::Cell, rc::Rc, time::Duration};
slint::include_modules!();
fn main() -> Result<(), Box<dyn std::error::Error>> {
    gpu_material::select_renderer()?;
    let ui = Home::new()?;
    focus::install(&ui);
    ui.set_hero_title("Prueba de ciclo de material".into());
    ui.set_art_available(false);
    ui.set_ambient_motion(true);
    ui.global::<DaycycleAssets>().set_palette_index(3);
    ui.global::<DaycycleAssets>().set_palette_next_index(3);
    gpu_material::install(&ui);
    let phase = Rc::new(Cell::new(0));
    let last_frame = Rc::new(Cell::new(0));
    let done = Rc::new(Cell::new(false));
    let poll = slint::Timer::default();
    let weak = ui.as_weak();
    let phase_cb = phase.clone();
    let done_cb = done.clone();
    poll.start(slint::TimerMode::Repeated,Duration::from_millis(80),move||{
        let Some(ui)=weak.upgrade() else{return};
        let style=ui.global::<GpuMaterialDesign>();
        let frame=style.get_rendered_frames();
        if frame<=last_frame.get()||!style.get_enabled(){ui.window().request_redraw();return;}
        last_frame.set(frame);
        match phase_cb.get(){
            0=>{ui.set_pan_position(1.0);ui.window().request_redraw();}
            1=>{assert!(style.get_environment_builds()>=2);ui.global::<DaycycleAssets>().set_palette_index(6);ui.global::<DaycycleAssets>().set_palette_next_index(6);ui.window().request_redraw();}
            2=>{assert!(style.get_environment_builds()>=3);ui.hide().expect("hide Home after rendering");let weak=ui.as_weak();slint::Timer::single_shot(Duration::from_millis(60),move||{if let Some(ui)=weak.upgrade(){ui.show().expect("show Home again");ui.window().request_redraw();}});}
            _=>{assert_eq!(ui.get_hero_title(),"Prueba de ciclo de material");println!("GPU_MATERIAL_LIFECYCLE passed: pan, day/night, rendered hide/show, content retained; contexts={} frames={frame}",style.get_context_count());done_cb.set(true);ui.hide().expect("final hide");slint::quit_event_loop().expect("exit");return;}
        }
        phase_cb.set(phase_cb.get()+1);
    });
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_secs(8), move || {
        if let Some(ui) = weak.upgrade() {
            let _ = ui.hide();
        }
        let _ = slint::quit_event_loop();
    });
    gpu_material::arm_capture(&ui);
    ui.show()?;
    slint::run_event_loop_until_quit()?;
    poll.stop();
    if !done.get() {
        return Err(format!("lifecycle did not finish; phase={}", phase.get()).into());
    }
    Ok(())
}
