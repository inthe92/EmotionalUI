//! Presentation-only separation of discovery from the continuity protagonist.
//!
//! The recommender's scores and persisted order stay untouched. Callers use the
//! returned order consistently for labels, artwork and game actions.

use std::collections::HashSet;

/// Keep valid candidates in their ranked order, making room for a different
/// protagonist when continuity already occupies the first place. A collection
/// with one available candidate remains useful rather than becoming empty.
pub(super) fn presentation_order(
    ranked: &[usize],
    recent: Option<usize>,
    available: &[bool],
) -> Vec<usize> {
    let mut seen = HashSet::with_capacity(ranked.len());
    let mut order: Vec<_> = ranked
        .iter()
        .copied()
        .filter(|index| available.get(*index) == Some(&true) && seen.insert(*index))
        .collect();
    if order.len() > 1 && order.first().copied() == recent {
        order.swap(0, 1);
    }
    order
}

#[cfg(test)]
mod tests {
    use super::presentation_order;

    #[test]
    fn separates_protagonists_without_rewriting_the_base_order() {
        let base = [2, 0, 1, 3];
        assert_eq!(presentation_order(&base, Some(2), &[true; 4]), [0, 2, 1, 3]);
        assert_eq!(base, [2, 0, 1, 3]);
    }

    #[test]
    fn already_distinct_and_history_free_orders_stay_ranked() {
        assert_eq!(
            presentation_order(&[2, 0, 1], Some(0), &[true; 3]),
            [2, 0, 1]
        );
        assert_eq!(presentation_order(&[2, 0, 1], None, &[true; 3]), [2, 0, 1]);
    }

    #[test]
    fn one_available_game_can_belong_to_both_sections() {
        assert_eq!(presentation_order(&[0], Some(0), &[true]), [0]);
        assert_eq!(presentation_order(&[0, 1], Some(0), &[true, false]), [0]);
    }

    #[test]
    fn unavailable_invalid_and_duplicate_candidates_are_never_presented() {
        assert_eq!(
            presentation_order(&[8, 0, 1, 1, 2, 0], Some(1), &[false, true, true]),
            [2, 1]
        );
        assert!(presentation_order(&[], None, &[]).is_empty());
        assert!(presentation_order(&[0, 1], Some(0), &[false; 2]).is_empty());
    }

    #[test]
    fn explicit_ranked_candidates_are_the_only_source_of_discovery() {
        // Availability alone never adds a game excluded by the recommender.
        assert_eq!(presentation_order(&[0], Some(0), &[true; 10_000]), [0]);
        let order: Vec<_> = (0..10_000).collect();
        let result = presentation_order(&order, Some(0), &[true; 10_000]);
        assert_eq!(&result[..4], [1, 0, 2, 3]);
        assert_eq!(result.len(), order.len());
        assert_eq!(&result[2..], &order[2..]);
    }
}
