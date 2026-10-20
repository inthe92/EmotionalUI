use emotional_core::controls::Action;
use serde::Deserialize;
use std::path::PathBuf;

pub fn keyboard(key: &str) -> Option<Action> {
    let arrow = match key.chars().next()? {
        c if c == char::from(slint::platform::Key::LeftArrow) => Some(Action::Left),
        c if c == char::from(slint::platform::Key::RightArrow) => Some(Action::Right),
        c if c == char::from(slint::platform::Key::UpArrow) => Some(Action::Up),
        c if c == char::from(slint::platform::Key::DownArrow) => Some(Action::Down),
        c if c == char::from(slint::platform::Key::Return) => Some(Action::Confirm),
        c if c == char::from(slint::platform::Key::Escape) => Some(Action::Back),
        c if c == char::from(slint::platform::Key::F1) => Some(Action::Menu),
        c if c == char::from(slint::platform::Key::F2) => Some(Action::MomentPrevious),
        c if c == char::from(slint::platform::Key::F3) => Some(Action::MomentNext),
        _ => None,
    };
    arrow.or_else(|| match key.to_lowercase().as_str() {
        "z" | " " => Some(Action::Confirm),
        "x" | "b" => Some(Action::Back),
        "n" => Some(Action::Refresh),
        "a" | "y" | "v" => Some(Action::Library),
        "m" => Some(Action::ReducedMotion),
        "s" => Some(Action::Menu),
        "d" => Some(Action::Details),
        "q" | "l" => Some(Action::Previous),
        "e" | "r" => Some(Action::Next),
        "[" => Some(Action::PreviewPrevious),
        "]" => Some(Action::PreviewNext),
        _ => None,
    })
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Keyboard {
    #[serde(default)]
    pub bindings: std::collections::BTreeMap<String, Action>,
    #[serde(skip)]
    held: std::collections::BTreeMap<String, Action>,
}
impl Keyboard {
    pub fn press(&mut self, key: &str) -> Option<Action> {
        let action = self.action(key)?;
        let key = key.to_lowercase();
        if self.held.contains_key(&key) {
            return Some(action);
        }
        let already = self.held.values().any(|a| *a == action);
        self.held.insert(key, action);
        (!already).then_some(action)
    }
    pub fn release(&mut self, key: &str) -> Option<Action> {
        let action = self.held.remove(&key.to_lowercase())?;
        (!self.held.values().any(|a| *a == action)).then_some(action)
    }
    pub fn reset(&mut self) {
        self.held.clear();
    }
    pub fn validate(&self) -> Result<(), String> {
        const NAMES: &[&str] = &["up", "down", "left", "right", "enter", "escape", "start"];
        if self.bindings.len() > 64
            || self.bindings.keys().any(|k| {
                k != &k.to_lowercase() || (k.chars().count() != 1 && !NAMES.contains(&k.as_str()))
            })
        {
            return Err("Keyboard bindings need lowercase characters or up/down/left/right/enter/escape/start".into());
        }
        for action in [
            Action::Up,
            Action::Down,
            Action::Left,
            Action::Right,
            Action::Confirm,
            Action::Back,
            Action::Menu,
        ] {
            let possible = [
                "up", "down", "left", "right", "enter", "escape", "start", "z", "x", "s",
            ];
            if !possible.iter().any(|k| self.named(k) == Some(action))
                && !self.bindings.values().any(|v| *v == action)
            {
                return Err("Keyboard configuration removes an essential action".into());
            }
        }
        Ok(())
    }
    fn named(&self, key: &str) -> Option<Action> {
        self.bindings.get(key).copied().or_else(|| match key {
            "up" => Some(Action::Up),
            "down" => Some(Action::Down),
            "left" => Some(Action::Left),
            "right" => Some(Action::Right),
            "enter" => Some(Action::Confirm),
            "escape" => Some(Action::Back),
            "start" => Some(Action::Menu),
            _ => keyboard(key),
        })
    }
    pub fn action(&self, key: &str) -> Option<Action> {
        let canonical = match key.chars().next()? {
            c if c == char::from(slint::platform::Key::UpArrow) => "up",
            c if c == char::from(slint::platform::Key::DownArrow) => "down",
            c if c == char::from(slint::platform::Key::LeftArrow) => "left",
            c if c == char::from(slint::platform::Key::RightArrow) => "right",
            c if c == char::from(slint::platform::Key::Return) => "enter",
            c if c == char::from(slint::platform::Key::Escape) => "escape",
            c if c == char::from(slint::platform::Key::F1) => "start",
            _ => return self.named(&key.to_lowercase()),
        };
        self.named(canonical)
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub enabled: bool,
    pub device_name: String,
    pub device_path: Option<PathBuf>,
    pub mapping: emotional_core::controls::Mapping,
}
impl Profile {
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Err(
                "Controller profile is disabled; identify controls before enabling it".into(),
            );
        }
        if self.device_name.is_empty() || self.device_name.len() > 128 {
            return Err("Invalid controller name".into());
        }
        if self
            .device_path
            .as_ref()
            .is_some_and(|p| !p.is_absolute() || !p.starts_with("/dev/input"))
        {
            return Err("Expected /dev/input path".into());
        }
        self.mapping.validate()?;
        let actions = self
            .mapping
            .buttons
            .iter()
            .map(|b| b.action)
            .chain(
                self.mapping
                    .axes
                    .iter()
                    .flat_map(|a| [a.negative, a.positive]),
            )
            .collect::<std::collections::BTreeSet<_>>();
        if [
            Action::Left,
            Action::Right,
            Action::Up,
            Action::Down,
            Action::Confirm,
            Action::Back,
            Action::Menu,
        ]
        .iter()
        .any(|a| !actions.contains(a))
        {
            return Err("Controller profile lacks essential navigation actions".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn aliases_share_an_action() {
        assert_eq!(keyboard("Z"), keyboard("z"));
        assert_eq!(keyboard("z"), keyboard(" "));
        assert_eq!(
            keyboard(&char::from(slint::platform::Key::Return).to_string()),
            Some(Action::Confirm)
        );
    }
    #[test]
    fn configurable_mapping_preserves_abstract_actions() {
        let k: Keyboard = toml::from_str("[bindings]\nz='back'\nx='confirm'\n").unwrap();
        k.validate().unwrap();
        assert_eq!(k.action("z"), Some(Action::Back));
        assert_eq!(k.action("X"), Some(Action::Confirm));
        assert_eq!(Keyboard::default().action("s"), Some(Action::Menu));
        let invalid: Keyboard = toml::from_str("[bindings]\nUP='up'").unwrap();
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn releasing_one_alias_keeps_the_other_held() {
        let mut k = Keyboard::default();
        let enter = char::from(slint::platform::Key::Return).to_string();
        assert_eq!(k.press("Z"), Some(Action::Confirm));
        assert_eq!(k.press(&enter), None);
        assert_eq!(k.release("z"), None);
        assert_eq!(k.release(&enter), Some(Action::Confirm));
        k.press("x");
        k.reset();
        assert_eq!(k.release("x"), None);
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    fn profile() -> Profile {
        Profile {
            enabled: true,
            device_name: "retrogame_joypad".into(),
            device_path: None,
            mapping: emotional_core::controls::Mapping {
                buttons: [
                    Action::Left,
                    Action::Right,
                    Action::Up,
                    Action::Down,
                    Action::Confirm,
                    Action::Back,
                    Action::Menu,
                ]
                .into_iter()
                .enumerate()
                .map(|(i, action)| emotional_core::controls::Button {
                    code: i as u16,
                    action,
                })
                .collect(),
                axes: vec![],
            },
        }
    }
    #[test]
    fn disabled_or_incomplete_cannot_enable_input() {
        let mut p = profile();
        assert!(p.validate().is_ok());
        p.enabled = false;
        assert!(p.validate().is_err());
        p.enabled = true;
        p.mapping.buttons.pop();
        assert!(p.validate().is_err());
    }
    #[test]
    fn arbitrary_file_is_not_an_input_profile_device() {
        let mut p = profile();
        p.device_path = Some("/etc/passwd".into());
        assert!(p.validate().is_err());
    }
}
