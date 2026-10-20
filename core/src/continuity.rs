//! Conservative, pure personal continuity. No popularity or save-state inference.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct Preference {
    pub pinned: bool,
    pub removed: bool,
    pub completed: bool,
    pub affinity: i8,
}
#[derive(Clone, Debug)]
pub struct Evidence {
    pub game_id: String,
    pub ordinal: u64,
    pub utc: Option<i64>,
    pub seconds: Option<u32>,
    pub confirmed: bool,
    pub unexpected: bool,
    pub voluntary_return: Option<bool>,
    pub saved: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Weights {
    pub session: u16,
    pub duration: u16,
    pub return_visit: u16,
    pub recency: u16,
    pub saved: u16,
    pub explicit: u16,
}
impl Default for Weights {
    fn default() -> Self {
        Self {
            session: 12,
            duration: 24,
            return_visit: 12,
            recency: 18,
            saved: 14,
            explicit: 30,
        }
    }
}
impl Weights {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.session,
            self.duration,
            self.return_visit,
            self.recency,
            self.saved,
            self.explicit,
        ]
        .iter()
        .any(|v| *v > 1000)
        {
            return Err("Continuity weights must be 0..1000".into());
        }
        Ok(())
    }
}
#[derive(Debug, PartialEq, Eq)]
pub struct Candidate {
    pub index: usize,
    pub score: u64,
    pub reasons: Vec<&'static str>,
}
pub fn rank(
    games: &[crate::library::Game],
    activity: &[Evidence],
    preferences: &BTreeMap<String, Preference>,
    now: Option<i64>,
    weights: &Weights,
) -> Vec<Candidate> {
    let valid_time = |t: i64| (1577836800..=4133980800).contains(&t);
    let now = now.filter(|t| valid_time(*t));
    let mut grouped: HashMap<&str, Vec<&Evidence>> = HashMap::new();
    let newest = activity.iter().map(|e| e.ordinal).max().unwrap_or(0);
    for e in activity {
        if e.confirmed && !e.unexpected {
            grouped.entry(&e.game_id).or_default().push(e);
        }
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for (index, g) in games
        .iter()
        .enumerate()
        .filter(|(_, g)| g.available && !g.id.is_empty() && seen.insert(g.id.as_str()))
    {
        let p = preferences.get(&g.id).cloned().unwrap_or_default();
        if p.removed || p.completed {
            continue;
        }
        let sessions = grouped.get(g.id.as_str()).cloned().unwrap_or_default();
        if sessions.is_empty() && !p.pinned {
            continue;
        }
        let mut reasons = Vec::new();
        let count = sessions.len().min(8) as u64;
        let mut score = count * u64::from(weights.session);
        if count > 1 {
            reasons.push("Has vuelto a este juego");
        }
        let target = u64::from(
            g.editorial
                .session_minutes
                .unwrap_or(g.session_minutes)
                .clamp(5, 60),
        ) * 60;
        let duration = sessions
            .iter()
            .filter_map(|e| e.seconds)
            .map(u64::from)
            .sum::<u64>()
            .min(target * 3);
        score += duration * u64::from(weights.duration) / target;
        if duration > 0 {
            reasons.push("Tiempo de sesión observado");
        }
        let voluntary = sessions
            .iter()
            .filter(|e| e.voluntary_return == Some(true))
            .count()
            .min(5) as u64;
        score += voluntary * u64::from(weights.return_visit);
        if voluntary > 0 {
            reasons.push("Regreso voluntario confirmado");
        }
        if sessions.iter().any(|e| e.saved == Some(true)) {
            score += u64::from(weights.saved);
            reasons.push("Guardado confirmado");
        }
        let last = sessions.iter().map(|e| e.ordinal).max().unwrap_or(0);
        let age_days = now.and_then(|n| {
            sessions
                .iter()
                .filter_map(|e| e.utc)
                .filter(|t| valid_time(*t) && *t <= n)
                .max()
                .map(|t| (n - t) as u64 / 86400)
        });
        let age = age_days
            .unwrap_or_else(|| newest.saturating_sub(last))
            .min(90);
        score += u64::from(weights.recency) * 7 / (age + 7);
        score = score.saturating_add_signed(
            i64::from(p.affinity.clamp(-1, 1)) * i64::from(weights.explicit),
        );
        if p.pinned {
            reasons.push("Fijado por ti");
        }
        out.push((
            p.pinned,
            last,
            Candidate {
                index,
                score,
                reasons,
            },
        ));
    }
    out.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| b.2.score.cmp(&a.2.score))
            .then_with(|| b.1.cmp(&a.1))
            .then_with(|| games[a.2.index].id.cmp(&games[b.2.index].id))
    });
    out.into_iter().take(3).map(|v| v.2).collect()
}
/// Extra slots 4/5 are compact continuity entries. Old one-game navigation unchanged.
pub fn focus(
    current: usize,
    d: crate::input::Direction,
    resume: bool,
    count: usize,
    has: bool,
    n: usize,
) -> usize {
    use crate::input::Direction::*;
    if !resume || n < 2 {
        return crate::product::spatial_focus(current, d, resume, count, has);
    }
    match (current, d) {
        (0, Down) => 4,
        (4, Right) if n > 2 => 5,
        (5, Left) => 4,
        (4 | 5, Up) => 0,
        (4, Down) => 2,
        (5, Down) => 3,
        (2, Up) => 4,
        (3, Up) => {
            if n > 2 {
                5
            } else {
                4
            }
        }
        (2, Right) => 3,
        (3, Left) => 2,
        _ => clamp(current, resume, count, has, n),
    }
}
pub fn clamp(current: usize, resume: bool, count: usize, has: bool, n: usize) -> usize {
    if resume && ((current == 4 && n > 1) || (current == 5 && n > 2)) {
        current
    } else {
        crate::product::clamp_focus(current, resume, count, has)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn e(id: usize, seconds: Option<u32>, ordinal: u64) -> Evidence {
        Evidence {
            game_id: format!("demo-{id}"),
            ordinal,
            utc: None,
            seconds,
            confirmed: true,
            unexpected: false,
            voluntary_return: None,
            saved: None,
        }
    }
    fn ranks(a: &[Evidence], p: &BTreeMap<String, Preference>) -> Vec<usize> {
        rank(
            &crate::library::demo_catalog(),
            a,
            p,
            None,
            &Weights::default(),
        )
        .iter()
        .map(|c| c.index)
        .collect()
    }
    #[test]
    fn long_and_short_sessions_are_positive() {
        assert_eq!(
            ranks(&[e(0, Some(3600), 1), e(1, Some(300), 2)], &BTreeMap::new()),
            [0, 1]
        );
        assert_eq!(ranks(&[e(1, Some(60), 1)], &BTreeMap::new()), [1]);
    }
    #[test]
    fn repeated_arcade_short_sessions_can_lead() {
        let mut a = (1..=5).map(|n| e(3, Some(300), n)).collect::<Vec<_>>();
        a.push(e(0, Some(1800), 6));
        assert_eq!(ranks(&a, &BTreeMap::new())[0], 3);
    }
    #[test]
    fn unknown_save_never_becomes_confirmed() {
        let a = e(0, None, 1);
        let mut b = e(1, None, 1);
        b.saved = Some(true);
        assert_eq!(ranks(&[a.clone(), b.clone()], &BTreeMap::new())[0], 1);
        b.saved = None;
        assert_eq!(ranks(&[a, b], &BTreeMap::new())[0], 0);
    }
    #[test]
    fn explicit_control_overrides_inference() {
        let mut p = BTreeMap::new();
        p.insert(
            "demo-4".into(),
            Preference {
                pinned: true,
                ..Default::default()
            },
        );
        assert_eq!(ranks(&[e(0, Some(3600), 9)], &p)[0], 4);
        p.get_mut("demo-4").unwrap().removed = true;
        assert_eq!(ranks(&[e(0, None, 1)], &p), [0]);
        p.get_mut("demo-4").unwrap().removed = false;
        p.get_mut("demo-4").unwrap().completed = true;
        assert!(ranks(&[], &p).is_empty());
    }
    #[test]
    fn unexpected_and_empty_do_not_invent_interest() {
        let mut a = e(0, Some(1000), 1);
        a.unexpected = true;
        assert!(ranks(&[a], &BTreeMap::new()).is_empty());
        assert!(ranks(&[], &BTreeMap::new()).is_empty());
    }
    #[test]
    fn unknown_invalid_and_reordered_clocks_are_safe() {
        let g = crate::library::demo_catalog();
        let mut a = vec![e(0, None, 1), e(1, None, 2)];
        a[0].utc = Some(i64::MAX);
        let r = rank(&g, &a, &BTreeMap::new(), Some(-1), &Weights::default());
        a.reverse();
        assert_eq!(r, rank(&g, &a, &BTreeMap::new(), None, &Weights::default()));
    }
    #[test]
    fn only_three_available_unique_games() {
        let a = (0..6).map(|i| e(i, None, i as u64)).collect::<Vec<_>>();
        assert_eq!(ranks(&a, &BTreeMap::new()).len(), 3);
        let mut g = crate::library::demo_catalog();
        g[5].available = false;
        assert!(rank(&g, &a, &BTreeMap::new(), None, &Weights::default())
            .iter()
            .all(|c| c.index != 5));
    }
    #[test]
    fn returning_old_game_remains_eligible() {
        let mut a = vec![
            e(0, Some(300), 1),
            e(0, Some(300), 90),
            e(1, Some(300), 100),
        ];
        a[0].utc = Some(1600000000);
        assert!(ranks(&a, &BTreeMap::new()).contains(&0));
    }
    #[test]
    fn weights_and_focus_are_bounded() {
        let w = Weights {
            saved: 1001,
            ..Default::default()
        };
        assert!(w.validate().is_err());
        use crate::input::Direction::*;
        assert_eq!(focus(0, Down, true, 6, true, 3), 4);
        assert_eq!(focus(4, Right, true, 6, true, 3), 5);
        assert_eq!(focus(5, Down, true, 6, true, 3), 3);
        assert_eq!(clamp(5, true, 6, true, 1), 2);
    }
    #[test]
    fn cards_keep_their_column_when_reaching_the_footer() {
        use crate::input::Direction::*;
        for (top, bottom) in [(4, 2), (5, 3)] {
            assert_eq!(focus(top, Down, true, 6, true, 3), bottom);
            assert_eq!(focus(bottom, Up, true, 6, true, 3), top);
        }
        assert_eq!(focus(3, Up, true, 6, true, 2), 4);
    }
}
