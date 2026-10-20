use std::{collections::BTreeMap, time::Duration};
/// Tracks independent held controls: another press cannot re-arm confirmation.
#[derive(Default)]
pub struct RepeatGate {
    held: BTreeMap<String, (Duration, Duration)>,
}
impl RepeatGate {
    pub fn press(&mut self, key: &str, now: Duration, navigation: bool) -> bool {
        if let Some((since, last)) = self.held.get_mut(key) {
            if !navigation
                || now.saturating_sub(*since) < Duration::from_millis(320)
                || now.saturating_sub(*last) < Duration::from_millis(120)
            {
                return false;
            }
            *last = now;
        } else {
            self.held.insert(key.into(), (now, now));
        }
        true
    }
    pub fn is_held(&self) -> bool {
        !self.held.is_empty()
    }
    pub fn release_key(&mut self, key: &str) {
        self.held.remove(key);
    }
    pub fn release(&mut self) {
        self.held.clear();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeat_is_bounded() {
        let mut g = RepeatGate::default();
        assert!(g.press("down", Duration::ZERO, true));
        assert!(!g.press("down", Duration::from_millis(100), true));
        assert!(g.press("down", Duration::from_millis(330), true));
        assert!(!g.press("down", Duration::from_millis(400), true));
        g.release();
        assert!(g.press("down", Duration::from_millis(401), true));
    }
    #[test]
    fn confirm_never_repeats() {
        let mut g = RepeatGate::default();
        assert!(g.press("a", Duration::ZERO, false));
        assert!(!g.press("a", Duration::from_secs(3), false));
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}
/// Home has one primary action above up to two alternatives; no edge wrapping.
pub fn home_focus(current: usize, direction: Direction, count: usize) -> usize {
    if count == 0 {
        return 0;
    }
    let current = current.min(count - 1);
    match (current, direction) {
        (0, Direction::Down) => {
            if count > 1 {
                1
            } else {
                0
            }
        }
        (1 | 2, Direction::Up) => 0,
        (1, Direction::Right) if count > 2 => 2,
        (2, Direction::Left) => 1,
        _ => current,
    }
}
#[cfg(test)]
mod navigation_tests {
    use super::*;
    #[test]
    fn spatial_navigation() {
        assert_eq!(home_focus(0, Direction::Down, 3), 1);
        assert_eq!(home_focus(1, Direction::Right, 3), 2);
        assert_eq!(home_focus(2, Direction::Up, 3), 0);
        assert_eq!(home_focus(0, Direction::Left, 3), 0);
    }
    #[test]
    fn small_catalog_focus() {
        for n in 0..=3 {
            for c in 0..5 {
                for d in [
                    Direction::Left,
                    Direction::Right,
                    Direction::Up,
                    Direction::Down,
                ] {
                    assert!(home_focus(c, d, n) < n.max(1));
                }
            }
        }
    }
    #[test]
    fn unrelated_release_does_not_reenable_confirmation() {
        let mut g = RepeatGate::default();
        g.press("a", Duration::ZERO, false);
        g.release_key("b");
        assert!(!g.press("a", Duration::from_secs(1), false));
        g.release_key("a");
        assert!(g.press("a", Duration::from_secs(2), false));
    }
}

#[cfg(test)]
mod simultaneous_tests {
    use super::*;
    #[test]
    fn another_key_cannot_rearm_held_confirm() {
        let mut g = RepeatGate::default();
        assert!(g.press("confirm", Duration::ZERO, false));
        assert!(g.press("right", Duration::from_millis(1), true));
        assert!(!g.press("confirm", Duration::from_secs(2), false));
        g.release_key("right");
        assert!(g.is_held());
        g.release_key("confirm");
        assert!(!g.is_held());
    }
    #[test]
    fn rapid_release_press_is_not_throttled() {
        let mut g = RepeatGate::default();
        for n in 0..100 {
            assert!(g.press("down", Duration::from_millis(n), true));
            g.release_key("down");
        }
    }
}
/// Prototype focus order: protagonist, alternative, library. Empty slots are skipped.
pub fn moment_focus(current: usize, direction: Direction, count: usize) -> usize {
    let slots = if count == 0 {
        vec![2]
    } else if count == 1 {
        vec![0, 2]
    } else {
        vec![0, 1, 2]
    };
    let position = slots.iter().position(|v| *v == current).unwrap_or(0);
    let next = match direction {
        Direction::Down | Direction::Right => (position + 1).min(slots.len() - 1),
        _ => position.saturating_sub(1),
    };
    slots[next]
}
