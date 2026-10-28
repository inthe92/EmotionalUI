//! UI half of launch: no process or filesystem access inside an input callback.
use crate::{state::State, supervisor, Home};
use emotional_services::ambient_audio::ReleaseTicket;
use std::{path::PathBuf, time::{Duration, Instant}};
pub struct Client {
    pub directory: PathBuf,
    pub pending: Option<Pending>,
    pub committed: Option<supervisor::Request>,
}
pub struct Pending {
    pub request: supervisor::Request,
    pub ticket: ReleaseTicket,
    pub start: Instant,
    pub frame_started: Option<Instant>,
}
fn entrance_duration(reduced: bool, system_option: bool) -> Duration {
    Duration::from_millis(if system_option { 0 } else if reduced { 100 } else { 440 })
}
fn entrance_progress(elapsed: Duration, duration: Duration) -> f32 {
    if duration.is_zero() { return 1.; }
    let t = (elapsed.as_secs_f32() / duration.as_secs_f32()).clamp(0., 1.);
    t
}
impl State {
    pub(super) fn launch_game(&mut self, ui: &Home, index: usize) {
        if self.launch.as_ref().is_some_and(|c|c.pending.is_some() || c.committed.is_some()) {
            return;
        }
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
            library_view: self.library.then(||self.library_browser.saved()),
            system_option: None,
        };
        let existing = if self.recent == Some(index) {
            (ui.get_resume_cover(), ui.get_resume_art_available())
        } else if self.continuity_order.get(1) == Some(&index) {
            (ui.get_resume_two_cover(), ui.get_resume_two_art_available())
        } else if self.continuity_order.get(2) == Some(&index) {
            (ui.get_resume_three_cover(), ui.get_resume_three_art_available())
        } else if self.order.first() == Some(&index) {
            (ui.get_local_cover(), ui.get_local_art_available())
        } else {
            (slint::Image::default(), false)
        };
        self.queue_launch(ui, request);
        if ui.get_launch_open() && existing.1 {
            ui.set_launch_cover(existing.0);
            ui.set_launch_cover_available(true);
        }
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
        ui.set_launch_open(request.system_option.is_none());
        ui.set_launch_progress(if self.config.reduced_motion { 1. } else { 0. });
        ui.set_launch_title(self.games.iter().find(|g|g.id==request.game_id).map(|g|g.title.clone()).unwrap_or_default().into());
        ui.set_launch_cover(slint::Image::default());
        ui.set_launch_cover_available(false);
        ui.set_launch_frame_requested(false);
        ui.set_launch_frame_ready(false);
        #[cfg(feature = "gpu-material")]
        let _ = crate::gpu_material::take_launch_frame();
        client.pending = Some(Pending {
            request,
            ticket: self.audio.suspend_for_game(),
            start: Instant::now(),
            frame_started: None,
        });
        self.gate.release();
        self.keyboard.reset();
        self.notify(ui, "Preparando…");
    }
    pub(super) fn cancel_launch(&mut self, ui: &Home) {
        if let Some(client) = &mut self.launch {
            client.pending = None;
        }
        ui.set_launch_open(false);
        ui.set_launch_frame_requested(false);
        ui.set_launch_frame_ready(false);
        let _ = self.audio.return_from_game(self.audio_preferences.clone());
        self.notify(ui, "Seguimos aquí.");
    }
    pub(super) fn launch_tick(&mut self, ui: &Home) {
        let Some(p) = self.launch.as_ref().and_then(|c| c.pending.as_ref()) else {
            return;
        };
        let elapsed = p.start.elapsed();
        let duration = entrance_duration(self.config.reduced_motion, p.request.system_option.is_some());
        ui.set_launch_progress(if self.config.reduced_motion { 1. } else { entrance_progress(elapsed, duration) });
        let audio_ready = self.audio.released(p.ticket);
        if !audio_ready && elapsed > Duration::from_secs(2) {
            self.cancel_launch(ui);
            self.notify(ui, "No pudimos preparar el sonido. Inténtalo de nuevo.");
            return;
        }
        if audio_ready && elapsed >= duration {
            // One optional final-frame readback; bounded wait, never a GPU owner
            // surviving into the emulator. This bridge is off until device validation.
            if p.request.system_option.is_none() && crate::supervisor::frame_bridge_enabled() {
                let waited=p.frame_started.map(|start|start.elapsed()).unwrap_or(Duration::ZERO);
                let first=p.frame_started.is_none();
                ui.set_launch_frame_requested(true);
                if first {
                    if let Some(p)=self.launch.as_mut().and_then(|c|c.pending.as_mut()) {
                        p.frame_started=Some(Instant::now());
                    }
                }
                if !ui.get_launch_frame_ready() && waited < Duration::from_millis(200) {
                    ui.window().request_redraw();
                    return;
                }
            }
            println!(
                "LAUNCH_UI audio_ready=true entrance_ready=true handoff_wait_ms={:.3}",
                elapsed.as_secs_f64() * 1000.0
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
    fn entrance_is_bounded_monotonic_and_reduced_motion_does_not_move() {
        use std::time::Duration;
        let normal=super::entrance_duration(false,false);
        let mut previous=0.;
        for ms in 0..=600 {
            let progress=super::entrance_progress(Duration::from_millis(ms),normal);
            assert!(progress>=previous && (0. ..=1.).contains(&progress));
            previous=progress;
        }
        assert_eq!(previous,1.);
        assert_eq!(super::entrance_duration(true,false),Duration::from_millis(100));
        assert_eq!(super::entrance_duration(false,true),Duration::ZERO);
    }
    #[test]
    fn returning_secondary_game_follows_identity_after_reordering() {
        let games=emotional_core::library::demo_catalog();
        let saved=super::supervisor::Request {recommended:false,game_id:games[1].id.clone(),resume:true,selected:4,library:false,library_index:0,library_view:None,system_option:None};
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
        let saved=super::supervisor::Request {recommended:false,game_id:"missing".into(),resume:true,selected:4,library:false,library_index:7,library_view:None,system_option:None};
        let restored=super::restore_position(&saved,&[],&[]);
        assert_eq!(restored.selected,4);
        assert_eq!(restored.library_index,7);
    }
}
