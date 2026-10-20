//! Button-first settings controller; all playback and persistence are asynchronous.
use super::{state::State, Home};
use emotional_core::{audio::Mode, controls::Action};
impl State {
    pub(super) fn audio_action(
        &mut self,
        ui: &Home,
        action: Action,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            Action::Up => self.audio_index = self.audio_index.saturating_sub(1),
            Action::Down => self.audio_index = (self.audio_index + 1).min(6),
            Action::Back | Action::Menu => self.audio_panel = false,
            Action::Previous | Action::Next | Action::Library => {
                self.audio_panel = false;
                return self.action(ui, action);
            }
            Action::ReducedMotion => self.config.reduced_motion = !self.config.reduced_motion,
            Action::Left | Action::Right | Action::Confirm => {
                if self.audio_index == 6 {
                    self.audio_panel = false;
                } else {
                    let forward = action != Action::Left;
                    match self.audio_index {
                        0 => {
                            self.audio_preferences.mode = self.audio_preferences.mode.step(forward)
                        }
                        4 => {
                            self.audio_preferences.interface_enabled =
                                !self.audio_preferences.interface_enabled
                        }
                        row => {
                            let value = match row {
                                1 => &mut self.audio_preferences.master,
                                2 => &mut self.audio_preferences.music,
                                3 => &mut self.audio_preferences.ambience,
                                _ => &mut self.audio_preferences.interface,
                            };
                            *value = if forward {
                                value.saturating_add(5).min(100)
                            } else {
                                value.saturating_sub(5)
                            };
                        }
                    }
                    self.audio.preferences(self.audio_preferences.clone())?;
                    self.writer.audio(&self.audio_preferences)?;
                }
            }
            _ => {}
        }
        self.paint(ui);
        Ok(())
    }
    pub(super) fn audio_values(&self) -> Vec<slint::SharedString> {
        vec![
            self.audio_preferences.mode.label().into(),
            format!("{} %", self.audio_preferences.master).into(),
            format!("{} %", self.audio_preferences.music).into(),
            format!("{} %", self.audio_preferences.ambience).into(),
            if self.audio_preferences.interface_enabled {
                "Activados"
            } else {
                "Desactivados"
            }
            .into(),
            format!("{} %", self.audio_preferences.interface).into(),
            "B volver".into(),
        ]
    }
    pub(super) fn audio_status(&self) -> &'static str {
        let status = self.audio.status();
        if self.audio_preferences.mode == Mode::Silence && !self.audio_preferences.interface_enabled
        {
            "Espacio para el silencio"
        } else if status.error.is_some() {
            "Sin salida de audio disponible"
        } else if status.missing_assets > 0 {
            "Algunos sonidos no están disponibles"
        } else if self.audio_preferences.mode == Mode::Silence {
            "Espacio para el silencio"
        } else {
            "Sonidos locales, a tu ritmo"
        }
    }
}
