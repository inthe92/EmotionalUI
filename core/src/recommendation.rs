use crate::{library::Game, mood::Mood};
use std::collections::HashSet;
#[derive(Clone, Copy)]
pub struct Context {
    pub mood: Mood,
    pub minutes: u16,
    pub raining: bool,
    pub slot: u64,
    pub refresh: u64,
}
/// Integer scores, deterministic tie breaking. Context never excludes a genre.
pub fn rank(games: &[Game], ctx: Context) -> Vec<usize> {
    rank_editorial(games, ctx, &crate::editorial::Signals::default())
}
pub fn rank_editorial(
    games: &[Game],
    ctx: Context,
    signals: &crate::editorial::Signals<'_>,
) -> Vec<usize> {
    rank_policy(games, ctx, signals, true)
}
pub fn rank_policy(
    games: &[Game],
    ctx: Context,
    signals: &crate::editorial::Signals<'_>,
    personalized: bool,
) -> Vec<usize> {
    rank_affinity(
        games,
        ctx,
        signals,
        personalized,
        &std::collections::BTreeMap::new(),
    )
}
pub fn rank_affinity(
    games: &[Game],
    ctx: Context,
    signals: &crate::editorial::Signals<'_>,
    personalized: bool,
    affinity: &std::collections::BTreeMap<String, i8>,
) -> Vec<usize> {
    let mut seen = HashSet::new();
    let mut ranked: Vec<(usize, i64)> = games
        .iter()
        .enumerate()
        .filter(|(_, g)| g.available && !g.id.trim().is_empty() && seen.insert(g.id.as_str()))
        .map(|(i, g)| {
            let fit = if g.editorial.session_minutes.unwrap_or(g.session_minutes) <= ctx.minutes {
                18
            } else {
                -15
            };
            let ambient = if ctx.raining {
                i64::from(g.emotions.calm) / 20
            } else {
                0
            };
            let quiet = if matches!(ctx.mood, Mood::Shelter | Mood::Silence) {
                i64::from(g.emotions.immersion) / 25
            } else {
                i64::from(g.emotions.joy) / 25
            };
            let seed = g.id.bytes().fold(
                ctx.slot.wrapping_add(ctx.refresh.wrapping_mul(7919)),
                |a, b| a.wrapping_mul(31).wrapping_add(u64::from(b)),
            );
            let variety = (seed % 29) as i64;
            let score = if personalized {
                i64::from(affinity.get(&g.id).copied().unwrap_or(0).clamp(-1, 1)) * 100
            } else {
                0
            } + i64::from(g.started) * 65
                + if g.started {
                    g.editorial.resume_ease.map_or(0, |v| i64::from(v) / 25)
                } else {
                    0
                }
                + (crate::editorial::bonus(&g.editorial, signals, ctx.raining)
                    + i64::from(
                        *g.editorial
                            .extra
                            .get(match ctx.mood {
                                Mood::Dawn => "morning",
                                Mood::Horizon => "afternoon",
                                Mood::Shelter => "night",
                                Mood::Silence => "late",
                            })
                            .unwrap_or(&0),
                    ) / 25
                    + if ctx.raining {
                        i64::from(*g.editorial.extra.get("rain").unwrap_or(&0)) / 25
                    } else {
                        0
                    })
                .min(24)
                + i64::from(g.favorite) * 18
                + if personalized {
                    i64::from(g.choices.min(10)) * 5
                } else {
                    0
                }
                - i64::from(g.recent_recommendations.min(10)) * 16
                + if personalized {
                    i64::from(g.choices_by_mood[ctx.mood.index() as usize].min(10)) * 5
                } else {
                    0
                }
                + if ctx.minutes <= 15 {
                    // Authored suitability, not an invented duration or abandonment signal.
                    i64::from(*g.editorial.extra.get("short_sessions").unwrap_or(&0)) / 10
                } else {
                    0
                }
                + fit
                + ambient
                + quiet
                + variety;
            (i, score)
        })
        .collect();
    ranked.sort_by(|(ia, sa), (ib, sb)| sb.cmp(sa).then_with(|| games[*ia].id.cmp(&games[*ib].id)));
    // A near-tie may favor another genre in the two secondary places.
    // Never displace the principal recommendation merely to force diversity.
    for pos in 1..ranked.len().min(3) {
        let chosen: HashSet<&str> = ranked[..pos]
            .iter()
            .map(|(i, _)| games[*i].genre.as_str())
            .collect();
        let score = ranked[pos].1;
        if let Some(offset) = ranked[pos..]
            .iter()
            .position(|(i, s)| score - *s <= 12 && !chosen.contains(games[*i].genre.as_str()))
        {
            let candidate = ranked.remove(pos + offset);
            ranked.insert(pos, candidate);
        }
    }
    ranked.into_iter().map(|(i, _)| i).collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::demo_catalog;
    fn ctx() -> Context {
        Context {
            mood: Mood::Shelter,
            minutes: 30,
            raining: false,
            slot: 42,
            refresh: 0,
        }
    }
    #[test]
    fn authored_brief_suitability_is_used_without_inventing_duration() {
        let mut g = demo_catalog();
        for game in &mut g {
            game.started = false;
            game.editorial = Default::default();
        }
        let c = Context {
            minutes: 10,
            ..ctx()
        };
        let before = rank(&g, c);
        let candidate = *before.last().unwrap();
        g[candidate]
            .editorial
            .extra
            .insert("short_sessions".into(), 100);
        let after = rank(&g, c);
        assert!(
            after.iter().position(|i| *i == candidate)
                < before.iter().position(|i| *i == candidate)
        );
        assert_eq!(g[candidate].editorial.session_minutes, None);
        assert!(rank(&g, Context { minutes: 30, ..c }).contains(&candidate));
    }
    #[test]
    fn only_available() {
        let mut g = demo_catalog();
        g[0].available = false;
        assert!(!rank(&g, ctx()).contains(&0));
    }
    #[test]
    fn resume_priority() {
        assert_eq!(rank(&demo_catalog(), ctx())[0], 0);
    }
    #[test]
    fn stable_and_unique() {
        let g = demo_catalog();
        let a = rank(&g, ctx());
        assert_eq!(a, rank(&g, ctx()));
        assert_eq!(a.len(), g.len());
        let mut b = a.clone();
        b.sort();
        b.dedup();
        assert_eq!(a.len(), b.len());
    }
    #[test]
    fn repetition_penalty() {
        let mut g = demo_catalog();
        g[0].recent_recommendations = 10;
        assert_ne!(rank(&g, ctx())[0], 0);
    }
    #[test]
    fn empty_is_valid() {
        assert!(rank(&[], ctx()).is_empty());
    }
    #[test]
    fn choices_teach() {
        // Equally suitable titles: preference must outweigh the bounded variety term.
        let mut a = demo_catalog()[1].clone();
        a.id = "alpha".into();
        let mut b = a.clone();
        b.id = "beta".into();
        let mut games = vec![a, b];
        let before = rank(&games, ctx());
        let last = before[1];
        games[last].choices = 10;
        assert_eq!(rank(&games, ctx())[0], last);
    }
    #[test]
    fn context_does_not_exclude_action() {
        let g = demo_catalog();
        assert!(rank(&g, ctx()).contains(&3));
    }
    #[test]
    fn refresh_changes_order_eventually() {
        let g = demo_catalog();
        let a = rank(&g, ctx());
        assert!((1..30).any(|r| rank(
            &g,
            Context {
                refresh: r,
                ..ctx()
            }
        ) != a));
    }
}

