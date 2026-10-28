//! Scrollable app settings and the actual installed dArkOS Options catalogue.
use crate::{state::State, Home};
use slint::{ModelRc, SharedString, VecModel};

impl State {
    pub(super) fn settings_count(&self) -> usize {
        if self.layout != 4 {
            3
        } else if self.options_open {
            self.options_browser.rows(&self.system_options).len()
        } else {
            13
        }
    }

    pub(super) fn paint_settings(&self, ui: &Home) {
        let on = |value| if value { "Activado" } else { "Desactivado" };
        let rows: Vec<(String, String)> = if self.options_open {
            self.options_browser.rows(&self.system_options)
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
                    "Ajustes de dArkOS".into(),
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
        ui.set_settings_title(self.options_browser.title().into());
        ui.set_settings_hint(
            self.options_browser
                .hint(self.menu_index, &self.system_options)
                .into(),
        );
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
        let outcome = self
            .options_browser
            .activate(self.menu_index, &self.system_options);
        self.apply_settings_outcome(ui, outcome);
    }

    pub(super) fn apply_settings_outcome(
        &mut self,
        ui: &Home,
        outcome: crate::settings_browser::Outcome,
    ) {
        use crate::settings_browser::Outcome;
        match outcome {
            Outcome::Focus(index) => {
                self.menu_index = index;
                ui.set_message("".into());
            }
            Outcome::Close => {
                self.options_open = false;
                self.menu_index = 11;
            }
            Outcome::Controls => self.begin_modal(crate::navigation::Modal::System),
            Outcome::Launch(relative) => self.queue_launch(
                ui,
                crate::supervisor::Request {
                    system_option: Some(relative),
                    selected: self.menu_index,
                    ..Default::default()
                },
            ),
        }
    }

    pub(super) fn open_connectivity_settings(&mut self, ui: &Home, bluetooth: bool) {
        let relative = if bluetooth { "Bluetooth.sh" } else { "Wifi.sh" };
        if !self.system_options.iter().any(|e| e.relative == relative) {
            self.system_note =
                Some("La herramienta original no está disponible en esta instalación.".into());
            return;
        }
        self.close_modal();
        self.navigate(ui, 3);
        self.dock_focus = None;
        self.system_focus = false;
        self.options_open = true;
        ui.set_message("".into());
        self.menu_index = self
            .options_browser
            .show_tool(relative, &self.system_options, true);
    }
}
