//! Settings navigation is pure; only the existing supervisor can run a tool.
use emotional_services::system_options::{Category, Entry};

#[derive(Clone, Debug, Default)]
enum Page {
    #[default]
    Categories,
    Tools(Category),
    Confirm {
        category: Category,
        relative: String,
        previous: usize,
    },
}
#[derive(Default)]
pub(super) struct Browser {
    page: Page,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Outcome {
    Focus(usize),
    Close,
    Controls,
    Launch(String),
}

impl Browser {
    pub fn title(&self) -> &str {
        match &self.page {
            Page::Categories => "Ajustes de dArkOS",
            Page::Tools(category) => category.label(),
            Page::Confirm { .. } => "Confirmar apertura",
        }
    }
    fn entries<'a>(category: Category, entries: &'a [Entry]) -> Vec<&'a Entry> {
        entries.iter().filter(|e| category.contains(e)).collect()
    }
    pub fn rows(&self, entries: &[Entry]) -> Vec<(String, String)> {
        match &self.page {
            Page::Categories => {
                std::iter::once(("Volumen, brillo y batería".into(), "Abrir".into()))
                    .chain(Category::ALL.into_iter().map(|c| {
                        (
                            c.label().into(),
                            Self::entries(c, entries).len().to_string(),
                        )
                    }))
                    .chain(std::iter::once(("Volver a Ajustes".into(), String::new())))
                    .collect()
            }
            Page::Tools(category) => Self::entries(*category, entries)
                .iter()
                .map(|e| (e.title.clone(), "Abrir".into()))
                .chain(std::iter::once((
                    "Volver a categorías".into(),
                    String::new(),
                )))
                .collect(),
            Page::Confirm { relative, .. } => vec![
                ("Cancelar".into(), "B".into()),
                (
                    entries
                        .iter()
                        .find(|e| &e.relative == relative)
                        .map(|e| e.title.clone())
                        .unwrap_or_else(|| "Herramienta no disponible".into()),
                    "Abrir".into(),
                ),
            ],
        }
    }
    pub fn hint(&self, index: usize, entries: &[Entry]) -> String {
        match &self.page {
            Page::Categories => "Ajustes compartidos con las herramientas de tu consola.".into(),
            Page::Tools(category) => Self::entries(*category, entries)
                .get(index)
                .map(|e| e.notice().into())
                .unwrap_or_else(|| "B vuelve a las categorías.".into()),
            Page::Confirm { relative, .. } => entries
                .iter()
                .find(|e| &e.relative == relative)
                .map(|e| e.notice().into())
                .unwrap_or_else(|| "La herramienta ya no está disponible.".into()),
        }
    }
    pub fn activate(&mut self, index: usize, entries: &[Entry]) -> Outcome {
        match &self.page {
            Page::Categories if index == 0 => Outcome::Controls,
            Page::Categories => {
                if let Some(category) = index
                    .checked_sub(1)
                    .and_then(|i| Category::ALL.get(i))
                    .copied()
                {
                    self.page = Page::Tools(category);
                    Outcome::Focus(0)
                } else {
                    Outcome::Close
                }
            }
            Page::Tools(category) => {
                if let Some(entry) = Self::entries(*category, entries).get(index) {
                    self.page = Page::Confirm {
                        category: *category,
                        relative: entry.relative.clone(),
                        previous: index,
                    };
                    Outcome::Focus(0) // Cancel is focused; a repeated A cannot run a tool.
                } else {
                    self.back()
                }
            }
            Page::Confirm { relative, .. }
                if index == 1 && entries.iter().any(|e| &e.relative == relative) =>
            {
                Outcome::Launch(relative.clone())
            }
            Page::Confirm { .. } => self.back(),
        }
    }
    pub fn back(&mut self) -> Outcome {
        match &self.page {
            Page::Categories => Outcome::Close,
            Page::Tools(category) => {
                let index = Category::ALL
                    .iter()
                    .position(|c| c == category)
                    .unwrap_or(0)
                    + 1;
                self.page = Page::Categories;
                Outcome::Focus(index)
            }
            Page::Confirm {
                category, previous, ..
            } => {
                let (category, previous) = (*category, *previous);
                self.page = Page::Tools(category);
                Outcome::Focus(previous)
            }
        }
    }
    pub fn show_tool(&mut self, relative: &str, entries: &[Entry], confirm: bool) -> usize {
        let Some(entry) = entries.iter().find(|e| e.relative == relative) else {
            self.page = Page::Categories;
            return 0;
        };
        let category = entry.category();
        let previous = Self::entries(category, entries)
            .iter()
            .position(|e| e.relative == relative)
            .unwrap_or(0);
        self.page = if confirm {
            Page::Confirm {
                category,
                relative: relative.into(),
                previous,
            }
        } else {
            Page::Tools(category)
        };
        if confirm {
            0
        } else {
            previous
        }
    }
    pub fn reset(&mut self) {
        self.page = Page::Categories;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tools() -> Vec<Entry> {
        vec![
            Entry {
                relative: "Wifi.sh".into(),
                title: "Wi-Fi".into(),
            },
            Entry {
                relative: "Advanced/Reset Retroarch Settings.sh".into(),
                title: "Restaurar RetroArch".into(),
            },
        ]
    }
    #[test]
    fn opening_and_cancelling_never_launches_even_with_repeated_confirmation() {
        let mut b = Browser::default();
        let e = tools();
        assert_eq!(b.activate(1, &e), Outcome::Focus(0));
        assert_eq!(b.activate(0, &e), Outcome::Focus(0));
        assert_eq!(b.activate(0, &e), Outcome::Focus(0));
        assert_eq!(b.activate(0, &e), Outcome::Focus(0));
        assert_eq!(b.activate(1, &e), Outcome::Launch("Wifi.sh".into()));
    }
    #[test]
    fn back_restores_category_and_tool_focus_and_return_uses_identity() {
        let mut b = Browser::default();
        let e = tools();
        assert_eq!(b.show_tool("Wifi.sh", &e, true), 0);
        assert_eq!(b.back(), Outcome::Focus(0));
        assert_eq!(b.back(), Outcome::Focus(1));
        let mut reordered = e.clone();
        reordered.reverse();
        let at = b.show_tool("Wifi.sh", &reordered, false);
        assert_eq!(b.rows(&reordered)[at].0, "Wi-Fi");
    }
    #[test]
    fn empty_categories_have_a_back_action_and_missing_tools_cannot_launch() {
        let mut b = Browser::default();
        let e = tools();
        b.activate(2, &e);
        assert_eq!(b.rows(&e).len(), 1);
        assert_eq!(b.activate(0, &e), Outcome::Focus(2));
        b.show_tool("Wifi.sh", &e, true);
        assert_eq!(b.activate(1, &[]), Outcome::Focus(0));
        assert!(!matches!(b.activate(1, &[]), Outcome::Launch(_)));
    }
}
