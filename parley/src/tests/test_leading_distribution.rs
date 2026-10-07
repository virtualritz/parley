// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for the line-height boxes of [`LeadingDistribution`], which are private. The tests of
//! the public API are in `parley_tests`.

use parley_engine::FontMetrics;

use crate::LeadingDistribution;
use crate::layout::style_metrics::BoxMetrics;

const EPSILON: f32 = 1e-4;

/// Font metrics with the given extents and line gap.
fn font_metrics(ascent: f32, descent: f32, leading: f32) -> FontMetrics {
    FontMetrics {
        ascent,
        descent,
        leading,
        ..FontMetrics::fallback(0.)
    }
}

#[test]
fn half_leading_box_is_centred_on_the_content() {
    let font = font_metrics(15., 5., 0.);
    let half_leading = BoxMetrics::from_font(&font, 30., LeadingDistribution::HalfLeading, false);
    assert_eq!((half_leading.over, half_leading.under), (20., 10.));
}

#[test]
fn proportional_box_keeps_the_ascent_to_descent_ratio() {
    let font = font_metrics(15., 5., 0.);
    let proportional = BoxMetrics::from_font(&font, 30., LeadingDistribution::Proportional, false);
    assert_eq!((proportional.ascent, proportional.descent), (15., 5.));
    assert_eq!((proportional.over, proportional.under), (22.5, 7.5));

    // A tight line height scales the box down in the same ratio.
    let tight = BoxMetrics::from_font(&font, 10., LeadingDistribution::Proportional, false);
    assert_eq!((tight.over, tight.under), (7.5, 2.5));
}

/// Like Skia, half of the line gap is added to the ascent and half to the descent before they
/// are scaled, so a `normal` line height puts half of the gap on each side.
#[test]
fn proportional_box_splits_the_line_gap_evenly_first() {
    let font = font_metrics(14., 4., 2.);
    let proportional = BoxMetrics::from_font(&font, 40., LeadingDistribution::Proportional, false);
    assert!((proportional.over - 40. * 15. / 20.).abs() < EPSILON);
    assert!((proportional.under - 40. * 5. / 20.).abs() < EPSILON);

    let normal = BoxMetrics::from_font(&font, 20., LeadingDistribution::Proportional, false);
    assert!((normal.over - 15.).abs() < EPSILON);
    assert!((normal.under - 5.).abs() < EPSILON);
}

/// When quantizing, the leading above the baseline is floored as for
/// [`LeadingDistribution::HalfLeading`], and the ratio comes from the unrounded metrics.
#[test]
fn proportional_box_quantized() {
    let font = font_metrics(14.6, 5.4, 0.);
    let quantized = BoxMetrics::from_font(&font, 31., LeadingDistribution::Proportional, true);
    // Unrounded: 31 * 14.6 / 20 = 22.63 above the baseline.
    assert_eq!((quantized.ascent, quantized.descent), (15., 5.));
    assert_eq!((quantized.over, quantized.under), (22., 9.));
}

/// A font without extents falls back to half-leading.
#[test]
fn proportional_box_without_extents() {
    let font = font_metrics(0., 0., 0.);
    let proportional = BoxMetrics::from_font(&font, 10., LeadingDistribution::Proportional, false);
    assert_eq!((proportional.over, proportional.under), (5., 5.));
}