/// Restore only known, unique, still-available IDs; append newly discovered titles.
pub fn restore(games: &[Game], saved: &[String], context: Context) -> Vec<usize> {
    restore_ranked(games, saved, rank(games, context))
}
pub fn restore_ranked(games: &[Game], saved: &[String], ranked: Vec<usize>) -> Vec<usize> {
    let by_id = ranked
        .iter()
        .map(|i| (games[*i].id.as_str(), *i))
        .collect::<std::collections::HashMap<_, _>>();
    let mut seen = HashSet::new();
    saved
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .chain(ranked.iter().copied())
        .filter(|i| seen.insert(*i))
        .collect()
}
/// Recency fades on an explicit request, so penalties cannot stay saturated forever.
pub fn renew(games: &mut [Game], shown: &[usize]) {
    for g in games.iter_mut() {
        g.recent_recommendations = g.recent_recommendations.saturating_sub(1);
    }
    for i in shown.iter().take(3) {
        if let Some(g) = games.get_mut(*i) {
            g.recent_recommendations = g.recent_recommendations.saturating_add(3).min(10);
        }
    }
}
#[cfg(test)]
mod edge_tests {
    use super::*;
    use crate::library::demo_catalog;
    fn context() -> Context {
        Context {
            mood: Mood::Shelter,
            minutes: 30,
            raining: false,
            slot: u64::MAX,
            refresh: u64::MAX,
        }
    }
    #[test]
    fn small_and_unavailable_catalogs() {
        let mut g = demo_catalog();
        assert_eq!(rank(&g[..1], context()), [0]);
        assert_eq!(rank(&g[..2], context()).len(), 2);
        for x in &mut g {
            x.available = false;
        }
        assert!(rank(&g, context()).is_empty());
    }
    #[test]
    fn invalid_and_duplicate_ids_are_excluded() {
        let mut g = demo_catalog();
        g[1].id = g[0].id.clone();
        g[2].id.clear();
        assert_eq!(rank(&g, context()).len(), 4);
    }
    #[test]
    fn stale_saved_order_is_repaired() {
        let mut g = demo_catalog();
        g[1].available = false;
        let r = restore(
            &g,
            &[
                "missing".into(),
                "demo-0".into(),
                "demo-0".into(),
                "demo-1".into(),
            ],
            context(),
        );
        assert_eq!(r.len(), 5);
        assert_eq!(r[0], 0);
        assert!(!r.contains(&1));
    }
    #[test]
    fn extreme_history_cannot_overflow() {
        let mut g = demo_catalog();
        for x in &mut g {
            x.choices = u32::MAX;
            x.recent_recommendations = u32::MAX;
            x.choices_by_mood = [u16::MAX; 4];
        }
        assert_eq!(rank(&g, context()).len(), 6);
    }
    #[test]
    fn habits_only_add_a_bounded_preference() {
        let mut g = demo_catalog();
        g[1].choices_by_mood[2] = 5;
        let night = rank(&g, context());
        g[1].choices_by_mood = [0; 4];
        let neutral = rank(&g, context());
        assert!(night.iter().position(|i| *i == 1) <= neutral.iter().position(|i| *i == 1));
    }
    #[test]
    fn long_sessions_are_still_candidates() {
        let mut g = demo_catalog();
        g[0].session_minutes = u16::MAX;
        assert!(rank(&g, context()).contains(&0));
    }
    #[test]
    fn recency_recovers() {
        let mut g = demo_catalog();
        g[0].recent_recommendations = 10;
        renew(&mut g, &[1]);
        assert_eq!(g[0].recent_recommendations, 9);
        assert_eq!(g[1].recent_recommendations, 3);
    }
    #[test]
    fn every_context_preserves_availability() {
        let g = demo_catalog();
        for mood in [Mood::Dawn, Mood::Horizon, Mood::Shelter, Mood::Silence] {
            for raining in [false, true] {
                assert_eq!(
                    rank(
                        &g,
                        Context {
                            mood,
                            raining,
                            ..context()
                        }
                    )
                    .len(),
                    6
                );
            }
        }
    }
}

