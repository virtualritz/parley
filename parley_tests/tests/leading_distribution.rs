// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`LeadingDistribution`].

use parley::{FontFamily, LayoutContext, LeadingDistribution, LineHeight, StyleProperty};

use crate::util::ColorBrush;
use crate::util::env::{FONT_FAMILY_LIST, create_font_context};
use crate::util::line_boxes::{
    EPSILON, FontExtents, Options, Span, assert_glyphs_on_the_baseline, baseline_from_top, layout,
    run_fonts,
};

/// font sizes, and the baseline sits that line height's share of the ascent below the top.
#[test]
fn leading_distribution_proportional_line_is_the_largest_line_height_of_one_font() {
    // (first span, second span, line height)
    let cases = [
        ((21., 25.2), (13., 24.), 25.2),
        ((13., 15.6), (21., 25.2), 25.2),
        ((13., 30.), (21., 25.2), 30.),
    ];
    for ((size_a, height_a), (size_b, height_b), expected) in cases {
        let text = "first second";
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional),
            &[
                Span::new(0..6, "Roboto", size_a, height_a),
                Span::new(6..12, "Roboto", size_b, height_b),
            ],
            None,
        );
        let case = format!("{size_a}/{height_a} + {size_b}/{height_b}");
        assert_eq!(layout.len(), 1, "{case}");
        let line = layout.get(0).unwrap();
        let metrics = line.metrics();
        assert!(
            (metrics.line_height - expected).abs() < EPSILON,
            "{case}: line height {}",
            metrics.line_height
        );
        assert!((layout.height() - expected).abs() < EPSILON, "{case}");

        let font = run_fonts(&line)[0];
        let baseline = baseline_from_top(&line);
        assert!(
            (baseline - expected * font.above_fraction()).abs() < EPSILON,
            "{case}: baseline {baseline}"
        );
        assert!(baseline > 0. && baseline < metrics.line_height, "{case}");
        assert_glyphs_on_the_baseline(&line);
    }
}

/// further below the baseline than the larger font's box.
#[test]
fn leading_distribution_half_leading_line_is_the_union_of_centred_boxes() {
    let text = "first second";
    let spans = [
        Span::new(0..6, "Roboto", 21., 25.2),
        Span::new(6..12, "Roboto", 13., 24.),
    ];
    let half_leading = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::HalfLeading),
        &spans,
        None,
    );
    let line = half_leading.get(0).unwrap();
    let fonts = run_fonts(&line);
    let (large, small) = (fonts[0], fonts[1]);
    let over = large.ascent + (25.2 - (large.ascent + large.descent)) / 2.;
    let under = small.descent + (24. - (small.ascent + small.descent)) / 2.;
    assert!((line.metrics().line_height - (over + under)).abs() < EPSILON);
    assert!(line.metrics().line_height > 25.2 + 2.);
}

/// Each line is sized by the spans on it.
#[test]
fn leading_distribution_proportional_lines_keep_their_own_line_heights() {
    let text = "first line\nsecond line";
    let tall = 0..11;
    let short = 11..text.len();
    for (spans, expected) in [
        (
            [
                Span::new(tall.clone(), "Roboto", 21., 25.2),
                Span::new(short.clone(), "Roboto", 13., 15.6),
            ],
            [25.2, 15.6],
        ),
        (
            [
                Span::new(tall.clone(), "Roboto", 13., 15.6),
                Span::new(short.clone(), "Roboto", 21., 25.2),
            ],
            [15.6, 25.2],
        ),
    ] {
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional),
            &spans,
            None,
        );
        let heights: Vec<f32> = layout.lines().map(|l| l.metrics().line_height).collect();
        assert_eq!(heights.len(), 2);
        for (height, expected) in heights.iter().zip(expected) {
            assert!((height - expected).abs() < EPSILON, "{heights:?}");
        }
        assert!((layout.height() - 40.8).abs() < EPSILON);
        for line in layout.lines() {
            let font = run_fonts(&line)[0];
            let expected = line.metrics().line_height * font.above_fraction();
            assert!((baseline_from_top(&line) - expected).abs() < EPSILON);
            assert_glyphs_on_the_baseline(&line);
        }
    }
}

