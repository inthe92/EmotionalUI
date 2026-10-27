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
        let request = supervisor::Request {
            game_id: game.id.clone(),
            recommended: !self.resume && !self.library && self.order.iter().take(3).any(|i|self.games[*i].id==game.id),
            resume: self.resume && !self.library,
            selected: self.selected,
            library: self.library,
            library_index: self.library_index,
            system_option: None,
        };
        self.queue_launch(ui, request);
    }
    pub(super) fn queue_launch(&mut self, ui: &Home, request: supervisor::Request) {
        if self.trial_mode {
            self.notify(ui, "Esta acción requiere la versión funcional.");
            return;
        }
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
        client.pending = Some(Pending {
            request,
            ticket: self.audio.suspend_for_game(),
            start: Instant::now(),
        });
        self.gate.release();
        self.keyboard.reset();
        self.notify(ui, "Preparando…");
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

/// Rank/import can reorder a collection while a game is running. Restore the
/// selected identity when it remains visible, then let the normal clamp handle
/// a removed game. No filesystem or emulator work belongs in this operation.
pub(super) fn restore_position(
    saved: &supervisor::Request,
    games: &[emotional_core::library::Game],
    continuity: &[usize],
) -> supervisor::Request {
    let mut position = saved.clone();
    if let Some(index) = games.iter().position(|g| g.id == saved.game_id && g.available) {
        position.library_index = index;
        if saved.resume {
            if let Some(slot) = continuity.iter().take(3).position(|&i| i == index) {
                position.selected = if slot == 0 { 0 } else { slot + 3 };
            }
        }
    }
    position
}
#[cfg(test)]
mod tests {
    #[test]
    fn returning_secondary_game_follows_identity_after_reordering() {
        let games=emotional_core::library::demo_catalog();
        let saved=super::supervisor::Request {recommended:false,game_id:games[1].id.clone(),resume:true,selected:4,library:false,library_index:0,system_option:None};
        let restored=super::restore_position(&saved,&games,&[1,0]);
        assert_eq!(restored.selected,0);
        assert_eq!(restored.library_index,1);
        let moved=vec![games[1].clone(),games[0].clone()];
        let restored=super::restore_position(&saved,&moved,&[1,0]);
        assert_eq!(restored.selected,4);
        assert_eq!(restored.library_index,0);
    }
    #[test]
    fn disappeared_game_keeps_existing_clamp_fallback_without_inventing_selection() {
        let saved=super::supervisor::Request {recommended:false,game_id:"missing".into(),resume:true,selected:4,library:false,library_index:7,system_option:None};
        let restored=super::restore_position(&saved,&[],&[]);
        assert_eq!(restored.selected,4);
        assert_eq!(restored.library_index,7);
    }
}
