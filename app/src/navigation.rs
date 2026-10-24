//! Four global destinations, reusing existing library/settings state and actions.
use crate::{state::State, Home};
use emotional_core::controls::Action;

/// Logical section order is independent of transition progress or render timing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Section {
    Continue = 0,
    Moment = 1,
    Library = 2,
    Settings = 3,
}
impl Section {
    fn from_index(index: usize) -> Option<Self> {
        [Self::Continue, Self::Moment, Self::Library, Self::Settings]
            .get(index)
            .copied()
    }
    fn step(self, forward: bool) -> Self {
        Self::from_index((self as usize + if forward { 1 } else { 3 }) % 4).unwrap()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusTarget {
    Content,
    Dock(usize),
    Status,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct FocusSnapshot {
    section: Section,
    index: usize,
    game_id: Option<String>,
    target: FocusTarget,
    history: bool,
}

#[derive(Default)]
pub(super) struct NavigationMemory {
    sections: [Option<FocusSnapshot>; 4],
}
impl NavigationMemory {
    fn remember(&mut self, focus: FocusSnapshot) {
        let index = focus.section as usize;
        self.sections[index] = Some(focus);
    }
    fn recalled(&self, section: Section) -> Option<FocusSnapshot> {
        self.sections[section as usize].clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Modal {
    GameOptions,
    Audio,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ModalInput {
    Close,
    Consume,
    Dispatch,
}
pub(super) fn modal_input(action: Action) -> ModalInput {
    match action {
        Action::Back | Action::Menu => ModalInput::Close,
        Action::Previous | Action::Next | Action::Library => ModalInput::Consume,
        _ => ModalInput::Dispatch,
    }
}

fn retained_index<'a>(
    previous: usize,
    id: Option<&str>,
    ids: impl Iterator<Item = &'a str>,
    count: usize,
) -> usize {
    id.and_then(|wanted| {
        ids.enumerate()
            .find_map(|(i, id)| (wanted == id).then_some(i))
    })
    .unwrap_or(previous.min(count.saturating_sub(1)))
}

// Legacy layouts expose footer focus values 2/3. The four-section UI no
// longer renders these controls, so only genuine cards may regain content focus.
fn content_index(resume: bool, index: usize, recommendations: usize, continuing: usize) -> usize {
    if resume {
        match (index, continuing) {
            (_, 0) => 2, // Empty home focuses its global navigation instead.
            (4, 2..) => 4,
            (5, 3..) => 5,
            _ => 0,
        }
    } else if recommendations == 0 {
        2
    } else if index == 1 && recommendations > 1 {
        1
    } else {
        0
    }
}

fn card_focus(
    current: usize,
    action: Action,
    resume: bool,
    recommendations: usize,
    continuing: usize,
) -> usize {
    let current = content_index(resume, current, recommendations, continuing);
    match (resume, current, action) {
        (true, 0, Action::Down) if continuing > 1 => 4,
        (true, 4, Action::Right) if continuing > 2 => 5,
        (true, 5, Action::Left) => 4,
        (true, 4 | 5, Action::Up) => 0,
        (false, 0, Action::Down) if recommendations > 1 => 1,
        (false, 1, Action::Up) => 0,
        _ => current,
    }
}

impl State {
    fn section(&self) -> Section {
        if self.menu {
            Section::Settings
        } else if self.library {
            Section::Library
        } else if self.resume {
            Section::Continue
        } else {
            Section::Moment
        }
    }

    pub(super) fn focus_snapshot(&self) -> FocusSnapshot {
        let section = self.section();
        let (index, game) = match section {
            Section::Continue => (
                self.selected,
                match self.selected {
                    0 => self.recent,
                    4 | 5 => self.continuity_order.get(self.selected - 3).copied(),
                    _ => None,
                },
            ),
            Section::Moment => (
                self.selected,
                (self.selected <= 1)
                    .then(|| self.order.get(self.selected).copied())
                    .flatten(),
            ),
            Section::Library => (
                self.library_index,
                if self.history_panel {
                    None
                } else {
                    self.browser.selected_game()
                },
            ),
            Section::Settings => (self.menu_index, None),
        };
        FocusSnapshot {
            section,
            index,
            game_id: game.and_then(|i| self.games.get(i)).map(|g| g.id.clone()),
            target: if self.system_focus {
                FocusTarget::Status
            } else if let Some(cursor) = self.dock_focus {
                FocusTarget::Dock(cursor)
            } else {
                FocusTarget::Content
            },
            history: self.history_panel,
        }
    }

    pub(super) fn remember_focus(&mut self) {
        self.navigation.remember(self.focus_snapshot());
    }

    pub(super) fn restore_focus(&mut self, focus: FocusSnapshot) {
        self.library = focus.section == Section::Library;
        self.menu = focus.section == Section::Settings;
        self.history_panel = self.library && focus.history;
        match focus.section {
            Section::Continue | Section::Moment => {
                self.resume = focus.section == Section::Continue;
                let order = if self.resume {
                    &self.continuity_order
                } else {
                    &self.order
                };
                let previous = if self.resume && focus.index >= 4 {
                    focus.index - 3
                } else {
                    focus.index.min(1)
                };
                let visible = order.len().min(if self.resume { 3 } else { 2 });
                let position = retained_index(
                    previous,
                    focus.game_id.as_deref(),
                    order
                        .iter()
                        .take(visible)
                        .filter_map(|i| self.games.get(*i))
                        .map(|g| g.id.as_str()),
                    visible,
                );
                let index = if focus.game_id.is_some() {
                    if self.resume && position > 0 {
                        position + 3
                    } else {
                        position
                    }
                } else {
                    focus.index
                };
                self.selected = content_index(
                    self.resume,
                    index,
                    self.order.len(),
                    self.continuity_order.len(),
                );
                if self.resume {
                    self.resume_focus = self.selected;
                } else {
                    self.emotional_focus = self.selected;
                }
                self.dock_return = self.selected;
            }
            Section::Library => {
                if !self.history_panel {
                    if let Some(index) = focus
                        .game_id
                        .as_ref()
                        .and_then(|id| self.games.iter().position(|g| &g.id == id))
                    {
                        self.browser.select_game(index);
                    }
                }
                self.library_index = if self.history_panel {
                    focus
                        .index
                        .min(self.history.sessions.len().saturating_sub(1))
                } else {
                    retained_index(
                        focus.index,
                        focus.game_id.as_deref(),
                        self.games.iter().map(|g| g.id.as_str()),
                        self.games.len(),
                    )
                };
            }
            Section::Settings => {
                self.menu_index = focus.index.min(if self.layout == 4 { 9 } else { 2 })
            }
        }
        self.system_focus = focus.target == FocusTarget::Status;
        self.dock_focus = match focus.target {
            FocusTarget::Dock(cursor) => Some(cursor.min(3)),
            _ => None,
        };
        if !self.menu
            && !self.library
            && self.selected == 2
            && self.dock_focus.is_none()
            && !self.system_focus
        {
            self.dock_focus = Some(focus.section as usize);
        }
    }

    pub(super) fn restore_remembered_focus(&mut self) {
        if let Some(focus) = self.navigation.recalled(self.section()) {
            self.restore_focus(focus);
        }
    }

    pub(super) fn modal_active(&self) -> bool {
        self.game_options.is_some()
            || self.audio_panel
            || self.system_open
            || self
                .launch
                .as_ref()
                .is_some_and(|c| c.pending.is_some() || c.committed.is_some())
    }

    /// Modal ownership is exclusive. Replacing a panel preserves the original
    /// return location, so B can never fall through into the underlying section.
    pub(super) fn begin_modal(&mut self, modal: Modal) {
        if self.modal_return.is_none() {
            self.modal_return = Some(self.focus_snapshot());
        }
        self.game_options = None;
        self.audio_panel = modal == Modal::Audio;
        self.system_open = modal == Modal::System;
        self.system_focus = modal == Modal::System;
        self.dock_focus = None;
    }

    pub(super) fn close_modal(&mut self) {
        self.game_options = None;
        self.audio_panel = false;
        self.system_open = false;
        if let Some(focus) = self.modal_return.take() {
            self.restore_focus(focus);
        }
    }

    pub(super) fn open_status(&mut self) {
        if self
            .launch
            .as_ref()
            .is_some_and(|client| client.pending.is_some() || client.committed.is_some())
        {
            return;
        }
        self.begin_modal(Modal::System);
    }

    pub(super) fn modal_audio_action(
        &mut self,
        ui: &Home,
        action: Action,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if !self.audio_panel {
            return Ok(());
        }
        match modal_input(action) {
            ModalInput::Close => {
                self.close_modal();
                self.paint(ui);
                return Ok(());
            }
            ModalInput::Consume => return Ok(()),
            ModalInput::Dispatch => {}
        }
        self.audio_action(ui, action)?;
        // The audio panel's explicit last row is another close action. Restore
        // the same snapshot as B, including the originating settings row.
        if !self.audio_panel {
            self.close_modal();
            self.paint(ui);
        }
        Ok(())
    }

    pub(super) fn navigate(&mut self, ui: &Home, destination: usize) {
        self.change_section(ui, destination, true);
    }

    pub(super) fn change_section(&mut self, ui: &Home, destination: usize, from_dock: bool) {
        let Some(section) = Section::from_index(destination) else {
            return;
        };
        if self.modal_active() {
            return;
        }
        let old_section = self.section() as i32;
        if from_dock {
            ui.set_navigation_direction(if destination as i32 >= old_section {
                1
            } else {
                -1
            });
        }
        self.remember_focus();
        let remembered = self
            .navigation
            .recalled(section)
            .unwrap_or_else(|| FocusSnapshot {
                section,
                index: match section {
                    Section::Continue => self.resume_focus,
                    Section::Moment => self.emotional_focus,
                    Section::Library => self.library_index,
                    Section::Settings => self.menu_index,
                },
                game_id: None,
                target: FocusTarget::Content,
                history: false,
            });
        self.restore_focus(remembered);
        self.clear_armed = false;
        self.menu_from_library = false;
        if from_dock {
            self.dock_focus = Some(destination);
            self.system_focus = false;
        }
        ui.set_library_offset(-1);
        self.paint(ui);
    }

    pub(super) fn step_section(&mut self, ui: &Home, forward: bool) {
        ui.set_navigation_direction(if forward { 1 } else { -1 });
        self.change_section(ui, self.section().step(forward) as usize, false);
    }

    pub(super) fn close_status(&mut self) {
        self.close_modal();
    }

    pub(super) fn status_action(&mut self, ui: &Home, action: Action) -> bool {
        if self.system_open {
            if action == Action::Confirm {
                self.close_modal();
                self.navigate(ui, 3);
            }
            self.paint(ui);
            return true;
        }
        if self.system_focus {
            match action {
                Action::Confirm => self.open_status(),
                Action::Up => {}
                _ => {
                    self.system_focus = false;
                    self.dock_focus = self.system_return_dock.take();
                }
            }
            self.paint(ui);
            return true;
        }
        false
    }

    /// The shared navigation is now above content. Active destination and
    /// focused cursor remain independent; L/R is handled before this method.
    pub(super) fn dock_action(&mut self, ui: &Home, action: Action) -> bool {
        if let Some(cursor) = self.dock_focus {
            match action {
                Action::Left => self.dock_focus = Some(cursor.saturating_sub(1)),
                Action::Right if cursor == 3 => {
                    // The capsule is physically beside Settings, not above it.
                    // Keep the existing Up shortcut and make the spatial route
                    // reachable without discovering a second, hidden Up step.
                    self.system_return_dock = self.dock_focus.take();
                    self.system_focus = true;
                }
                Action::Right => self.dock_focus = Some(cursor + 1),
                Action::Confirm => self.navigate(ui, cursor),
                Action::Up => {
                    self.system_return_dock = self.dock_focus.take();
                    self.system_focus = true;
                }
                Action::Down | Action::Back => {
                    let empty = self.library && self.games.is_empty()
                        || !self.library
                            && !self.menu
                            && (self.resume && self.recent.is_none()
                                || !self.resume && self.order.is_empty());
                    if !empty {
                        self.dock_focus = None;
                        if !self.library && !self.menu {
                            self.selected = self.dock_return;
                        }
                    }
                }
                _ => return false,
            }
            self.paint(ui);
            return true;
        }
        let first = if self.menu {
            self.menu_index == 0
        } else if self.library {
            if self.history_panel {
                self.library_index == 0
            } else {
                self.browser.toolbar
            }
        } else {
            self.selected == 0
        };
        if action == Action::Up && first {
            self.dock_return = self.selected;
            self.remember_focus();
            self.dock_focus = Some(self.section() as usize);
            self.system_focus = false;
            self.paint(ui);
            return true;
        }
        // Legacy footer slots are no longer visible. Spatial movement stays
        // among actual cards; no direction can focus an invisible old shortcut.
        if !self.menu && !self.library && action.navigation() {
            self.selected = card_focus(
                self.selected,
                action,
                self.resume,
                self.order.len(),
                self.continuity_order.len(),
            );
            self.paint(ui);
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoulders_visit_all_four_sections_in_both_directions() {
        let mut section = Section::Continue;
        let mut forward = Vec::new();
        for _ in 0..4 {
            section = section.step(true);
            forward.push(section);
        }
        assert_eq!(
            forward,
            [
                Section::Moment,
                Section::Library,
                Section::Settings,
                Section::Continue
            ]
        );
        let mut reverse = Vec::new();
        for _ in 0..4 {
            section = section.step(false);
            reverse.push(section);
        }
        assert_eq!(
            reverse,
            [
                Section::Settings,
                Section::Library,
                Section::Moment,
                Section::Continue
            ]
        );
    }

    #[test]
    fn rapid_switches_have_no_pending_transition_state() {
        let mut section = Section::Continue;
        for _ in 0..3 {
            section = section.step(true);
        }
        assert_eq!(section, Section::Settings);
        for _ in 0..101 {
            section = section.step(false);
        }
        assert_eq!(section, Section::Library);
        assert!(Section::from_index(4).is_none());
    }

    fn focus(section: Section, index: usize, id: &str) -> FocusSnapshot {
        FocusSnapshot {
            section,
            index,
            game_id: Some(id.into()),
            target: FocusTarget::Content,
            history: false,
        }
    }

    #[test]
    fn each_section_retains_its_own_content_and_global_focus() {
        let mut memory = NavigationMemory::default();
        let mut resume = focus(Section::Continue, 5, "resume-game");
        resume.target = FocusTarget::Dock(2);
        let moment = focus(Section::Moment, 1, "moment-game");
        let library = focus(Section::Library, 800, "library-game");
        let settings = focus(Section::Settings, 7, "");
        for snapshot in [&resume, &moment, &library, &settings] {
            memory.remember(snapshot.clone());
        }
        for snapshot in [resume, moment, library, settings] {
            assert_eq!(memory.recalled(snapshot.section).as_ref(), Some(&snapshot));
        }
    }

    #[test]
    fn library_refresh_tracks_identity_and_handles_removal_and_empty_catalog() {
        assert_eq!(
            retained_index(2, Some("c"), ["c", "a", "b"].into_iter(), 3),
            0
        );
        assert_eq!(
            retained_index(800, Some("removed"), ["a", "b"].into_iter(), 2),
            1
        );
        assert_eq!(retained_index(1, Some("c"), [].into_iter(), 0), 0);
    }

    #[test]
    fn removed_or_reordered_recommendations_never_focus_hidden_footer_controls() {
        for resume in [true, false] {
            for count in 0..10 {
                for old in 0..12 {
                    let selected = content_index(resume, old, count, count.min(3));
                    assert!(if count == 0 {
                        selected == 2
                    } else if resume {
                        selected == 0 || selected == 4 && count > 1 || selected == 5 && count > 2
                    } else {
                        selected == 0 || selected == 1 && count > 1
                    });
                }
            }
        }
    }

    #[test]
    fn all_card_directions_stay_on_visible_cards() {
        for resume in [true, false] {
            for count in 0..5 {
                for current in 0..7 {
                    for action in [Action::Up, Action::Down, Action::Left, Action::Right] {
                        let next = card_focus(current, action, resume, count, count.min(3));
                        assert_eq!(next, content_index(resume, next, count, count.min(3)));
                        assert_ne!(next, 3);
                    }
                }
            }
        }
        assert_eq!(card_focus(1, Action::Left, false, 3, 0), 1);
        assert_eq!(card_focus(5, Action::Down, true, 3, 3), 5);
    }

    #[test]
    fn modal_back_is_consumed_and_shoulders_never_reach_underlying_navigation() {
        assert_eq!(modal_input(Action::Back), ModalInput::Close);
        assert_eq!(modal_input(Action::Menu), ModalInput::Close);
        for action in [Action::Previous, Action::Next, Action::Library] {
            assert_eq!(modal_input(action), ModalInput::Consume);
        }
        for action in [Action::Up, Action::Down, Action::Confirm] {
            assert_eq!(modal_input(action), ModalInput::Dispatch);
        }
    }
}
