// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`LeadingDistribution`].

use parley::{
    Alignment, AlignmentOptions, FontFamily, LayoutContext, LeadingDistribution, LineHeight,
    StyleProperty, VerticalAlign,
};

use crate::test_name;
use crate::util::env::{FONT_FAMILY_LIST, create_font_context};
use crate::util::line_boxes::{
    EPSILON, FontExtents, Options, Span, assert_close, assert_glyphs_on_the_baseline,
    baseline_from_top, heights_and_baselines, layout, run_baselines, run_fonts,
};
use crate::util::{ColorBrush, TestEnv};

/// Spans in one font make a line as tall as the largest of their line heights, whatever their
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

/// With the default half-leading, the smaller font's relatively larger line height reaches
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

/// The root style is every line's strut, and its box takes part like a span's: its line height
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

/// Spans in fonts with different ascent to descent ratios can make a line taller than the
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

/// A `normal` (metrics relative) line height is resolved per style as before, and its box puts
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

/// The baseline of the second glyph run of `text` relative to the line's baseline (positive
/// downwards), where the run is a span of 10px text with a line height of 40 and `align`.
fn span_baseline_offset(distribution: LeadingDistribution, align: VerticalAlign) -> (f32, f32) {
    let text = "root small";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), distribution),
        &[Span::new(5..10, "Roboto", 10., 40.).with_vertical_align(align)],
        None,
    );
    let line = layout.get(0).unwrap();
    let baselines = run_baselines(&line);
    assert_eq!(baselines.len(), 2, "{baselines:?}");
    // The box of the span reaches this far above its baseline.
    let span = run_fonts(&line)[1];
    let over = match distribution {
        LeadingDistribution::HalfLeading => span.ascent + (40. - (span.ascent + span.descent)) / 2.,
        LeadingDistribution::Proportional => 40. * span.above_fraction(),
    };
    (baselines[1] - line.metrics().baseline, over)
}

/// `vertical-align: text-top | text-bottom | middle` align a span by its line-height box, so they
/// place the span according to its leading distribution. `baseline` doesn't.
#[test]
fn leading_distribution_moves_vertical_align() {
    // `text-top` aligns the top of the span's box with the top of the root's content area.
    let (half_leading, half_leading_over) =
        span_baseline_offset(LeadingDistribution::HalfLeading, VerticalAlign::TEXT_TOP);
    let (proportional, proportional_over) =
        span_baseline_offset(LeadingDistribution::Proportional, VerticalAlign::TEXT_TOP);
    assert!((half_leading - 8.57).abs() < 0.01, "{half_leading}");
    assert!((proportional - 16.82).abs() < 0.01, "{proportional}");

    // The other values move the span by the same difference between the boxes' tops.
    let difference = proportional_over - half_leading_over;
    assert_close(proportional - half_leading, difference, "text-top");
    for (align, name) in [
        (VerticalAlign::TEXT_BOTTOM, "text-bottom"),
        (VerticalAlign::MIDDLE, "middle"),
    ] {
        let (half_leading, _) = span_baseline_offset(LeadingDistribution::HalfLeading, align);
        let (proportional, _) = span_baseline_offset(LeadingDistribution::Proportional, align);
        assert_close(proportional - half_leading, difference, name);
    }
    for distribution in [
        LeadingDistribution::HalfLeading,
        LeadingDistribution::Proportional,
    ] {
        let (offset, _) = span_baseline_offset(distribution, VerticalAlign::BASELINE);
        assert_close(offset, 0., "baseline");
    }
}

/// When quantizing, the proportional box is placed with whole-pixel ascents and a floored
/// leading above the baseline, so baselines and line heights are whole pixels.
#[test]
fn leading_distribution_proportional_quantized() {
    let text = "first second";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional).quantize(true),
        &[
            Span::new(0..6, "Roboto", 21., 25.),
            Span::new(6..12, "Roboto", 13., 24.),
        ],
        None,
    );
    let line = layout.get(0).unwrap();
    let fonts = run_fonts(&line);
    let (large, small) = (fonts[0], fonts[1]);
    // Each box is `round(ascent) + floor(line_height * fraction - round(ascent))` above the
    // baseline, where the fraction comes from the unrounded metrics.
    let over = |font: FontExtents, line_height: f32| {
        font.ascent.round() + (line_height * font.above_fraction() - font.ascent.round()).floor()
    };
    let large_over = over(large, 25.);
    let small_over = over(small, 24.);
    let expected_over = large_over.max(small_over);
    let expected_under = (25. - large_over).max(24. - small_over);
    let (height, baseline) = heights_and_baselines(&layout)[0];
    assert_close(baseline, expected_over, "baseline");
    assert_close(height, expected_over + expected_under, "line height");
    assert_eq!(baseline, baseline.round(), "baseline {baseline}");
    assert_eq!(height, height.round(), "line height {height}");
    assert_glyphs_on_the_baseline(&line);
}

/// With a `normal` line height, the glyphs of a fallback font add a box of that font, and its
/// leading is distributed proportionally too.
#[test]
fn leading_distribution_proportional_fallback_font() {
    // Roboto, the first available font, has no Arabic, so this is set in Noto Kufi Arabic.
    let text = "عليكم";
    let normal = LineHeight::MetricsRelative(1.5);
    let proportional = layout(
        text,
        Options::new(normal, LeadingDistribution::Proportional),
        &[],
        None,
    );
    let line = proportional.get(0).unwrap();
    let fallback = run_fonts(&line)[0];
    let line_height = 1.5 * (fallback.ascent + fallback.descent + fallback.leading);
    let (height, baseline) = heights_and_baselines(&proportional)[0];
    // The fallback font's box is taller than the root's box in Roboto, above and below the
    // baseline.
    assert_close(height, line_height, "line height");
    assert_close(
        baseline,
        line_height * fallback.above_fraction(),
        "baseline",
    );

    // With half-leading, the same box sits lower.
    let half_leading = layout(
        text,
        Options::new(normal, LeadingDistribution::HalfLeading),
        &[],
        None,
    );
    let (_, half_leading_baseline) = heights_and_baselines(&half_leading)[0];
    let expected = fallback.ascent + (line_height - (fallback.ascent + fallback.descent)) / 2.;
    assert_close(half_leading_baseline, expected, "half-leading baseline");
}

/// Small text with a large line height, large text with a tight one, and a `text-top` span,
/// under each distribution.
#[test]
fn leading_distribution_mixed_sizes() {
    let text = "small LARGE top مرحبا and a second line";
    let mut env = TestEnv::new(test_name!(), None);
    for (distribution, name) in [
        (LeadingDistribution::HalfLeading, "half_leading"),
        (LeadingDistribution::Proportional, "proportional"),
    ] {
        let mut builder = env.ranged_builder(text);
        builder.push_default(LineHeight::FontSizeRelative(1.));
        builder.push_default(distribution);
        builder.push(StyleProperty::FontSize(10.), 0..5);
        builder.push(LineHeight::Absolute(40.), 0..5);
        builder.push(StyleProperty::FontSize(32.), 6..11);
        builder.push(LineHeight::Absolute(34.), 6..11);
        builder.push(StyleProperty::FontSize(10.), 12..15);
        builder.push(LineHeight::Absolute(40.), 12..15);
        builder.push(VerticalAlign::TEXT_TOP, 12..15);
        let mut layout = builder.build(text);
        layout.break_all_lines(Some(200.));
        layout.align(Alignment::Start, AlignmentOptions::default());
        env.with_name(name).check_layout_snapshot(&layout);
    }
}
