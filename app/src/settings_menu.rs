//! Scrollable app settings and the actual installed dArkOS Options catalogue.
use crate::{state::State, Home};
use slint::{ModelRc, SharedString, VecModel};

impl State {
    pub(super) fn settings_count(&self) -> usize {
        if self.layout != 4 {
            3
        } else if self.options_open {
            self.system_options.len() + 1
        } else {
            13
        }
    }

    pub(super) fn paint_settings(&self, ui: &Home) {
        let on = |value| if value { "Activado" } else { "Desactivado" };
        let rows: Vec<(String, String)> = if self.options_open {
            self.system_options
                .iter()
                .map(|e| (e.title.clone(), "Abrir".into()))
                .chain(std::iter::once(("Volver a Ajustes".into(), String::new())))
                .collect()
        } else if self.layout == 4 {
            vec![
                (
                    "Movimiento reducido".into(),
                    on(self.config.reduced_motion).into(),
                ),
                ("Sonido".into(), self.audio_preferences.mode.label().into()),
                ("Estadísticas".into(), on(self.privacy.statistics).into()),
                (
                    "Personalización".into(),
                    on(self.privacy.personalized).into(),
                ),
                (
                    "Apariencia".into(),
                    if self.privacy.dark_mode {
                        "Oscuro"
                    } else {
                        "Claro"
                    }
                    .into(),
                ),
                (
                    "Movimiento ambiental".into(),
                    on(self.privacy.ambient_motion).into(),
                ),
                ("Consultar historial".into(), String::new()),
                (
                    if self.clear_armed {
                        "Confirmar borrado"
                    } else {
                        "Borrar historial"
                    }
                    .into(),
                    String::new(),
                ),
                ("Volver a la consola".into(), String::new()),
                ("Esta partida".into(), String::new()),
                (
                    "Guardar historial".into(),
                    on(self.privacy.record_history).into(),
                ),
                (
                    "Opciones de dArkOS".into(),
                    format!("{} herramientas", self.system_options.len()),
                ),
                ("Controles del sistema".into(), String::new()),
            ]
        } else {
            vec![
                (
                    "Movimiento reducido".into(),
                    on(self.config.reduced_motion).into(),
                ),
                ("Sonido".into(), String::new()),
                ("Volver a la consola".into(), String::new()),
            ]
        };
        let offset = if self.layout == 4 {
            self.menu_index
                .saturating_sub(3)
                .min(rows.len().saturating_sub(6))
        } else {
            0
        };
        ui.set_menu_offset(offset as i32);
        ui.set_options_open(self.options_open);
        ui.set_menu_titles(ModelRc::new(VecModel::from(
            rows.iter()
                .skip(offset)
                .take(6)
                .map(|(label, _)| SharedString::from(label))
                .collect::<Vec<_>>(),
        )));
        ui.set_menu_values(ModelRc::new(VecModel::from(
            rows.iter()
                .skip(offset)
                .take(6)
                .map(|(_, value)| SharedString::from(value))
                .collect::<Vec<_>>(),
        )));
    }

    pub(super) fn activate_system_option(&mut self, ui: &Home) {
        let Some(entry) = self.system_options.get(self.menu_index) else {
            self.options_open = false;
            self.menu_index = 11;
            self.paint(ui);
            return;
        };
        self.queue_launch(
            ui,
            crate::supervisor::Request {
                system_option: Some(entry.relative.clone()),
                selected: self.menu_index,
                ..Default::default()
            },
        );
    }
}