/// is a floor.
#[test]
fn leading_distribution_proportional_root_line_height_is_a_floor() {
    let text = "first";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(40.), LeadingDistribution::Proportional),
        &[Span::new(0..5, "Roboto", 13., 15.6)],
        None,
    );
    let line = layout.get(0).unwrap();
    assert!((line.metrics().line_height - 40.).abs() < EPSILON);
    let font = run_fonts(&line)[0];
    assert!((baseline_from_top(&line) - 40. * font.above_fraction()).abs() < EPSILON);
}

/// largest line height: one box reaches highest above the baseline, the other lowest below it.
#[test]
fn leading_distribution_proportional_line_of_mixed_fonts_can_exceed_the_largest_line_height() {
    // Roboto puts 1900/2400 of a line height above the baseline, Noto Kufi Arabic 1282/1897.
    let text = "abc مرحبا";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional),
        &[
            Span::new(0..4, "Roboto", 20., 30.),
            Span::new(4..text.len(), "Noto Kufi Arabic", 20., 30.),
        ],
        None,
    );
    let line = layout.get(0).unwrap();
    let fractions: Vec<f32> = run_fonts(&line)
        .iter()
        .map(FontExtents::above_fraction)
        .collect();
    let highest = fractions.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let lowest = fractions.iter().copied().fold(f32::INFINITY, f32::min);
    assert!(highest - lowest > 0.1, "{fractions:?}");
    let expected = 30. * highest + 30. * (1. - lowest);
    assert!((line.metrics().line_height - expected).abs() < EPSILON);
    // 23.75 + 9.73.
    assert!((line.metrics().line_height - 33.476).abs() < 1e-3);
    assert!((baseline_from_top(&line) - 30. * highest).abs() < EPSILON);
    assert_glyphs_on_the_baseline(&line);
}

/// the line gap half above and half below the content.
#[test]
fn leading_distribution_proportional_metrics_relative_line_height() {
    // Arimo has a line gap.
    let text = "normal";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional),
        &[Span::new(0..6, "Arimo", 20., 0.).with_line_height(LineHeight::MetricsRelative(1.5))],
        None,
    );
    let line = layout.get(0).unwrap();
    let font = run_fonts(&line)[0];
    assert!(font.leading > 0.);
    let line_height = 1.5 * (font.ascent + font.descent + font.leading);
    assert!((line.metrics().line_height - line_height).abs() < EPSILON);
    let baseline = 1.5 * (font.ascent + font.leading / 2.);
    assert!((baseline_from_top(&line) - baseline).abs() < EPSILON);
}

/// The distribution is a style property: a span can override the one it inherits.
#[test]
fn leading_distribution_is_per_style() {
    let text = "first second";
    let mut fcx = create_font_context();
    let mut lcx: LayoutContext<ColorBrush> = LayoutContext::new();
    let mut builder = lcx.ranged_builder(&mut fcx, text, 1., false);
    builder.push_default(FontFamily::from(FONT_FAMILY_LIST));
    builder.push_default(LineHeight::Absolute(0.));
    builder.push_default(LeadingDistribution::Proportional);
    builder.push(StyleProperty::FontSize(21.), 0..6);
    builder.push(LineHeight::Absolute(25.2), 0..6);
    builder.push(StyleProperty::FontSize(13.), 6..12);
    builder.push(LineHeight::Absolute(24.), 6..12);
    builder.push(LeadingDistribution::HalfLeading, 6..12);
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    let line = layout.get(0).unwrap();
    let fonts = run_fonts(&line);
    let (large, small) = (fonts[0], fonts[1]);
    // The 21px box is proportional, the 13px box half-leading.
    let over = 25.2 * large.above_fraction();
    let under = small.descent + (24. - (small.ascent + small.descent)) / 2.;
    assert!((line.metrics().line_height - (over + under)).abs() < EPSILON);
}
