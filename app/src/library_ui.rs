//! Map visible library rows to original game IDs; keep history on its own path.
use crate::{state::State, Home};
use emotional_core::controls::Action;
use slint::{ModelRc, SharedString, VecModel};
impl State {
    pub(super) fn grouped_library(&self) -> bool {
        self.layout == 4 && !self.history_panel
    }
    pub(super) fn library_cursor(&self) -> usize {
        if self.grouped_library() {
            self.library_browser.cursor
        } else {
            self.library_index
        }
    }
    pub(super) fn library_game(&self) -> Option<usize> {
        if self.history_panel {
            None
        } else if self.grouped_library() {
            self.library_browser.selected_game()
        } else {
            Some(self.library_index)
        }
    }
    pub(super) fn sync_library_game(&mut self) {
        if let Some(i) = self.library_game() {
            self.library_index = i;
        }
    }
    pub(super) fn rebuild_library(&mut self) {
        self.library_browser.rebuild(&self.games);
        self.sync_library_game();
    }
    pub(super) fn paint_library(&self, ui: &Home) {
        let grouped = self.grouped_library();
        let cursor = self.library_cursor();
        let count = if self.history_panel {
            self.history.sessions.len()
        } else if grouped {
            self.library_browser.count()
        } else {
            self.games.len()
        };
        let offset = cursor.saturating_sub(3).min(count.saturating_sub(6));
        if ui.get_library_offset() != offset as i32 {
            let titles: Vec<SharedString> = if self.history_panel {
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
            } else if grouped {
                (offset..(offset + 6).min(count))
                    .filter_map(|i| self.library_browser.row(&self.games, i))
                    .map(Into::into)
                    .collect()
            } else {
                self.games
                    .iter()
                    .skip(offset)
                    .take(6)
                    .map(|g| g.title.as_str().into())
                    .collect()
            };
            ui.set_library_titles(ModelRc::new(VecModel::from(titles)));
            ui.set_library_offset(offset as i32);
        }
        ui.set_library_index(cursor as i32);
        ui.set_library_heading(if self.history_panel {
            "Tus momentos de juego.".into()
        } else if grouped {
            self.library_browser.title().into()
        } else {
            "Toda tu colección, a tu ritmo.".into()
        });
        ui.set_library_empty_text(if grouped {
            self.library_browser.empty_text().into()
        } else {
            "Tu biblioteca está vacía.".into()
        });
        ui.set_library_search_open(self.library && grouped && self.library_browser.searching());
        ui.set_library_query(self.library_browser.query.clone().into());
        ui.set_library_key(self.library_browser.key as i32);
    }
    pub(super) fn library_search_key(&mut self, ui: &Home, key: usize) {
        if !self.library
            || !self.grouped_library()
            || self.modal_active()
            || !self.library_browser.searching()
        {
            return;
        }
        self.dock_focus = None;
        self.system_focus = false;
        self.library_browser.choose_key(key, &self.games);
        self.sync_library_game();
        ui.set_library_offset(-1);
        self.paint(ui);
    }
    pub(super) fn library_search_action(&mut self, ui: &Home, action: Action) -> bool {
        if !self.library || !self.grouped_library() {
            return false;
        }
        if self.library_browser.searching() {
            self.library_browser.search_action(action, &self.games);
            self.sync_library_game();
            ui.set_library_offset(-1);
            self.paint(ui);
            return true;
        }
        if action == Action::Refresh {
            self.library_browser.open_search();
            self.dock_focus = None;
            self.system_focus = false;
            ui.set_library_offset(-1);
            self.paint(ui);
            return true;
        }
        false
    }
}
