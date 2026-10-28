//! User intent, edited locally; no filesystem work in a button callback.
use crate::{state::State, Home};
use emotional_core::controls::Action;
impl State {
    pub(super) fn rebuild_continuity(&mut self) {
        let focused = if self.resume {
            match self.selected {
                0 => self.recent,
                4 | 5 => self.continuity_order.get(self.selected - 3).copied(),
                _ => None,
            }
        } else {
            None
        };
        self.continuity_order = emotional_core::continuity::rank(
            &self.games,
            &self.continuity_activity,
            &self.continuity_preferences,
            self.clock.known_now().map(|d| d.and_utc().timestamp()),
            &self.config.continuity,
        )
        .into_iter()
        .map(|c| c.index)
        .collect();
        self.recent = self.continuity_order.first().copied();
        if let Some(i) = focused {
            self.selected = self
                .continuity_order
                .iter()
                .position(|n| *n == i)
                .map(|p| if p == 0 { 0 } else { p + 3 })
                .unwrap_or(2);
        }

        self.selected = emotional_core::continuity::clamp(
            self.selected,
            self.resume,
            self.order.len(),
            self.recent.is_some(),
            self.continuity_order.len(),
        );
    }
    pub(super) fn open_game_options(&mut self, ui: &Home) {
        let index = if self.library || self.menu && self.menu_from_library {
            self.library_game()
        } else if self.resume {
            match self.selected {
                0 => self.recent,
                4 | 5 => self.continuity_order.get(self.selected - 3).copied(),
                _ => None,
            }
        } else {
            self.order
                .get(if self.selected == 1 { 1 } else { 0 })
                .copied()
        };
        if let Some(i) = index.filter(|i| self.games.get(*i).is_some_and(|g| g.available)) {
            self.begin_modal(crate::navigation::Modal::GameOptions);
            self.game_options = Some(i);
            self.game_option_index = 0;
            self.game_explained = false;
        } else {
            self.notify(ui, "Elige una partida o un juego en tu biblioteca.");
        }
    }
    pub(super) fn game_options_action(
        &mut self,
        ui: &Home,
        action: Action,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let Some(i) = self.game_options else {
            return Ok(());
        };
        match action {
            Action::Up => self.game_option_index = self.game_option_index.saturating_sub(1),
            Action::Down => self.game_option_index = (self.game_option_index + 1).min(5),
            Action::Back | Action::Menu => self.close_modal(),
            Action::Confirm if self.game_option_index == 4 => {
                self.game_explained = !self.game_explained
            }
            Action::Confirm if self.game_option_index == 5 => self.close_modal(),
            Action::Confirm if self.game_option_index < 4 => {
                let id = self.games[i].id.clone();
                let mut p = self
                    .continuity_preferences
                    .get(&id)
                    .cloned()
                    .unwrap_or_default();
                match self.game_option_index {
                    0 => {
                        p.pinned = !p.pinned;
                        if p.pinned {
                            p.removed = false;
                            p.completed = false;
                        }
                    }
                    1 => p.removed = !p.removed,
                    2 => p.completed = !p.completed,
                    _ => {
                        p.affinity = match p.affinity {
                            0 => 1,
                            1 => -1,
                            _ => 0,
                        }
                    }
                }
                self.writer.continuity(&id, &p)?;
                self.continuity_preferences.insert(id.clone(), p.clone());
                self.history.completed.retain(|v| v != &id);
                if p.completed {
                    self.history.completed.push(id);
                }
                self.statistics =
                    Self::statistics_line(&self.history, &self.games, self.demo_history);
                self.rebuild_continuity();
                if let Some(cache) = &mut self.discovery_reserve {
                    cache.affinity = self
                        .continuity_preferences
                        .iter()
                        .filter(|(_, p)| p.affinity != 0)
                        .map(|(id, p)| (id.clone(), p.affinity))
                        .collect();
                    if p.affinity > 0 && !cache.pool.contains(&self.games[i].id) {
                        if cache.pool.len() >= 96 {
                            cache.pool.pop();
                        }
                        cache.pool.insert(0, self.games[i].id.clone());
                    }
                    cache.selections.clear();
                }
                self.rerank()?;
            }
            _ => {}
        }
        self.paint(ui);
        Ok(())
    }
}
