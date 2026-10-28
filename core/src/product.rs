//! Product state independent of Slint, evdev and SQLite.
use crate::library::Game;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Privacy {
    pub statistics: bool,
    pub record_history: bool,
    pub personalized: bool,
    pub neutral: bool,
    pub ambient_motion: bool,
    pub dark_mode: bool,
}
impl Default for Privacy {
    fn default() -> Self {
        Self {
            statistics: true,
            record_history: true,
            personalized: true,
            neutral: false,
            ambient_motion: true,
            dark_mode: true,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub game_id: String,
    pub started_utc: i64,
    pub duration_seconds: Option<u32>,
}
#[derive(Clone, Debug, Default)]
pub struct History {
    pub sessions: Vec<Session>,
    pub completed: Vec<String>,
}
impl History {
    pub fn latest(&self, games: &[Game]) -> Option<usize> {
        let by_id = games
            .iter()
            .enumerate()
            .filter(|(_, g)| g.available)
            .map(|(i, g)| (g.id.as_str(), i))
            .collect::<std::collections::HashMap<_, _>>();
        self.sessions
            .iter()
            .filter_map(|s| by_id.get(s.game_id.as_str()).map(|i| (s.started_utc, *i)))
            .max_by_key(|(t, _)| *t)
            .map(|(_, i)| i)
    }
    pub fn summary(&self, games: &[Game], enabled: bool) -> String {
        if !enabled {
            return String::new();
        }
        let finished = self
            .completed
            .iter()
            .map(String::as_str)
            .collect::<std::collections::HashSet<_>>();
        let played = self
            .sessions
            .iter()
            .map(|s| s.game_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let completed = games
            .iter()
            .filter(|g| g.available && finished.contains(g.id.as_str()))
            .count();
        let underway = games
            .iter()
            .filter(|g| {
                g.available && !finished.contains(g.id.as_str()) && played.contains(g.id.as_str())
            })
            .count();
        let mut parts = Vec::new();
        if completed > 0 {
            parts.push(format!("{completed} completados"));
        }
        if underway > 0 {
            parts.push(format!("{underway} en curso"));
        }
        parts.join(" · ")
    }
}
/// Library is one directional action and one confirmation away, including empty state.
pub fn focus(current: usize, forward: bool, resume: bool, count: usize, has_recent: bool) -> usize {
    let slots: &[usize] = if resume {
        if has_recent {
            &[0, 2]
        } else {
            &[2]
        }
    } else {
        match count {
            0 => &[2],
            1 => &[0, 2],
            _ => &[0, 2, 1],
        }
    };
    let p = slots.iter().position(|i| *i == current).unwrap_or(0);
    slots[if forward {
        (p + 1).min(slots.len() - 1)
    } else {
        p.saturating_sub(1)
    }]
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::demo_catalog;
    #[test]
    fn history_is_not_playcount() {
        let g = demo_catalog();
        let h = History::default();
        assert_eq!(h.latest(&g), None);
        assert_eq!(h.summary(&g, true), "");
    }
    #[test]
    fn latest_available_and_explicit_completion() {
        let mut g = demo_catalog();
        let h = History {
            sessions: vec![
                Session {
                    game_id: g[0].id.clone(),
                    started_utc: 10,
                    duration_seconds: None,
                },
                Session {
                    game_id: g[1].id.clone(),
                    started_utc: 20,
                    duration_seconds: Some(600),
                },
                Session {
                    game_id: "absent".into(),
                    started_utc: 100,
                    duration_seconds: None,
                },
            ],
            completed: vec![g[0].id.clone()],
        };
        assert_eq!(h.latest(&g), Some(1));
        assert_eq!(h.summary(&g, true), "1 completados · 1 en curso");
        assert_eq!(h.summary(&g, false), "");
        g[1].available = false;
        assert_eq!(h.latest(&g), Some(0));
    }
    #[test]
    fn library_two_actions_and_empty_focus() {
        for n in 0..4 {
            for resume in [false, true] {
                assert_eq!(focus(0, true, resume, n, true), 2);
                assert_eq!(focus(2, true, true, n, false), 2);
            }
        }
    }
    #[test]
    fn rapid_changes_stay_reachable() {
        let mut s = 0;
        for _ in 0..100 {
            s = focus(s, true, false, 3, false);
            s = focus(s, false, false, 3, false);
            assert_eq!(s, 0);
        }
    }
}

/// Main panel above two secondary controls; no wrap, no skipped focus targets.
pub fn spatial_focus(
    current: usize,
    direction: crate::input::Direction,
    resume: bool,
    count: usize,
    has_recent: bool,
) -> usize {
    use crate::input::Direction::*;
    let hero = if resume { has_recent } else { count > 0 };
    let secondary = if resume {
        Some(3)
    } else {
        (count > 1).then_some(1)
    };
    let original = current;
    let current = clamp_focus(current, resume, count, has_recent);
    match (current, direction) {
        (0, Down) => 2,
        // Empty Resume still has two useful destinations: permit a vertical route.
        (2, Down) if resume && !hero && original == 2 => 3,
        (2, Down) if !resume && original == 2 => 3,
        (1, Down) if !resume && count > 1 => 3,
        (1, Right) if !resume && count > 2 => 4,
        (4, Left) if !resume && count > 2 => 1,
        (4, Up) if !resume && count > 2 => 0,
        (4, Down) if !resume && count > 2 => 3,
        (3, Up) if !resume => 2,
        (3, Up) if resume && !hero => 2,
        (2, Right) => secondary.unwrap_or(2),
        (i, Left) if secondary == Some(i) => 2,
        (2, Up) if hero => 0,
        (i, Up) if secondary == Some(i) && hero => 0,
        _ => current,
    }
}
#[cfg(test)]
mod visual_navigation_tests {
    use super::*;
    use crate::input::Direction::*;
    #[test]
    fn discovery_control_is_reachable_from_resume() {
        assert_eq!(spatial_focus(0, Down, true, 6, true), 2);
        assert_eq!(spatial_focus(2, Right, true, 6, true), 3);
        assert_eq!(spatial_focus(3, Up, true, 6, true), 0);
        assert_eq!(spatial_focus(3, Left, true, 6, true), 2);
    }
    #[test]
    fn empty_resume_can_reach_both_destinations() {
        assert_eq!(spatial_focus(0, Down, true, 6, false), 2);
        assert_eq!(spatial_focus(2, Right, true, 6, false), 3);
        assert_eq!(spatial_focus(3, Left, true, 6, false), 2);
    }
    #[test]
    fn empty_resume_has_a_vertical_route_between_available_actions() {
        for count in [0, 1, 10000] {
            assert_eq!(spatial_focus(2, Down, true, count, false), 3);
            assert_eq!(spatial_focus(3, Up, true, count, false), 2);
            assert_eq!(spatial_focus(2, Right, true, count, false), 3);
            assert_eq!(spatial_focus(3, Left, true, count, false), 2);
        }
    }
    #[test]
    fn empty_emotional_skips_absent_alternative() {
        for d in [Up, Left, Right] {
            assert_eq!(spatial_focus(2, d, false, 0, false), 2);
        }
        // The dock is now a real destination in both sections, even when empty.
        assert_eq!(spatial_focus(2, Down, false, 0, false), 3);
        assert_eq!(spatial_focus(3, Up, false, 0, false), 2);
    }
    #[test]
    fn every_catalog_and_view_keeps_focus_on_existing_controls() {
        for count in 0..4 {
            for resume in [true, false] {
                for recent in [true, false] {
                    for i in 0..7 {
                        for d in [Up, Down, Left, Right] {
                            let f = spatial_focus(i, d, resume, count, recent);
                            assert!(
                                f == 2
                                    || (f == 0 && if resume { recent } else { count > 0 })
                                    || f == 3
                                    || (f == 1 && !resume && count > 1)
                                    || (f == 4 && !resume && count > 2)
                            );
                        }
                    }
                }
            }
        }
    }
}

/// Clamp only missing controls; environmental updates must not move a valid focus.
pub fn clamp_focus(current: usize, resume: bool, count: usize, has_recent: bool) -> usize {
    if current == 2
        || (current == 0 && if resume { has_recent } else { count > 0 })
        || current == 3
        || (current == 1 && !resume && count > 1)
        || (current == 4 && !resume && count > 2)
    {
        current
    } else {
        2
    }
}
#[cfg(test)]
mod focus_retention_tests {
    use super::*;
    #[test]
    fn environmental_change_keeps_discovery_focus_even_when_empty() {
        for n in 0..4 {
            assert_eq!(clamp_focus(3, true, n, false), 3);
            assert_eq!(clamp_focus(2, false, n, false), 2);
        }
    }
    #[test]
    fn vanished_alternative_gets_a_real_destination() {
        assert_eq!(clamp_focus(1, false, 1, false), 2);
        assert_eq!(clamp_focus(0, true, 5, false), 2);
    }
    #[test]
    fn discovery_dock_is_reachable_and_retained_for_every_catalog_size() {
        use crate::input::Direction::*;
        for count in [0, 1, 2, 10000] {
            assert_eq!(spatial_focus(2, Down, false, count, false), 3);
            assert_eq!(spatial_focus(3, Up, false, count, false), 2);
            assert_eq!(clamp_focus(3, false, count, false), 3);
            if count > 1 {
                assert_eq!(spatial_focus(1, Down, false, count, false), 3);
            }
        }
    }
}

pub const PAN_PIXELS: f32 = 3.0;
pub const PAN_DURATION_MS: u64 = 240;
pub fn panorama(resume: bool, reduced: bool, ambient_motion: bool) -> f32 {
    if reduced || !ambient_motion {
        0.0
    } else if resume {
        -PAN_PIXELS
    } else {
        PAN_PIXELS
    }
}

#[cfg(test)]
mod panorama_tests {
    use super::*;
    #[test]
    fn reduced_motion_never_moves() {
        for resume in [true, false] {
            assert_eq!(panorama(resume, true, true), 0.0);
            assert_eq!(panorama(resume, false, false), 0.0);
            assert_eq!(panorama(resume, false, true).abs(), 3.0);
        }
    }
}
