//! Four global destinations, reusing existing library/settings state and actions.
use crate::{state::State, Home};
use emotional_core::controls::Action;

impl State {
    pub(super) fn navigate(&mut self, ui: &Home, destination: usize) {
        if destination > 3 || self.game_options.is_some() || self.audio_panel || self.system_open {
            return;
        }
        if self
            .launch
            .as_ref()
            .is_some_and(|c| c.pending.is_some() || c.committed.is_some())
        {
            return;
        }
        if !self.library && !self.menu && self.dock_focus.is_none() {
            if self.resume {
                self.resume_focus = self.selected;
            } else {
                self.emotional_focus = self.selected;
            }
        }
        self.system_focus = false;
        match destination {
            0 | 1 => {
                self.library = false;
                self.menu = false;
                self.history_panel = false;
                self.resume = destination == 0;
                self.selected = emotional_core::continuity::clamp(
                    if self.resume {
                        self.resume_focus
                    } else {
                        self.emotional_focus
                    },
                    self.resume,
                    self.order.len(),
                    self.recent.is_some(),
                    self.continuity_order.len(),
                );
                // Return from the dock into the destination's content, not the
                // secondary card remembered by the section we just left.
                self.dock_return = self.selected;
            }
            2 => {
                self.library = true;
                self.menu = false;
                self.history_panel = false;
                ui.set_library_offset(-1);
            }
            _ => {
                self.menu_from_library = false;
                self.library = false;
                self.menu = true;
            }
        }
        self.dock_focus = Some(destination);
        self.paint(ui);
    }

    pub(super) fn close_status(&mut self) {
        self.system_open = false;
        self.dock_focus = self.system_return_dock.take();
        self.system_focus = self.dock_focus.is_none();
    }

    pub(super) fn status_action(&mut self, ui: &Home, action: Action) -> bool {
        if self.system_open {
            match action {
                Action::Back => self.close_status(),
                Action::Confirm => {
                    self.system_open = false;
                    self.navigate(ui, 3);
                }
                _ => {}
            }
            self.paint(ui);
            return true;
        }
        if self.system_focus {
            match action {
                Action::Confirm => self.system_open = true,
                Action::Up => {}
                _ => {
                    self.system_focus = false;
                    self.dock_focus = self.system_return_dock.take();
                }
            }
            self.paint(ui);
            return true;
        }
        if self.dock_focus.is_none()
            && action == Action::Up
            && (self.menu && self.menu_index == 0
                || self.library && self.library_index == 0
                || !self.menu && !self.library && self.selected == 0)
        {
            self.system_return_dock = None;
            self.system_focus = true;
            self.paint(ui);
            return true;
        }
        false
    }

    /// Return true only when global navigation consumed the action. Modal
    /// controls are handled before this function, so inputs cannot pass through.
    pub(super) fn dock_action(&mut self, ui: &Home, action: Action) -> bool {
        if let Some(cursor) = self.dock_focus {
            match action {
                Action::Left => self.dock_focus = Some(cursor.saturating_sub(1)),
                Action::Right => self.dock_focus = Some((cursor + 1).min(3)),
                Action::Confirm => self.navigate(ui, cursor),
                Action::Up | Action::Back => {
                    let empty = self.library && self.games.is_empty()
                        || !self.library
                            && !self.menu
                            && (self.resume && self.recent.is_none()
                                || !self.resume && self.order.is_empty());
                    if action == Action::Up && empty {
                        self.system_return_dock = self.dock_focus.take();
                        self.system_focus = true;
                        self.paint(ui);
                        return true;
                    }
                    if !self.library
                        && !self.menu
                        && (self.resume && self.recent.is_none()
                            || !self.resume && self.order.is_empty())
                    {
                        self.paint(ui);
                        return true;
                    }
                    self.dock_focus = None;
                    if !self.library && !self.menu {
                        self.selected = self.dock_return;
                        if self.selected == 2 || self.selected == 3 {
                            self.selected = if self.resume && self.recent.is_none()
                                || !self.resume && self.order.is_empty()
                            {
                                2
                            } else {
                                0
                            };
                        }
                    }
                }
                Action::Down => {}
                _ => {
                    self.dock_focus = None;
                    if !self.library && !self.menu {
                        self.selected = self.dock_return;
                    }
                    return false;
                }
            }
            self.paint(ui);
            return true;
        }
        let enter = if !self.menu && !self.library && action == Action::Right && self.selected == 0
        {
            true
        } else if self.menu {
            action == Action::Right || action == Action::Down && self.menu_index == 9
        } else if self.library {
            action == Action::Right
                || action == Action::Down
                    && (self.games.is_empty() || self.library_index + 1 >= self.games.len())
        } else if self.resume {
            action == Action::Down
                && (matches!(self.selected, 4 | 5) || self.continuity_order.len() < 2)
        } else {
            action == Action::Down && (self.selected == 1 || self.order.len() < 2)
        };
        if enter {
            self.dock_return = self.selected;
            if !self.library && !self.menu {
                if self.resume {
                    self.resume_focus = self.selected;
                } else {
                    self.emotional_focus = self.selected;
                }
            }
            self.dock_focus = Some(if self.menu { 3 } else { 2 });
            self.selected = 2;
            self.paint(ui);
            return true;
        }
        // Discover has no global Library tile: down reaches its one alternative.
        if !self.resume
            && !self.menu
            && !self.library
            && self.order.len() > 1
            && self.selected == 0
            && action == Action::Down
        {
            self.selected = 1;
            self.paint(ui);
            return true;
        }
        false
    }
}
