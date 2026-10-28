//! Local, bounded library grouping and search; no file or network access.
use emotional_core::{atlas::title_key, controls::Action, library::Game};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    mode: String,
    system: Option<String>,
    query: String,
    cursor: usize,
}
impl Saved {
    pub fn valid(&self) -> bool {
        ["systems", "games", "results"].contains(&self.mode.as_str())
            && self.query.chars().count() <= 64
            && self.query.len() <= 256
            && self
                .system
                .as_ref()
                .is_none_or(|s| s.len() <= 128 && !s.chars().any(char::is_control))
            && self.cursor <= 1_000_000
    }
}

pub const KEYS: [&str; 40] = [
    "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S",
    "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "Espacio",
    "Borrar", "Buscar", "Volver",
];
#[derive(Clone, Debug, PartialEq, Eq)]
enum Page {
    Systems,
    Games(Option<String>),
    Search,
    Results,
}
#[derive(Clone, Debug)]
struct Group {
    key: String,
    label: String,
    count: usize,
}
pub(super) enum Choice {
    Changed,
    Game(usize),
    None,
}
pub(super) struct Browser {
    page: Page,
    pub cursor: usize,
    pub key: usize,
    pub query: String,
    groups: Vec<Group>,
    items: Vec<usize>,
    item_ids: Vec<String>,
    titles: Vec<String>,
}
impl Browser {
    pub fn new(games: &[Game]) -> Self {
        let mut b = Self {
            page: Page::Systems,
            cursor: 0,
            key: 0,
            query: String::new(),
            groups: Vec::new(),
            items: Vec::new(),
            item_ids: Vec::new(),
            titles: Vec::new(),
        };
        b.rebuild(games);
        b
    }
    pub fn searching(&self) -> bool {
        self.page == Page::Search
    }
    pub fn system_page(&self) -> bool {
        self.page == Page::Systems
    }
    pub fn count(&self) -> usize {
        if self.system_page() {
            self.groups.len() + 2
        } else if self.searching() {
            0
        } else {
            self.items.len()
        }
    }
    pub fn selected_game(&self) -> Option<usize> {
        self.items
            .get(self.cursor)
            .copied()
            .filter(|_| !self.system_page() && !self.searching())
    }
    pub fn current_game_id(&self) -> Option<&str> {
        if self.system_page() || self.searching() {
            None
        } else {
            self.item_ids.get(self.cursor).map(String::as_str)
        }
    }
    pub fn restore(&mut self, index: usize, id: Option<&str>) {
        self.cursor = id
            .and_then(|value| self.item_ids.iter().position(|v| v == value))
            .unwrap_or(index)
            .min(self.count().saturating_sub(1));
    }
    pub fn title(&self) -> String {
        match &self.page {
            Page::Systems => "Elige una consola o busca un juego".into(),
            Page::Games(None) => format!("Todos los juegos · {}", self.items.len()),
            Page::Games(Some(key)) => format!("{} · {}", label(key), self.items.len()),
            Page::Search => "Buscar juegos".into(),
            Page::Results => format!(
                "«{}» · {} {}",
                self.query.trim(),
                self.items.len(),
                if self.items.len() == 1 {
                    "resultado"
                } else {
                    "resultados"
                }
            ),
        }
    }
    pub fn empty_text(&self) -> &'static str {
        if self.page == Page::Results {
            "No hay juegos con ese nombre."
        } else {
            "No hay juegos disponibles en esta consola."
        }
    }
    pub fn rows(&self, games: &[Game]) -> Vec<String> {
        (0..self.count())
            .filter_map(|i| self.row(games, i))
            .collect()
    }
    pub fn row(&self, games: &[Game], row: usize) -> Option<String> {
        if self.system_page() {
            return match row {
                0 => Some("Buscar juegos".into()),
                1 => Some(format!("Todos los juegos · {}", games.len())),
                _ => self
                    .groups
                    .get(row - 2)
                    .map(|g| format!("{} · {}", g.label, g.count)),
            };
        }
        if self.searching() {
            return None;
        }
        let g = games.get(*self.items.get(row)?)?;
        Some(if self.page == Page::Results {
            format!("{}  ·  {}", g.title, label(&system(g)))
        } else {
            g.title.clone()
        })
    }
    pub fn saved(&self) -> Saved {
        Saved {
            mode: match self.page {
                Page::Games(_) => "games",
                Page::Results => "results",
                _ => "systems",
            }
            .into(),
            system: match &self.page {
                Page::Games(system) => system.clone(),
                _ => None,
            },
            query: self.query.clone(),
            cursor: self.cursor,
        }
    }
    pub fn restore_saved(&mut self, saved: &Saved, games: &[Game], id: Option<&str>) {
        if !saved.valid() {
            return;
        }
        self.page = match saved.mode.as_str() {
            "games" => Page::Games(saved.system.clone()),
            "results" => Page::Results,
            _ => Page::Systems,
        };
        self.query = saved.query.clone();
        self.update_items(games);
        self.restore(saved.cursor, id);
    }
    pub fn open_search(&mut self) {
        self.page = Page::Search;
        self.key = 0;
    }
    pub fn rebuild(&mut self, games: &[Game]) {
        let selected = self.current_game_id().map(str::to_owned);
        let selected_system = self
            .system_page()
            .then(|| {
                self.cursor
                    .checked_sub(2)
                    .and_then(|i| self.groups.get(i))
                    .map(|g| g.key.clone())
            })
            .flatten();
        let mut groups = BTreeMap::<String, usize>::new();
        for g in games {
            *groups.entry(system(g)).or_default() += 1;
        }
        self.groups = groups
            .into_iter()
            .map(|(key, count)| Group {
                label: label(&key),
                key,
                count,
            })
            .collect();
        self.groups.sort_by(|a, b| {
            a.label
                .to_lowercase()
                .cmp(&b.label.to_lowercase())
                .then(a.key.cmp(&b.key))
        });
        self.titles = games.iter().map(|g| title_key(&g.title)).collect();
        self.update_items(games);
        self.restore(self.cursor, selected.as_deref());
        if let Some(key) = selected_system {
            if let Some(i) = self.groups.iter().position(|g| g.key == key) {
                self.cursor = i + 2;
            }
        }
    }
    fn update_items(&mut self, games: &[Game]) {
        let query = title_key(&self.query);
        self.items = games
            .iter()
            .enumerate()
            .filter(|(i, g)| match &self.page {
                Page::Games(None) => true,
                Page::Games(Some(key)) => system(g) == *key,
                Page::Results => self.titles.get(*i).is_some_and(|t| t.contains(&query)),
                _ => false,
            })
            .map(|(i, _)| i)
            .collect();
        self.items.sort_by(|a, b| {
            self.titles[*a]
                .cmp(&self.titles[*b])
                .then(games[*a].id.cmp(&games[*b].id))
        });
        self.item_ids = self.items.iter().map(|i| games[*i].id.clone()).collect();
    }
    pub fn choose(&mut self, row: usize, games: &[Game]) -> Choice {
        if row >= self.count() {
            return Choice::None;
        }
        self.cursor = row;
        if self.system_page() {
            if row == 0 {
                self.page = Page::Search;
                self.key = 0;
            } else {
                self.page = Page::Games(if row == 1 {
                    None
                } else {
                    Some(self.groups[row - 2].key.clone())
                });
                self.update_items(games);
                self.cursor = 0;
            }
            Choice::Changed
        } else {
            self.selected_game()
                .map(Choice::Game)
                .unwrap_or(Choice::None)
        }
    }
    pub fn back(&mut self, games: &[Game]) -> bool {
        match self.page {
            Page::Systems => false,
            Page::Games(_) | Page::Results | Page::Search => {
                self.page = Page::Systems;
                self.update_items(games);
                self.cursor = 0;
                true
            }
        }
    }
    pub fn move_row(&mut self, action: Action) {
        self.cursor = match action {
            Action::Up => self.cursor.saturating_sub(1),
            Action::Down => (self.cursor + 1).min(self.count().saturating_sub(1)),
            _ => self.cursor,
        };
    }
    pub fn search_action(&mut self, action: Action, games: &[Game]) {
        if !self.searching() {
            return;
        }
        match action {
            Action::Left => self.key = self.key.saturating_sub(1),
            Action::Right => self.key = (self.key + 1).min(KEYS.len() - 1),
            Action::Up => self.key = self.key.saturating_sub(8),
            Action::Down => self.key = (self.key + 8).min(KEYS.len() - 1),
            Action::Confirm => self.choose_key(self.key, games),
            Action::Refresh => {
                self.query.pop();
            }
            Action::Menu => self.finish_search(games),
            Action::Back => {
                self.back(games);
            }
            _ => {}
        }
    }
    pub fn choose_key(&mut self, key: usize, games: &[Game]) {
        if !self.searching() || key >= KEYS.len() {
            return;
        }
        self.key = key;
        match key {
            0..=35 if self.query.chars().count() < 64 => self.query.push_str(KEYS[key]),
            36 if self.query.chars().count() < 64 => self.query.push(' '),
            37 => {
                self.query.pop();
            }
            38 => self.finish_search(games),
            39 => {
                self.back(games);
            }
            _ => {}
        }
    }
    fn finish_search(&mut self, games: &[Game]) {
        self.page = Page::Results;
        self.update_items(games);
        self.cursor = 0;
    }
}
fn system(g: &Game) -> String {
    let key = g.system.trim();
    if key.is_empty() {
        "unknown".into()
    } else {
        key.to_lowercase()
    }
}
fn label(key: &str) -> String {
    match key {
        "nes" => "NES".into(),
        "snes" => "Super Nintendo".into(),
        "gb" => "Game Boy".into(),
        "gbc" => "Game Boy Color".into(),
        "gba" => "Game Boy Advance".into(),
        "n64" => "Nintendo 64".into(),
        "nds" => "Nintendo DS".into(),
        "megadrive" => "Mega Drive".into(),
        "psx" => "PlayStation".into(),
        "psp" => "PSP".into(),
        "unknown" => "Sin sistema".into(),
        _ => key.to_uppercase(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn games() -> Vec<Game> {
        let mut g = emotional_core::library::demo_catalog();
        g.truncate(3);
        g[0].system = "snes".into();
        g[0].title = "La montaña".into();
        g[1].system = "nes".into();
        g[1].title = "Mario".into();
        g[2].system = "snes".into();
        g[2].title = "Zelda".into();
        g
    }
    #[test]
    fn groups_are_separate_and_map_to_original_game_identity() {
        let g = games();
        let mut b = Browser::new(&g);
        assert_eq!(
            b.rows(&g),
            vec![
                "Buscar juegos",
                "Todos los juegos · 3",
                "NES · 1",
                "Super Nintendo · 2"
            ]
        );
        assert!(matches!(b.choose(3, &g), Choice::Changed));
        assert_eq!(b.rows(&g), vec!["La montaña", "Zelda"]);
        assert!(matches!(b.choose(1, &g), Choice::Game(2)));
        assert!(b.back(&g));
    }
    #[test]
    fn search_is_accent_insensitive_and_keyboard_can_cancel_and_edit() {
        let g = games();
        let mut b = Browser::new(&g);
        b.choose(0, &g);
        for ch in "MONTANA".chars() {
            let k = KEYS.iter().position(|v| *v == ch.to_string()).unwrap();
            b.choose_key(k, &g)
        }
        b.search_action(Action::Menu, &g);
        assert_eq!(b.rows(&g), vec!["La montaña  ·  Super Nintendo"]);
        assert!(matches!(b.choose(0, &g), Choice::Game(0)));
        assert!(b.back(&g));
        b.choose(0, &g);
        b.choose_key(37, &g);
        assert_eq!(b.query, "MONTAN");
        b.search_action(Action::Back, &g);
        assert!(b.system_page());
    }
    #[test]
    fn refresh_retains_selected_identity_after_reorder_and_handles_removed() {
        let mut g = games();
        let mut b = Browser::new(&g);
        b.choose(1, &g);
        b.cursor = 1;
        let wanted = b.current_game_id().unwrap().to_owned();
        g.reverse();
        b.rebuild(&g);
        assert_eq!(b.current_game_id(), Some(wanted.as_str()));
        g.retain(|x| x.id != wanted);
        b.rebuild(&g);
        assert!(b.cursor < b.count());
    }
    #[test]
    fn saved_search_restores_identity_and_rejects_invalid_state() {
        let mut g = games();
        let mut b = Browser::new(&g);
        b.choose(0, &g);
        b.search_action(Action::Menu, &g);
        b.cursor = 1;
        let wanted = b.current_game_id().unwrap().to_owned();
        let saved = b.saved();
        g.reverse();
        let mut next = Browser::new(&g);
        next.restore_saved(&saved, &g, Some(&wanted));
        assert_eq!(next.current_game_id(), Some(wanted.as_str()));
        let encoded = toml::to_string(&saved).unwrap();
        assert!(toml::from_str::<Saved>(&encoded).unwrap().valid());
        let invalid = Saved {
            mode: "games".into(),
            system: None,
            query: "x".repeat(65),
            cursor: 0,
        };
        assert!(!invalid.valid());
    }
    #[test]
    fn identical_titles_in_distinct_systems_keep_distinct_game_indices() {
        let mut g = games();
        g[0].title = "Same title".into();
        g[1].title = "Same title".into();
        let mut b = Browser::new(&g);
        b.choose(2, &g);
        assert!(matches!(b.choose(0, &g), Choice::Game(1)));
        b.back(&g);
        b.choose(3, &g);
        assert!(matches!(b.choose(0, &g), Choice::Game(0)));
    }
    #[test]
    fn new_system_from_import_does_not_move_the_focused_console() {
        let mut g = games();
        let mut b = Browser::new(&g);
        b.cursor = 3; // Super Nintendo.
        let mut new = g[0].clone();
        new.id = "new-gba".into();
        new.system = "gba".into();
        g.push(new);
        b.rebuild(&g);
        assert_eq!(b.row(&g, b.cursor).as_deref(), Some("Super Nintendo · 2"));
    }
    #[test]
    fn physical_keyboard_navigation_edits_searches_and_handles_empty_results() {
        let g = games();
        let mut b = Browser::new(&g);
        b.open_search();
        b.search_action(Action::Down, &g); // I.
        b.search_action(Action::Right, &g); // J.
        b.search_action(Action::Confirm, &g);
        assert_eq!(b.query, "J");
        b.search_action(Action::Refresh, &g); // X deletes.
        b.search_action(Action::Confirm, &g);
        b.search_action(Action::Menu, &g); // Start searches.
        assert_eq!(b.count(), 0);
        assert!(matches!(b.choose(0, &g), Choice::None));
        assert!(b.back(&g));
        assert!(b.system_page());
        let mut empty = Browser::new(&[]);
        empty.choose(1, &[]);
        empty.move_row(Action::Down);
        assert_eq!(empty.count(), 0);
        assert_eq!(empty.selected_game(), None);
        assert!(empty.back(&[]));
        assert_eq!(empty.count(), 2); // Search and All remain accessible.
        empty.choose(0, &[]);
        empty.choose_key(39, &[]); // Touchscreen Volver also cancels.
        assert!(empty.system_page());
    }
}
