//! Static visual poses of the actual Home; does not simulate navigation or devices.
#[path = "../src/focus.rs"]
mod focus;
#[path = "../src/gpu_material.rs"]
mod gpu_material;
#[path = "../src/settings_browser.rs"]
mod settings_browser;
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::time::Duration;
slint::include_modules!();
fn numbers(values: Vec<f32>) -> ModelRc<f32> {
    ModelRc::new(VecModel::from(values))
}
fn strings(values: &[&str]) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        values.iter().map(|s| (*s).into()).collect::<Vec<_>>(),
    ))
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let case = std::env::args()
        .nth(1)
        .ok_or("focus preview requires a pose")?;
    gpu_material::select_renderer()?;
    let ui = Home::new()?;
    focus::install(&ui);
    ui.set_reduced(std::env::var_os("EMOTIONAL_PREVIEW_ANIMATE").is_none());
    ui.set_clock("16:00".into());
    ui.set_ambient_label("Vista de comprobación · datos de muestra".into());
    ui.set_hero_title("Un lugar al que volver".into());
    ui.set_play_label("Elegir".into());
    ui.set_alt_one("Jardín de nubes".into());
    ui.set_alt_two("Órbita de papel".into());
    ui.set_recommendation_count(3);
    ui.set_art_available(true);
    ui.set_dock_cursor(-1);
    let dark = std::env::var_os("EMOTIONAL_PREVIEW_DARK").is_some();
    ui.global::<FocusDesign>().set_light(!dark);
    ui.global::<DaycycleAssets>().set_palette_index(if dark { 6 } else { 3 });
    ui.global::<DaycycleAssets>().set_palette_next_index(if dark { 6 } else { 3 });
    let page = match case.as_str() {
        "library" => 2,
        "settings" | "darkos" | "darkos-network" | "darkos-confirm" => 3,
        _ => 1,
    };
    let offsets = (0..4)
        .map(|i| {
            if i == page {
                0.
            } else if i < page {
                -640.
            } else {
                640.
            }
        })
        .collect::<Vec<_>>();
    ui.set_page_offsets(numbers(offsets.clone()));
    ui.set_page_secondary_offsets(numbers(offsets));
    ui.set_nav_position(page as f32);
    ui.set_motion_section(page);
    match case.as_str() {
        "card" => ui.set_selected(0),
        "secondary" => ui.set_selected(1),
        "dock" => ui.set_dock_cursor(2),
        "library" => {
            ui.set_library_open(true);
            ui.set_library_titles(strings(&[
                "Un lugar al que volver",
                "Jardín de nubes",
                "Órbita de papel",
            ]));
            ui.set_library_offset(0);
            ui.set_library_index(1);
        }
        "settings" => {
            ui.set_menu_open(true);
            ui.set_menu_index(5);
        }
        "darkos" | "darkos-network" | "darkos-confirm" => {
            let root=std::env::var("EMOTIONAL_SETTINGS_TOOL_ROOT")?;
            let entries=emotional_services::system_options::discover(std::path::Path::new(&root))?;
            let mut browser=settings_browser::Browser::default();
            if case=="darkos-network" { browser.activate(1,&entries); }
            if case=="darkos-confirm" { browser.show_tool("Wifi.sh",&entries,true); }
            let rows=browser.rows(&entries);
            ui.set_ambient_label("".into());
            ui.set_menu_open(true);ui.set_options_open(true);ui.set_menu_index(0);
            ui.set_settings_title(browser.title().into());
            ui.set_settings_hint(browser.hint(0,&entries).into());
            ui.set_menu_titles(ModelRc::new(VecModel::from(rows.iter().take(6).map(|r|r.0.clone().into()).collect::<Vec<SharedString>>())));
            ui.set_menu_values(ModelRc::new(VecModel::from(rows.iter().take(6).map(|r|r.1.clone().into()).collect::<Vec<SharedString>>())));
        }
        "audio" => {
            ui.set_audio_open(true);
            ui.set_modal_kind(3);
            ui.set_modal_progress(1.);
            ui.set_audio_index(2);
            ui.set_audio_values(strings(&[
                "Silencio",
                "30 %",
                "20 %",
                "20 %",
                "Activados",
                "20 %",
                "",
            ]));
        }
        "game" => {
            ui.set_game_options_open(true);
            ui.set_modal_kind(2);
            ui.set_modal_progress(1.);
            ui.set_game_option_title("Un lugar al que volver".into());
            ui.set_game_option_labels(strings(&["Fijar partida", "Retirar sugerencia", "Volver"]));
            ui.set_game_option_index(1);
        }
        "status" => ui.set_system_focused(true),
        "system" => {
            ui.set_system_open(true);
            ui.set_modal_kind(1);
            ui.set_modal_progress(1.);
            ui.set_system_selected(2);
            ui.set_system_values(strings(&[
                "Sin verificar",
                "Sin verificar",
                "30 %",
                "70 %",
                "80 %",
            ]));
        }
        _ => return Err("unknown focus pose".into()),
    }
    gpu_material::install(&ui);
    gpu_material::arm_capture(&ui);
    let weak = ui.as_weak();
    slint::Timer::single_shot(Duration::from_secs(3), move || {
        if let Some(ui) = weak.upgrade() {
            if let Ok(path) = std::env::var("EMOTIONAL_FOCUS_SOFTWARE_CAPTURE") {
                let buffer = ui.window().take_snapshot().expect("software capture");
                let pixels = buffer.as_bytes();
                image::save_buffer(
                    path,
                    pixels,
                    buffer.width(),
                    buffer.height(),
                    image::ColorType::Rgba8,
                )
                .expect("write capture");
            }
            ui.hide().expect("close preview");
        }
        slint::quit_event_loop().expect("exit preview");
    });
    ui.run()?;
    Ok(())
}