#[cfg(test)]
mod phase7_tests {
    use super::*;
    use crate::{
        editorial::{Profile, Signals},
        library::demo_catalog,
    };
    fn ctx() -> Context {
        Context {
            mood: Mood::Shelter,
            minutes: 30,
            raining: true,
            slot: 22,
            refresh: 0,
        }
    }
    #[test]
    fn learned_night_action_outweighs_ambient_guess() {
        let mut games = vec![demo_catalog()[1].clone(), demo_catalog()[1].clone()];
        games[0].id = "quiet".into();
        games[1].id = "action".into();
        games[1].emotions.intensity = 100;
        games[1].choices = 10;
        games[1].choices_by_mood[2] = 10;
        games[0].editorial = Profile {
            calm: Some(100),
            mystery: Some(100),
            ..Default::default()
        };
        assert_eq!(rank_policy(&games, ctx(), &Signals::default(), true)[0], 1);
        assert_eq!(
            rank_policy(&games, ctx(), &Signals::default(), false).len(),
            2
        );
    }
    #[test]
    fn disabled_personalization_ignores_choices() {
        let mut games = demo_catalog();
        let before = rank_policy(&games, ctx(), &Signals::default(), false);
        for g in &mut games {
            g.choices = u32::MAX;
            g.choices_by_mood = [u16::MAX; 4];
        }
        assert_eq!(
            before,
            rank_policy(&games, ctx(), &Signals::default(), false)
        );
    }
    #[test]
    fn editorial_tail_retains_context_when_restoring() {
        let games = demo_catalog();
        let r = rank_policy(&games, ctx(), &Signals::default(), true);
        let out = restore_ranked(
            &games,
            &[games[r[1]].id.clone(), "missing".into()],
            r.clone(),
        );
        assert_eq!(out[0], r[1]);
        assert_eq!(out.len(), r.len());
        assert_eq!(out[1], r[0]);
    }
    #[test]
    fn seasonal_bonus_applies_only_to_authored_metadata() {
        let date = chrono::NaiveDate::from_ymd_opt(2026, 10, 31).unwrap();
        let events = vec!["halloween"];
        let s = Signals {
            date: Some(date),
            events,
            ..Default::default()
        };
        assert_eq!(crate::editorial::bonus(&Profile::default(), &s, true), 0);
        let mut p = Profile::default();
        p.celebrations.insert("halloween".into(), 100);
        assert!(crate::editorial::bonus(&p, &s, true) > 0);
    }
}
