//! Calls the exact pinned Slint fit routine; no GPU performance claim.
use slint::private_unstable_api::re_exports::{euclid, fit9slice, ImageTiling};

#[test]
fn default_nine_slice_has_no_temporary_tiling_branch_at_product_sizes() {
    assert_eq!(ImageTiling::default(), ImageTiling::None);
    for (source_w, source_h, radius, targets) in [
        (64, 64, 16, vec![(448., 192.), (400., 208.), (400., 216.)]),
        (64, 64, 12, vec![(216., 56.), (216., 80.), (248., 48.)]),
    ] {
        for (width, height) in targets {
            let fits: Vec<_> = fit9slice(
                euclid::size2(source_w, source_h),
                [radius; 4],
                euclid::size2(width, height),
                euclid::Scale::new(1.),
                Default::default(),
                (ImageTiling::None, ImageTiling::None),
            )
            .collect();
            assert_eq!(fits.len(), 9);
            assert!(fits.iter().all(|f| f.tiled.is_none()));
        }
    }
}
#[test]
fn repeated_nine_slice_is_a_positive_control_for_the_suspected_branch() {
    let original = euclid::size2(64, 64);
    let fits: Vec<_> = fit9slice(
        original,
        [16; 4],
        euclid::size2(448., 192.),
        euclid::Scale::new(1.),
        Default::default(),
        (ImageTiling::Repeat, ImageTiling::Repeat),
    )
    .collect();
    assert!(fits
        .iter()
        .any(|f| f.tiled.is_some() && f.clip_rect.size.cast() != original));
}
