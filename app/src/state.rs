use super::{Config, Design, Home};
use crate::store;
use chrono::{Datelike, Timelike};
use emotional_core::{
    controls::Action,
    input::{home_focus, Direction, RepeatGate},
    library::Game,
    mood::Mood,
    recommendation::{renew, Context},
};
use emotional_services::{clock::Clock, weather::Weather};
use slint::ComponentHandle;
use std::{error::Error, time::Instant};
pub(super) struct State {
    pub(super) system_focus: bool,
    pub(super) system_return_dock: Option<usize>,
    pub(super) system_open: bool,
    pub(super) dock_focus: Option<usize>,
    pub(super) dock_return: usize,
    pub(super) trial_mode: bool,
    pub(super) local_art: std::collections::BTreeMap<String, Vec<crate::artwork::Resource>>,
    pub(super) atlas_explanations: std::collections::BTreeMap<String, String>,
    pub(super) launch: Option<crate::launch_ui::Client>,
    pub(super) layout: i32,
    pub(super) discovery_reserve: Option<emotional_core::reserve::Reserve>,
    pub(super) resume: bool,
    pub(super) recent: Option<usize>,
    pub(super) continuity_order: Vec<usize>,
    pub(super) continuity_activity: Vec<emotional_core::continuity::Evidence>,
    pub(super) continuity_preferences:
        std::collections::BTreeMap<String, emotional_core::continuity::Preference>,
    pub(super) game_options: Option<usize>,
    pub(super) game_option_index: usize,
    pub(super) game_explained: bool,
    pub(super) history: emotional_core::product::History,
    pub(super) privacy: emotional_core::product::Privacy,
    pub(super) demo_history: bool,
    pub(super) statistics: String,
    pub(super) history_panel: bool,
    pub(super) clear_armed: bool,
    pub(super) resume_focus: usize,
    pub(super) emotional_focus: usize,
    pub(super) imported: bool,
    pub(super) calendar: emotional_core::editorial::Calendar,
    #[cfg(any(debug_assertions, feature = "simulator"))]
    pub(super) moment_index: Option<usize>,
    pub(super) games: Vec<Game>,
    pub(super) order: Vec<usize>,
    pub(super) selected: usize,
    pub(super) library_index: usize,
    pub(super) library: bool,
    pub(super) menu: bool,
    pub(super) menu_index: usize,
    pub(super) menu_from_library: bool,
    pub(super) native_held: bool,
    pub(super) audio: emotional_services::ambient_audio::AmbientAudioEngine,
    pub(super) audio_preferences: emotional_core::audio::Preferences,
    pub(super) audio_panel: bool,
    pub(super) audio_index: usize,
    pub(super) clock: Clock,
    pub(super) weather: Weather,
    pub(super) config: Config,
    pub(super) refresh: u64,
    pub(super) writer: store::Persistence,
    pub(super) slot: String,
    pub(super) gate: RepeatGate,
    pub(super) keyboard: crate::controls::Keyboard,
    pub(super) start: Instant,
}
impl State {
    pub(super) fn statistics_line(
        history: &emotional_core::product::History,
        games: &[Game],
        demo: bool,
    ) -> String {
        let mut line = history.summary(games, true);
        if let Some(i) = history.latest(games) {
            if let Some(last) = history
                .sessions
                .iter()
                .filter(|s| s.game_id == games[i].id)
                .max_by_key(|s| s.started_utc)
                .and_then(|s| chrono::DateTime::from_timestamp(s.started_utc, 0))
                .filter(|t| (2020..=2100).contains(&t.year()))
            {
                if !line.is_empty() {
                    line.push_str(" · ");
                }
                line.push_str(&format!(
                    "Última sesión {}{}",
                    last.with_timezone(&chrono::Local).format("%d/%m"),
                    if demo { " (muestra)" } else { "" }
                ));
            }
        }
        line
    }
    pub(super) fn sync_context(&mut self, ui: &Home) -> Result<(), Box<dyn Error>> {
        let slot = self
            .clock
            .known_now()
            .map(|d| format!("{}-{}", d.date(), Mood::at(d).index()))
            .unwrap_or_else(|| "unknown".into());
        if slot != self.slot {
            self.slot = slot;
            self.rerank()?;
            self.reveal(ui);
        }
        self.paint(ui);
        Ok(())
    }
    pub(super) fn context(&self) -> Context {
        let d = self.clock.known_now();
        Context {
            mood: d.map(Mood::at).unwrap_or(Mood::Shelter),
            minutes: self.config.session_minutes,
            raining: d.is_some() && self.weather == Weather::Rain,
            slot: d
                .map(|d| (d.and_utc().timestamp().max(0) as u64) / 21600)
                .unwrap_or(0),
            refresh: self.refresh,
        }
    }
    pub(super) fn rerank(&mut self) -> Result<(), Box<dyn Error>> {
        let date = self.clock.known_now().map(|d| d.date());
        let signals = emotional_core::editorial::Signals {
            date,
            events: self.calendar.active(date, &self.config.region),
            ..Default::default()
        };
        let context = self.context();
        self.order = if let Some(cache) = &mut self.discovery_reserve {
            let context_key = format!(
                "{}:{:?}:{:?}:{:?}:{}",
                date.map(emotional_core::editorial::season_id)
                    .unwrap_or("unknown"),
                context.mood,
                self.weather,
                signals.events,
                context.minutes
            );
            let selected = cache.select(
                &self.games,
                context,
                &signals,
                self.privacy.personalized,
                &context_key,
                self.refresh > 0,
            );
            self.writer.reserve(cache)?;
            selected
        } else {
            emotional_core::recommendation::rank_affinity(
                &self.games,
                context,
                &signals,
                self.privacy.personalized,
                &self
                    .continuity_preferences
                    .iter()
                    .map(|(id, p)| (id.clone(), p.affinity))
                    .collect(),
            )
        };
        let ids = self
            .order
            .iter()
            .map(|i| self.games[*i].id.clone())
            .collect::<Vec<_>>();
        self.writer.save(&self.games, &self.slot, ids)?;
        self.selected = if self.layout == 4 {
            emotional_core::continuity::clamp(
                self.selected,
                self.resume,
                self.order.len(),
                self.recent.is_some(),
                self.continuity_order.len(),
            )
        } else if self.layout > 0 && self.order.is_empty() {
            2
        } else {
            0
        };
        Ok(())
    }
    pub(super) fn paint(&self, ui: &Home) {
        ui.set_system_focused(self.system_focus);
        ui.set_system_open(self.system_open);
        ui.set_dock_cursor(self.dock_focus.map(|v| v as i32).unwrap_or(-1));
        ui.set_resume_two_artwork(self.continuity_order.get(1).copied().unwrap_or(0) as i32);
        ui.set_resume_three_artwork(self.continuity_order.get(2).copied().unwrap_or(0) as i32);
        ui.set_resume_count(self.continuity_order.len() as i32);
        ui.set_resume_two(
            self.continuity_order
                .get(1)
                .map(|i| self.games[*i].title.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_resume_three(
            self.continuity_order
                .get(2)
                .map(|i| self.games[*i].title.as_str())
                .unwrap_or("")
                .into(),
        );
        ui.set_game_options_open(self.game_options.is_some());
        ui.set_game_option_index(self.game_option_index as i32);
        if let Some(i) = self.game_options {
            let p = self
                .continuity_preferences
                .get(&self.games[i].id)
                .cloned()
                .unwrap_or_default();
            ui.set_game_option_title(self.games[i].title.clone().into());
            let explanation = if !self.game_explained {
                "Elige cómo te gustaría volver a este juego."
            } else if p.affinity > 0 && self.privacy.personalized {
                "Has indicado que te interesa este juego. Puedes cambiar esa preferencia."
            } else if p.pinned {
                "Fijado por ti. Puedes cambiarlo cuando quieras."
            } else if self
                .continuity_activity
                .iter()
                .any(|e| e.game_id == self.games[i].id && e.confirmed)
            {
                "Lo has abierto antes. Puedes continuar o elegir otra experiencia."
            } else if let Some(text) = self.atlas_explanations.get(&self.games[i].id) {
                text.as_str()
            } else if self.games[i].editorial.exploration.is_some_and(|v| v >= 70) {
                "Su ficha editorial propone una experiencia de exploración."
            } else {
                "Puedes elegirlo libremente desde tu biblioteca."
            };
            ui.set_game_option_reason(explanation.into());
            let labels = vec![
                if p.pinned {
                    "Desfijar de Continuar"
                } else {
                    "Fijar en Continuar"
                },
                if p.removed {
                    "Recuperar en Continuar"
                } else {
                    "Quitar de Continuar"
                },
                if p.completed {
                    "Marcar como en curso"
                } else {
                    "Marcar completado"
                },
                match p.affinity {
                    1 => "Preferencia: me interesa",
                    -1 => "Preferencia: menos por ahora",
                    _ => "Preferencia: sin indicar",
                },
                if self.game_explained {
                    "Ocultar explicación"
                } else {
                    "¿Por qué este juego?"
                },
                "Volver",
            ];
            ui.set_game_option_labels(
                std::rc::Rc::new(slint::VecModel::from(
                    labels
                        .into_iter()
                        .map(slint::SharedString::from)
                        .collect::<Vec<_>>(),
                ))
                .into(),
            );
        }
        ui.set_resume_view(self.resume);
        ui.set_has_recent(self.recent.is_some());
        ui.set_statistics_on(self.privacy.statistics);
        ui.set_personalized(self.privacy.personalized);
        ui.set_neutral(self.privacy.neutral);
        ui.set_ambient_motion(self.privacy.ambient_motion);
        ui.set_history_panel(self.history_panel);
        ui.set_clear_armed(self.clear_armed);
        ui.set_statistics(
            if self.privacy.statistics {
                self.statistics.as_str()
            } else {
                ""
            }
            .into(),
        );
        ui.set_demo_history(self.demo_history);
        if let Some(i) = self.recent {
            ui.set_recent_title(self.games[i].title.clone().into());
            ui.set_resume_detail(self.games[i].subtitle.clone().into());
            ui.set_recent_artwork(i as i32);
        } else {
            ui.set_recent_title("Tu lugar para volver".into());
        }
        ui.set_selected(self.selected as i32);
        let conditions = match self.weather {
            Weather::Clear => emotional_core::scene::Conditions::Clear,
            Weather::Rain => emotional_core::scene::Conditions::Rain,
            Weather::Cloud => emotional_core::scene::Conditions::Cloud,
            Weather::Fog => emotional_core::scene::Conditions::Fog,
            Weather::Snow => emotional_core::scene::Conditions::Snow,
            _ => emotional_core::scene::Conditions::Unknown,
        };
        let date = self.clock.known_now().map(|d| d.date());
        let events = self.calendar.active(date, &self.config.region);
        let scene = emotional_core::scene::Scene::select(
            self.clock.known_now(),
            conditions,
            &events,
            self.privacy.neutral,
        );
        ui.set_scene_season(
            scene
                .season
                .map(|s| match s {
                    emotional_core::calendar::Season::Winter => 0,
                    emotional_core::calendar::Season::Spring => 1,
                    emotional_core::calendar::Season::Summer => 2,
                    emotional_core::calendar::Season::Autumn => 3,
                })
                .unwrap_or(4),
        );
        ui.set_scene_phase(scene.phase.index());
        ui.set_scene_weather(match scene.weather {
            emotional_core::scene::Conditions::Rain => 2,
            emotional_core::scene::Conditions::Cloud => 1,
            emotional_core::scene::Conditions::Fog => 3,
            emotional_core::scene::Conditions::Snow => 4,
            _ => 0,
        });
        ui.set_scene_decoration(scene.decoration);
        self.audio.context(emotional_core::audio::Context {
            phase: scene.phase,
            weather: scene.weather,
        });
        ui.set_audio_open(self.audio_panel);
        ui.set_audio_index(self.audio_index as i32);
        ui.set_audio_master(i32::from(self.audio_preferences.master));
        ui.set_audio_mode(self.audio_preferences.mode.label().into());
        ui.set_audio_status(self.audio_status().into());
        ui.set_audio_values(slint::ModelRc::new(slint::VecModel::from(
            self.audio_values(),
        )));
        ui.set_pan_enabled(
            self.layout == 4 && self.privacy.ambient_motion && !self.privacy.neutral,
        );
        ui.set_imported(self.imported);
        let count = if self.history_panel {
            self.history.sessions.len()
        } else {
            self.games.len()
        };
        let offset = self
            .library_index
            .saturating_sub(3)
            .min(count.saturating_sub(6));
        if ui.get_library_offset() != offset as i32 {
            let titles: Vec<slint::SharedString> = if self.history_panel {
                self.history
                    .sessions
                    .iter()
                    .skip(offset)
                    .take(6)
                    .map(|s| {
                        format!(
                            "{} · {}",
                            self.games
                                .iter()
                                .find(|g| g.id == s.game_id)
                                .map(|g| g.title.as_str())
                                .unwrap_or("Juego no disponible"),
                            chrono::DateTime::from_timestamp(s.started_utc, 0)
                                .map(|t| t.format("%d/%m/%Y %H:%M UTC").to_string())
                                .unwrap_or_else(|| "Fecha desconocida".into())
                        )
                        .into()
                    })
                    .collect()
            } else {
                self.games
                    .iter()
                    .skip(offset)
                    .take(6)
                    .map(|g| g.title.as_str().into())
                    .collect()
            };
            ui.set_library_titles(slint::ModelRc::new(slint::VecModel::from(titles)));
            ui.set_library_offset(offset as i32);
        }
        let known = self.clock.known_now();
        let mood = known.map(Mood::at).unwrap_or(Mood::Shelter);
        ui.set_rain(known.is_some() && self.weather == Weather::Rain);
        ui.set_season(known.map(|d| ((d.month() % 12) / 3) as i32).unwrap_or(0));
        ui.set_editorial_caption(
            if known.is_none() {
                "Sin reloj. A tu ritmo."
            } else if self.weather == Weather::Rain {
                "Afuera llueve. Aquí hay un camino."
            } else {
                match mood {
                    Mood::Dawn => "La primera luz. Un mundo por abrir.",
                    Mood::Horizon => "Todavía queda tarde por descubrir.",
                    Mood::Shelter => "Un pequeño refugio para esta noche.",
                    Mood::Silence => "Lo demás puede esperar.",
                }
            }
            .into(),
        );
        let old = ui.get_mood();
        ui.set_reduced(self.config.reduced_motion);
        let section = if self.resume { 0.0 } else { 1.0 };
        let pan = emotional_core::product::panorama(
            self.resume,
            self.config.reduced_motion,
            self.layout == 4 && self.privacy.ambient_motion && !self.privacy.neutral,
        );
        // Disabling ambient motion must stop the room immediately; functional
        // section feedback can still run independently.
        if pan == 0.0 {
            ui.set_pan_position(0.0);
        }
        if ui.get_section_target() != section
            || ui.get_pan_target() != pan
            || self.config.reduced_motion
        {
            crate::motion::transition(
                ui,
                section,
                pan,
                self.config.reduced_motion || !ui.get_motion_ready(),
            );
        }
        ui.set_animate_palette((old >= 2) == (mood.index() >= 2));
        ui.set_mood(mood.index());
        ui.set_greeting(
            if known.is_some() {
                mood.greeting()
            } else {
                "¿En qué momento estamos?"
            }
            .into(),
        );
        if let Some(now) = known {
            let holiday = emotional_core::calendar::holiday(
                now.date(),
                &self.config.region,
                &self.config.holidays,
            );
            let weather = match self.weather {
                Weather::Rain => " · Lluvia",
                Weather::Cloud => " · Nubes",
                Weather::Wind => " · Brisa",
                _ => "",
            };
            ui.set_atmosphere(
                format!(
                    "{} · {}{}{}",
                    mood.label(),
                    emotional_core::calendar::season(now.date()).label(),
                    holiday.map(|s| format!(" · {s}")).unwrap_or_default(),
                    weather
                )
                .into(),
            );
            ui.set_clock(now.format("%H:%M").to_string().into());
            let months = [
                "enero",
                "febrero",
                "marzo",
                "abril",
                "mayo",
                "junio",
                "julio",
                "agosto",
                "septiembre",
                "octubre",
                "noviembre",
                "diciembre",
            ];
            let days = [
                "lunes",
                "martes",
                "miércoles",
                "jueves",
                "viernes",
                "sábado",
                "domingo",
            ];
            ui.set_date(
                format!(
                    "{} · {} de {}",
                    days[now.weekday().num_days_from_monday() as usize],
                    now.day(),
                    months[now.month0() as usize]
                )
                .into(),
            );
        } else {
            ui.set_atmosphere("Tu colección sigue aquí.".into());
            ui.set_clock("—:—".into());
            ui.set_date("".into());
        }
        ui.set_recommendation_count(self.order.len().min(3) as i32);
        ui.set_library_open(self.library);
        ui.set_menu_open(self.menu);
        ui.set_menu_index(self.menu_index as i32);
        ui.set_sound_on(self.audio_preferences.interface_enabled);
        ui.set_sound_available(self.audio.status().error.is_none());
        ui.set_library_index(self.library_index as i32);
        if self.order.is_empty() {
            ui.set_hero_title("Tu próxima aventura".into());
            ui.set_hero_detail("Cuando añadas juegos disponibles, aparecerán aquí.".into());
            ui.set_hero_tag("A TU RITMO".into());
            ui.set_play_label("Sin juegos".into());
            ui.set_alt_one("".into());
            ui.set_alt_two("".into());
            return;
        }
        let hero = &self.games[self.order[0]];
        ui.set_hero_category(hero.genre.clone().into());
        let explicit = self.continuity_preferences.get(&hero.id);
        let reason = if self.privacy.personalized && explicit.is_some_and(|p| p.affinity > 0) {
            "Has indicado que este juego te interesa. Puedes darle otro momento."
        } else if let Some(text) = self.atlas_explanations.get(&hero.id) {
            text.as_str()
        } else if hero.favorite {
            "Está entre tus favoritos. Puedes elegirlo también para este momento."
        } else if hero.editorial.exploration.is_some_and(|v| v >= 70) {
            "Su ficha editorial propone explorar sin perder la curiosidad."
        } else if hero.started {
            "Un camino que ya has empezado. Puedes retomarlo a tu ritmo."
        } else {
            "Una propuesta de tu colección. Tú eliges si este es su momento."
        };
        ui.set_editorial_reason(reason.into());
        ui.set_hero_title(hero.title.clone().into());
        ui.set_hero_detail(hero.subtitle.clone().into());
        ui.set_hero_tag(
            format!(
                "{}   ·   {} MIN",
                if hero.started {
                    "TU AVENTURA SIGUE"
                } else {
                    "PARA ESTE MOMENTO"
                },
                hero.session_minutes
            )
            .into(),
        );
        ui.set_play_label(if hero.started { "Continuar" } else { "Jugar" }.into());
        ui.set_artwork_index(self.order[0] as i32);
        for (slot, index) in [1, 2].into_iter().enumerate() {
            let game = self.order.get(index).map(|i| &self.games[*i]);
            let title = game.map(|g| g.title.clone()).unwrap_or_default();
            let detail = game
                .map(|g| format!("{} · {} min", g.genre, g.session_minutes))
                .unwrap_or_default();
            if slot == 0 {
                ui.set_alt_one(title.into());
                ui.set_alt_one_detail(detail.into());
            } else {
                ui.set_alt_two(title.into());
                ui.set_alt_two_detail(detail.into());
            }
        }
    }
    pub(super) fn activate(&mut self, ui: &Home, index: usize) -> Result<(), Box<dyn Error>> {
        if self.game_options.is_some() {
            return Ok(());
        }
        if self
            .launch
            .as_ref()
            .is_some_and(|c| c.pending.is_some() || c.committed.is_some())
        {
            return Ok(());
        }
        if (self.launch.is_some() || self.trial_mode)
            && !self.menu
            && !self.history_panel
            && ((self.library && index >= 10) || (!self.library && index == 0 && !self.resume))
        {
            let game = if self.library {
                Some(index - 10)
            } else {
                self.order.first().copied()
            };
            if let Some(i) = game {
                self.launch_game(ui, i);
            }
            return Ok(());
        }
        if self.layout == 4
            && self.resume
            && !self.library
            && !self.menu
            && (index == 4 || index == 5)
        {
            if let Some(i) = self.continuity_order.get(index - 3).copied() {
                self.launch_game(ui, i);
            }
            return Ok(());
        }
        if self.layout == 4 && index == 3 && !self.library && !self.menu {
            return self.action(ui, Action::Next);
        }
        if self.layout == 4 && self.history_panel {
            return Ok(());
        }
        if self.layout == 4 && !self.menu && !self.library && index == 0 && self.resume {
            if let Some(i) = self.recent {
                self.launch_game(ui, i);
            }
            return Ok(());
        }
        if ui.get_concept() > 0 && index == 2 && !self.menu && !self.library {
            self.library = true;
            self.paint(ui);
            return Ok(());
        }
        if self.menu || (self.library && index < 10) {
            return Ok(());
        }
        if index >= 10 {
            let i = index - 10;
            if self.games.get(i).is_some_and(|g| g.available) {
                self.promote(i);
                self.library = false;
                if self.layout == 4 {
                    self.resume = false;
                    self.emotional_focus = 0;
                    self.selected = 0;
                }
            } else {
                return Ok(());
            }
        } else if let Some(&i) = self.order.get(index) {
            if !self.games[i].available {
                return Ok(());
            }
            if index > 0 {
                self.promote(i);
            } else {
                let mood = self.clock.known_now().map(Mood::at);
                if self.privacy.personalized {
                    self.games[i].choices = self.games[i].choices.saturating_add(1);
                }
                if let Some(m) = mood.filter(|_| self.privacy.personalized) {
                    let count = &mut self.games[i].choices_by_mood[m.index() as usize];
                    *count = count.saturating_add(1);
                }
                ui.set_confirmed(true);
                let generation = ui.get_confirmation_generation().wrapping_add(1);
                ui.set_confirmation_generation(generation);
                let weak = ui.as_weak();
                slint::Timer::single_shot(
                    std::time::Duration::from_millis(
                        ui.global::<Design>().get_confirmation_hold().max(0) as u64,
                    ),
                    move || {
                        if let Some(ui) = weak.upgrade() {
                            if ui.get_confirmation_generation() == generation {
                                ui.set_confirmed(false);
                            }
                        }
                    },
                );
                self.notify(
                    ui,
                    if self.imported {
                        "Tu elección está guardada. El lanzamiento llegará pronto."
                    } else {
                        "Este mundo es ficticio. Elige otro camino."
                    },
                );
            }
        } else {
            return Ok(());
        }
        if index > 0 {
            self.reveal(ui);
        }
        let ids = self
            .order
            .iter()
            .map(|i| self.games[*i].id.clone())
            .collect::<Vec<_>>();
        self.writer.save(&self.games, &self.slot, ids)?;
        self.paint(ui);
        Ok(())
    }
    pub(super) fn confirm(&self, ui: &Home) {
        ui.set_confirmed(true);
        let generation = ui.get_confirmation_generation().wrapping_add(1);
        ui.set_confirmation_generation(generation);
        let weak = ui.as_weak();
        slint::Timer::single_shot(std::time::Duration::from_millis(140), move || {
            if let Some(ui) = weak.upgrade() {
                if ui.get_confirmation_generation() == generation {
                    ui.set_confirmed(false);
                }
            }
        });
    }
    pub(super) fn notify(&self, ui: &Home, text: &str) {
        ui.set_message(text.into());
        let weak = ui.as_weak();
        let expected = text.to_owned();
        slint::Timer::single_shot(
            std::time::Duration::from_millis(ui.global::<Design>().get_toast_time().max(0) as u64),
            move || {
                if let Some(ui) = weak.upgrade() {
                    if ui.get_message().as_str() == expected {
                        ui.set_message("".into());
                    }
                }
            },
        );
    }
    pub(super) fn reveal(&self, ui: &Home) {
        ui.set_previous_title(ui.get_hero_title());
        ui.set_previous_detail(ui.get_hero_detail());
        ui.set_previous_tag(ui.get_hero_tag());
        ui.set_previous_artwork(ui.get_artwork_index());
        let generation = ui.get_reveal_generation().wrapping_add(1);
        ui.set_reveal_generation(generation);
        if self.config.reduced_motion {
            ui.set_reveal(1.0);
            return;
        }
        ui.set_reveal(0.0);
        let weak = ui.as_weak();
        slint::Timer::single_shot(
            std::time::Duration::from_millis(
                ui.global::<Design>().get_content_defer().max(0) as u64
            ),
            move || {
                if let Some(ui) = weak.upgrade() {
                    if ui.get_reveal_generation() == generation {
                        ui.set_reveal(1.0);
                    }
                }
            },
        );
    }
    pub(super) fn promote(&mut self, index: usize) {
        self.order.retain(|i| *i != index);
        self.order.insert(0, index);
        self.selected = 0;
    }
    pub(super) fn key(&mut self, ui: &Home, key: &str) -> Result<(), Box<dyn Error>> {
        let Some(action) = self.keyboard.press(key) else {
            return Ok(());
        };
        if !self
            .gate
            .press(action.id(), self.start.elapsed(), action.navigation())
        {
            return Ok(());
        }
        self.action(ui, action)
    }
    pub(super) fn action(&mut self, ui: &Home, action: Action) -> Result<(), Box<dyn Error>> {
        if self
            .launch
            .as_ref()
            .is_some_and(|c| c.pending.is_some() || c.committed.is_some())
        {
            if action == Action::Back {
                self.cancel_launch(ui);
            }
            return Ok(());
        }
        use emotional_services::feedback::Cue;
        if self.game_options.is_some() {
            return self.game_options_action(ui, action);
        }
        if self.audio_panel {
            return self.audio_action(ui, action);
        }
        if self.layout == 4 && self.status_action(ui, action) {
            return Ok(());
        }
        if self.layout == 4 && self.dock_action(ui, action) {
            return Ok(());
        }
        let before = (
            self.selected,
            self.library_index,
            self.menu_index,
            self.library,
            self.menu,
        );
        let mut cue = None;
        match action {
            Action::Details => self.open_game_options(ui),
            Action::MomentPrevious | Action::MomentNext => {
                #[cfg(any(debug_assertions, feature = "simulator"))]
                {
                    let count = crate::moments::NAMES.len();
                    let index = match (self.moment_index, action) {
                        (Some(i), Action::MomentPrevious) => (i + count - 1) % count,
                        (Some(i), _) => (i + 1) % count,
                        (None, _) => 0,
                    };
                    let (clock, weather) = crate::moments::context(crate::moments::NAMES[index])?;
                    self.moment_index = Some(index);
                    self.clock = clock;
                    self.weather = weather;
                    self.sync_context(ui)?;
                }
            }
            Action::Left | Action::Right | Action::Up | Action::Down => {
                let direction = match action {
                    Action::Left => Direction::Left,
                    Action::Right => Direction::Right,
                    Action::Up => Direction::Up,
                    _ => Direction::Down,
                };
                if self.menu {
                    self.menu_index = match direction {
                        Direction::Up => self.menu_index.saturating_sub(1),
                        Direction::Down => {
                            (self.menu_index + 1).min(if self.layout == 4 { 9 } else { 2 })
                        }
                        _ => self.menu_index,
                    };
                } else if self.library {
                    let count = if self.history_panel {
                        self.history.sessions.len()
                    } else {
                        self.games.len()
                    };
                    if count > 0 {
                        self.library_index = match direction {
                            Direction::Up => self.library_index.saturating_sub(1),
                            Direction::Down => (self.library_index + 1).min(count - 1),
                            _ => self.library_index,
                        };
                    }
                } else {
                    self.selected = if self.layout == 4 {
                        emotional_core::continuity::focus(
                            self.selected,
                            direction,
                            self.resume,
                            self.order.len(),
                            self.recent.is_some(),
                            self.continuity_order.len(),
                        )
                    } else if ui.get_concept() > 0 {
                        emotional_core::input::moment_focus(
                            self.selected,
                            direction,
                            self.order.len(),
                        )
                    } else {
                        home_focus(self.selected, direction, self.order.len().min(3))
                    };
                }
            }
            Action::Confirm => {
                if self.menu {
                    if self.layout == 4 {
                        match self.menu_index {
                            0 => self.config.reduced_motion = !self.config.reduced_motion,
                            1 => {
                                self.audio_panel = true;
                                self.audio_index = 0;
                            }
                            2 => self.privacy.statistics = !self.privacy.statistics,
                            3 => {
                                self.privacy.personalized = !self.privacy.personalized;
                                if let Some(cache) = &mut self.discovery_reserve {
                                    cache.selections.clear();
                                }
                                self.rerank()?;
                            }
                            4 => self.privacy.neutral = !self.privacy.neutral,
                            5 => self.privacy.ambient_motion = !self.privacy.ambient_motion,
                            6 => {
                                self.menu = false;
                                self.library = true;
                                self.history_panel = true;
                                self.library_index = 0;
                                ui.set_library_offset(-1);
                            }
                            7 => {
                                if self.clear_armed {
                                    self.writer.clear_history()?;
                                    self.history.sessions.clear();
                                    self.statistics = self.history.summary(&self.games, true);
                                    self.continuity_activity.clear();
                                    if let Some(cache) = &mut self.discovery_reserve {
                                        cache.selections.clear();
                                    }
                                    self.rebuild_continuity();
                                    self.demo_history = false;
                                    for g in &mut self.games {
                                        g.choices = 0;
                                        g.choices_by_mood = [0; 4];
                                        g.recent_recommendations = 0;
                                    }
                                    self.clear_armed = false;
                                    self.rerank()?;
                                    self.notify(ui, "Historial borrado.");
                                } else {
                                    self.clear_armed = true;
                                }
                            }
                            9 => self.open_game_options(ui),
                            _ => {
                                ui.hide()?;
                                slint::quit_event_loop()?;
                            }
                        }
                        self.writer.privacy(&self.privacy)?;
                    } else {
                        match self.menu_index {
                            0 => self.config.reduced_motion = !self.config.reduced_motion,
                            1 => {
                                self.audio_preferences.interface_enabled =
                                    !self.audio_preferences.interface_enabled;
                                self.audio.preferences(self.audio_preferences.clone())?;
                                self.writer.audio(&self.audio_preferences)?;
                            }
                            _ => {
                                ui.hide()?;
                                slint::quit_event_loop()?;
                            }
                        }
                    }
                } else {
                    self.activate(
                        ui,
                        if self.library {
                            10 + self.library_index
                        } else {
                            self.selected
                        },
                    )?;
                }
                cue = Some(Cue::Confirm);
            }
            Action::Back => {
                self.library = self.menu && self.menu_from_library;
                self.menu = false;
                self.clear_armed = false;
                if !self.library {
                    self.history_panel = false;
                    ui.set_library_offset(-1);
                    if matches!(self.selected, 2 | 3) {
                        self.selected = self.dock_return;
                        if self.resume && self.recent.is_none()
                            || !self.resume && self.order.is_empty()
                        {
                            self.selected = 2;
                            self.dock_focus = Some(2);
                        }
                    }
                }
                ui.set_message("".into());
                cue = Some(Cue::Cancel);
            }
            Action::Library => {
                self.dock_focus = None;
                self.history_panel = false;
                ui.set_library_offset(-1);
                self.library = !self.library;
                self.menu = false;
                cue = Some(Cue::Panel);
            }
            Action::Menu => {
                self.dock_focus = None;
                if self.menu {
                    self.menu = false;
                    self.library = self.menu_from_library;
                } else {
                    self.menu_from_library = self.library;
                    self.library = false;
                    self.menu = true;
                }
                cue = Some(Cue::Panel);
            }
            Action::Previous | Action::Next if self.layout == 4 => {
                cue = Some(Cue::Panel);
                if self.resume {
                    self.resume_focus = self.selected;
                } else {
                    self.emotional_focus = self.selected;
                }
                self.resume = !self.resume;
                self.selected = if self.resume {
                    emotional_core::continuity::clamp(
                        self.resume_focus,
                        true,
                        self.order.len(),
                        self.recent.is_some(),
                        self.continuity_order.len(),
                    )
                } else {
                    self.emotional_focus
                };
                self.library = false;
                self.menu = false;
                self.history_panel = false;
                ui.set_library_offset(-1);
            }
            Action::Previous => {
                self.library = false;
                self.menu = false;
            }
            Action::Next => {
                self.library = true;
                self.menu = false;
            }
            Action::Refresh => {
                if self.layout == 4 && self.resume {
                    return Ok(());
                }
                renew(&mut self.games, &self.order);
                self.refresh = self.refresh.wrapping_add(1);
                self.rerank()?;
                self.reveal(ui);
                self.menu = false;
                self.notify(ui, "Otro camino, a tu ritmo.");
                cue = Some(Cue::Confirm);
            }
            Action::ReducedMotion => self.config.reduced_motion = !self.config.reduced_motion,
            Action::Sound => {
                self.audio_preferences.interface_enabled =
                    !self.audio_preferences.interface_enabled;
                self.audio.preferences(self.audio_preferences.clone())?;
                self.writer.audio(&self.audio_preferences)?;
            }
            Action::PreviewPrevious | Action::PreviewNext => {
                if !cfg!(any(debug_assertions, feature = "simulator")) {
                    return Ok(());
                }
                let d = self.clock.now();
                let hours = [8, 15, 21, 2];
                let current = Mood::at(d).index() as usize;
                let next = if action == Action::PreviewNext {
                    (current + 1) % 4
                } else {
                    (current + 3) % 4
                };
                self.clock.set(
                    d.date()
                        .and_hms_opt(hours[next], d.minute(), 0)
                        .ok_or("invalid simulation")?,
                );
                self.sync_context(ui)?;
            }
        }
        let after = (
            self.selected,
            self.library_index,
            self.menu_index,
            self.library,
            self.menu,
        );
        if cue.is_none() && before != after {
            cue = Some(if before.3 != after.3 || before.4 != after.4 {
                Cue::Panel
            } else {
                Cue::Focus
            });
        }
        if self.menu_index != 7 || !self.menu {
            self.clear_armed = false;
        }
        if let Some(cue) = cue {
            self.audio.cue(cue);
        }
        self.paint(ui);
        Ok(())
    }
}
