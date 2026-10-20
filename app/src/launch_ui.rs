//! UI half of launch: no process or filesystem access inside an input callback.
use crate::{state::State, supervisor, Home};
use emotional_services::ambient_audio::ReleaseTicket;
use std::{path::PathBuf, time::Instant};
pub struct Client {
    pub directory: PathBuf,
    pub pending: Option<Pending>,
    pub committed: Option<supervisor::Request>,
}
pub struct Pending {
    pub request: supervisor::Request,
    pub ticket: ReleaseTicket,
    pub start: Instant,
}
impl State {
    pub(super) fn launch_game(&mut self, ui: &Home, index: usize) {
        if self.trial_mode {
            self.notify(ui,"Modo prueba · Puedes explorar. El lanzamiento físico aún necesita validación supervisada.");
            return;
        }
        let Some(game) = self.games.get(index).filter(|g| g.available) else {
            return;
        };
        if self.launch.is_some() && self.writer.audio(&self.audio_preferences).is_err() {
            self.notify(ui, "No pudimos guardar tus ajustes. Inténtalo de nuevo.");
            return;
        }
        let Some(client) = &mut self.launch else {
            self.notify(
                ui,
                "Configura un lanzador para abrir tus juegos. Tu partida sigue intacta.",
            );
            self.confirm(ui);
            return;
        };
        if client.pending.is_some() || client.committed.is_some() {
            return;
        }
        let request = supervisor::Request {
            game_id: game.id.clone(),
            resume: self.resume,
            selected: self.selected,
            library: self.library,
            library_index: self.library_index,
        };
        client.pending = Some(Pending {
            request,
            ticket: self.audio.suspend_for_game(),
            start: Instant::now(),
        });
        self.gate.release();
        self.keyboard.reset();
        self.notify(ui, "Preparando tu juego…");
    }
    pub(super) fn cancel_launch(&mut self, ui: &Home) {
        if let Some(client) = &mut self.launch {
            client.pending = None;
        }
        let _ = self.audio.return_from_game(self.audio_preferences.clone());
        self.notify(ui, "Seguimos aquí.");
    }
    pub(super) fn launch_tick(&mut self, ui: &Home) {
        let Some(p) = self.launch.as_ref().and_then(|c| c.pending.as_ref()) else {
            return;
        };
        if self.audio.released(p.ticket) {
            println!(
                "LAUNCH_UI audio_released_ms={:.3}",
                p.start.elapsed().as_secs_f64() * 1000.0
            );
            let Some(client) = &mut self.launch else {
                return;
            };
            client.committed = client.pending.take().map(|p| p.request);
            // Full process exit, observed by supervisor, is the actual graphics/input lease.
            if ui.hide().is_ok() {
                let _ = slint::quit_event_loop();
            } else {
                client.committed = None;
                self.cancel_launch(ui);
            }
        } else if p.start.elapsed() > std::time::Duration::from_secs(2) {
            self.cancel_launch(ui);
            self.notify(ui, "No pudimos preparar el sonido. Inténtalo de nuevo.");
        }
    }
}
use slint::ComponentHandle;
