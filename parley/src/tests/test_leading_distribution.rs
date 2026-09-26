// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`LeadingDistribution`] and [`LineBoxSizing`].

use alloc::borrow::Cow;
use alloc::format;
use alloc::vec::Vec;
use core::ops::Range;

use parley_engine::FontMetrics;

use super::utils::ColorBrush;
use super::utils::fonts::{FONT_FAMILY_LIST, create_font_context};
use crate::layout::style_metrics::BoxMetrics;
use crate::{
    FontFamily, FontFamilyName, InlineBox, InlineBoxKind, Layout, LayoutContext,
    LeadingDistribution, Line, LineBoxSizing, LineHeight, PositionedLayoutItem, StyleProperty,
    VerticalAlign,
};

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

/// One span: a byte range with its font family, font size and line height.
struct Span {
    range: Range<usize>,
    family: &'static str,
    font_size: f32,
    line_height: LineHeight,
}

impl Span {
    fn roboto(range: Range<usize>, font_size: f32, line_height: f32) -> Self {
        Self {
            range,
            family: "Roboto",
            font_size,
            line_height: LineHeight::Absolute(line_height),
        }
    }

    fn new(range: Range<usize>, family: &'static str, font_size: f32, line_height: f32) -> Self {
        Self {
            range,
            family,
            font_size,
            line_height: LineHeight::Absolute(line_height),
        }
    }
}

/// Lays out `text` without quantization, with a root style of `root` line height and the leading
/// distribution `distribution`, which the spans inherit.
fn layout(
    text: &str,
    root: LineHeight,
    distribution: LeadingDistribution,
    spans: &[Span],
) -> Layout<ColorBrush> {
    layout_sized(text, root, distribution, LineBoxSizing::Union, spans, None)
}

/// Like [`layout`], with the line boxes sized by `sizing`, and `inline_box` if any.
fn layout_sized(
    text: &str,
    root: LineHeight,
    distribution: LeadingDistribution,
    sizing: LineBoxSizing,
    spans: &[Span],
    inline_box: Option<InlineBox>,
) -> Layout<ColorBrush> {
    let mut fcx = create_font_context();
    let mut lcx: LayoutContext<ColorBrush> = LayoutContext::new();
    let mut builder = lcx.ranged_builder(&mut fcx, text, 1., false);
    builder.push_default(FontFamily::from(FONT_FAMILY_LIST));
    builder.push_default(root);
    builder.push_default(distribution);
    for span in spans {
        builder.push(
            FontFamilyName::Named(Cow::Borrowed(span.family)),
            span.range.clone(),
        );
        builder.push(StyleProperty::FontSize(span.font_size), span.range.clone());
        builder.push(span.line_height, span.range.clone());
    }
    if let Some(inline_box) = inline_box {
        builder.push_inline_box(inline_box);
    }
    let mut layout = builder.build(text);
    layout.set_line_box_sizing(sizing);
    layout.break_all_lines(None);
    layout
}

/// The fraction of a line height the proportional box of `font` puts above the baseline.
fn above_fraction(font: &FontMetrics) -> f32 {
    (font.ascent + font.leading / 2.) / (font.ascent + font.descent + font.leading)
}

/// The font metrics of each run of the line.
fn run_fonts(line: &Line<'_, ColorBrush>) -> Vec<FontMetrics> {
    line.runs().map(|run| *run.font_metrics()).collect()
}

/// The line's baseline, measured from the top of its line box.
fn baseline_from_top(line: &Line<'_, ColorBrush>) -> f32 {
    line.metrics().baseline - line.metrics().block_min_coord
}

/// Asserts that every glyph run of `line` is drawn on the line's baseline.
fn assert_glyphs_on_the_baseline(line: &Line<'_, ColorBrush>) {
    for item in line.items() {
        if let PositionedLayoutItem::GlyphRun(run) = item {
            assert_eq!(run.baseline(), line.metrics().baseline);
        }
    }
}

/// Spans in one font make a line as tall as the largest of their line heights, whatever their
/// font sizes, and the baseline sits that line height's share of the ascent below the top.
#[test]
fn proportional_line_is_the_largest_line_height_of_one_font() {
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
            LineHeight::Absolute(0.),
            LeadingDistribution::Proportional,
            &[
                Span::roboto(0..6, size_a, height_a),
                Span::roboto(6..12, size_b, height_b),
            ],
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
            (baseline - expected * above_fraction(&font)).abs() < EPSILON,
            "{case}: baseline {baseline}"
        );
        assert!(baseline > 0. && baseline < metrics.line_height, "{case}");
        assert_glyphs_on_the_baseline(&line);
    }
}

/// With the default half-leading, the smaller font's relatively larger line height reaches
/// further below the baseline than the larger font's box.
#[test]
fn half_leading_line_is_the_union_of_centred_boxes() {
    let text = "first second";
    let spans = [Span::roboto(0..6, 21., 25.2), Span::roboto(6..12, 13., 24.)];
    let half_leading = layout(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::HalfLeading,
        &spans,
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
fn proportional_lines_keep_their_own_line_heights() {
    let text = "first line\nsecond line";
    let tall = 0..11;
    let short = 11..text.len();
    for (spans, expected) in [
        (
            [
                Span::roboto(tall.clone(), 21., 25.2),
                Span::roboto(short.clone(), 13., 15.6),
            ],
            [25.2, 15.6],
        ),
        (
            [
                Span::roboto(tall.clone(), 13., 15.6),
                Span::roboto(short.clone(), 21., 25.2),
            ],
            [15.6, 25.2],
        ),
    ] {
        let layout = layout(
            text,
            LineHeight::Absolute(0.),
            LeadingDistribution::Proportional,
            &spans,
        );
        let heights: Vec<f32> = layout.lines().map(|l| l.metrics().line_height).collect();
        assert_eq!(heights.len(), 2);
        for (height, expected) in heights.iter().zip(expected) {
            assert!((height - expected).abs() < EPSILON, "{heights:?}");
        }
        assert!((layout.height() - 40.8).abs() < EPSILON);
        for line in layout.lines() {
            let font = run_fonts(&line)[0];
            let expected = line.metrics().line_height * above_fraction(&font);
            assert!((baseline_from_top(&line) - expected).abs() < EPSILON);
            assert_glyphs_on_the_baseline(&line);
        }
    }
}

/// The root style is every line's strut, and its box takes part like a span's: its line height
/// is a floor.
#[test]
fn proportional_root_line_height_is_a_floor() {
    let text = "first";
    let layout = layout(
        text,
        LineHeight::Absolute(40.),
        LeadingDistribution::Proportional,
        &[Span::roboto(0..5, 13., 15.6)],
    );
    let line = layout.get(0).unwrap();
    assert!((line.metrics().line_height - 40.).abs() < EPSILON);
    let font = run_fonts(&line)[0];
    assert!((baseline_from_top(&line) - 40. * above_fraction(&font)).abs() < EPSILON);
}

/// Spans in fonts with different ascent to descent ratios can make a line taller than the
/// largest line height: one box reaches highest above the baseline, the other lowest below it.
#[test]
fn proportional_line_of_mixed_fonts_can_exceed_the_largest_line_height() {
    // Roboto puts 1900/2400 of a line height above the baseline, Noto Kufi Arabic 1282/1897.
    let text = "abc مرحبا";
    let layout = layout(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        &[
            Span::roboto(0..4, 20., 30.),
            Span {
                range: 4..text.len(),
                family: "Noto Kufi Arabic",
                font_size: 20.,
                line_height: LineHeight::Absolute(30.),
            },
        ],
    );
    let line = layout.get(0).unwrap();
    let fractions: Vec<f32> = run_fonts(&line).iter().map(above_fraction).collect();
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
fn proportional_metrics_relative_line_height() {
    // Arimo has a line gap.
    let text = "normal";
    let layout = layout(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        &[Span {
            range: 0..6,
            family: "Arimo",
            font_size: 20.,
            line_height: LineHeight::MetricsRelative(1.5),
        }],
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
    let over = 25.2 * above_fraction(&large);
    let under = small.descent + (24. - (small.ascent + small.descent)) / 2.;
    assert!((line.metrics().line_height - (over + under)).abs() < EPSILON);
}

/// The line height and the baseline below the top of each line.
fn heights_and_baselines(layout: &Layout<ColorBrush>) -> Vec<(f32, f32)> {
    layout
        .lines()
        .map(|line| (line.metrics().line_height, baseline_from_top(&line)))
        .collect()
}

/// Three spans in three fonts with line heights 23, 11 and 25 make a line of 25, where their union
/// is taller: Noto Kufi Arabic puts less of its line height above the baseline than Arimo does,
/// so its box reaches further below. The baseline is where the 25 span, in Arimo, puts it.
#[test]
fn largest_line_height_of_three_fonts() {
    let text = "arabic مرحبا roboto arimo";
    let arabic = 0..17;
    let roboto = 17..24;
    let arimo = 24..text.len();
    let spans = [
        Span::new(arabic, "Noto Kufi Arabic", 20., 23.),
        Span::new(roboto, "Roboto", 30., 11.),
        Span::new(arimo, "Arimo", 16., 25.),
    ];
    let sized = |sizing| {
        layout_sized(
            text,
            LineHeight::Absolute(0.),
            LeadingDistribution::Proportional,
            sizing,
            &spans,
            None,
        )
    };

    let strict = sized(LineBoxSizing::LargestLineHeight);
    assert_eq!(strict.len(), 1);
    let line = strict.get(0).unwrap();
    let fonts = run_fonts(&line);
    let arimo_font = fonts[fonts.len() - 1];
    let (height, baseline) = heights_and_baselines(&strict)[0];
    assert!((height - 25.).abs() < EPSILON, "line height {height}");
    assert!((strict.height() - 25.).abs() < EPSILON);
    assert!(
        (baseline - 25. * above_fraction(&arimo_font)).abs() < EPSILON,
        "baseline {baseline}"
    );
    assert_glyphs_on_the_baseline(&line);

    // The union of the same boxes is taller.
    let union = sized(LineBoxSizing::Union);
    let (union_height, union_baseline) = heights_and_baselines(&union)[0];
    assert!(union_height > 27., "union line height {union_height}");
    assert!((union_baseline - baseline).abs() < EPSILON);
}

/// With spans in one font, every box is proportional to its line height, so the union is the
/// tallest box: both sizings give the same line heights and baselines.
#[test]
fn largest_line_height_of_one_font_matches_the_proportional_union() {
    for (first, second) in [
        ((21., 25.2), (13., 24.)),
        ((13., 15.6), (21., 25.2)),
        ((13., 30.), (21., 25.2)),
    ] {
        let text = "first second\nthird";
        let spans = [
            Span::roboto(0..6, first.0, first.1),
            Span::roboto(6..13, second.0, second.1),
            Span::roboto(13..text.len(), 10., 12.),
        ];
        let sized = |sizing| {
            heights_and_baselines(&layout_sized(
                text,
                LineHeight::Absolute(0.),
                LeadingDistribution::Proportional,
                sizing,
                &spans,
                None,
            ))
        };
        let strict = sized(LineBoxSizing::LargestLineHeight);
        let union = sized(LineBoxSizing::Union);
        assert_eq!(strict.len(), 2);
        for ((strict_height, strict_baseline), (union_height, union_baseline)) in
            strict.iter().zip(&union)
        {
            assert!(
                (strict_height - union_height).abs() < EPSILON,
                "{strict:?} {union:?}"
            );
            assert!(
                (strict_baseline - union_baseline).abs() < EPSILON,
                "{strict:?} {union:?}"
            );
        }
        let expected = f32::max(first.1, second.1);
        assert!((strict[0].0 - expected).abs() < EPSILON, "{strict:?}");
        assert!((strict[1].0 - 12.).abs() < EPSILON, "{strict:?}");
    }
}

/// Of two spans with the same line height, the one whose box reaches furthest above the baseline
/// sizes the line, whatever their order.
#[test]
fn largest_line_height_tie_takes_the_box_reaching_highest() {
    let text = "roboto مرحبا";
    let roboto = 0..7;
    let arabic = 7..text.len();
    for spans in [
        [
            Span::new(roboto.clone(), "Roboto", 20., 30.),
            Span::new(arabic.clone(), "Noto Kufi Arabic", 20., 30.),
        ],
        [
            Span::new(roboto.clone(), "Noto Kufi Arabic", 20., 30.),
            Span::new(arabic.clone(), "Roboto", 20., 30.),
        ],
    ] {
        let layout = layout_sized(
            text,
            LineHeight::Absolute(0.),
            LeadingDistribution::Proportional,
            LineBoxSizing::LargestLineHeight,
            &spans,
            None,
        );
        let line = layout.get(0).unwrap();
        let highest = run_fonts(&line)
            .iter()
            .map(above_fraction)
            .fold(f32::NEG_INFINITY, f32::max);
        let (height, baseline) = heights_and_baselines(&layout)[0];
        assert!((height - 30.).abs() < EPSILON, "line height {height}");
        assert!(
            (baseline - 30. * highest).abs() < EPSILON,
            "baseline {baseline}, expected {}",
            30. * highest
        );
        assert_glyphs_on_the_baseline(&line);
    }
}

/// The two fonts of `proportional_line_of_mixed_fonts_can_exceed_the_largest_line_height` make a
/// line of exactly their line height.
#[test]
fn largest_line_height_of_mixed_fonts() {
    let text = "abc مرحبا";
    let spans = [
        Span::roboto(0..4, 20., 30.),
        Span::new(4..text.len(), "Noto Kufi Arabic", 20., 30.),
    ];
    let layout = layout_sized(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        LineBoxSizing::LargestLineHeight,
        &spans,
        None,
    );
    let (height, baseline) = heights_and_baselines(&layout)[0];
    assert!((height - 30.).abs() < EPSILON, "line height {height}");
    // Roboto's 1900/2400 of 30.
    assert!((baseline - 23.75).abs() < 1e-3, "baseline {baseline}");
}

/// Each line is sized by the largest line height on it.
#[test]
fn largest_line_height_per_line() {
    let text = "roboto مرحبا\nمرحبا arimo";
    let first_line = 0..18;
    let spans = [
        Span::new(0..7, "Roboto", 21., 25.2),
        Span::new(7..first_line.end, "Noto Kufi Arabic", 13., 24.),
        Span::new(
            first_line.end..first_line.end + 11,
            "Noto Kufi Arabic",
            13.,
            15.6,
        ),
        Span::new(first_line.end + 11..text.len(), "Arimo", 30., 12.),
    ];
    let layout = layout_sized(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        LineBoxSizing::LargestLineHeight,
        &spans,
        None,
    );
    let lines = heights_and_baselines(&layout);
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!((lines[0].0 - 25.2).abs() < EPSILON, "{lines:?}");
    assert!((lines[1].0 - 15.6).abs() < EPSILON, "{lines:?}");
    assert!((layout.height() - 40.8).abs() < EPSILON);
    for line in layout.lines() {
        assert_glyphs_on_the_baseline(&line);
    }
}

/// The root style's strut is a span on every line: its line height is a floor.
#[test]
fn largest_line_height_includes_the_root() {
    let text = "roboto";
    let layout = layout_sized(
        text,
        LineHeight::Absolute(40.),
        LeadingDistribution::Proportional,
        LineBoxSizing::LargestLineHeight,
        &[Span::roboto(0..6, 13., 15.6)],
        None,
    );
    let (height, _) = heights_and_baselines(&layout)[0];
    assert!((height - 40.).abs() < EPSILON, "line height {height}");
}

/// The tallest span's box is placed by its own leading distribution, here half-leading.
#[test]
fn largest_line_height_with_half_leading() {
    let text = "roboto مرحبا";
    let spans = [
        Span::roboto(0..7, 21., 25.2),
        Span::new(7..text.len(), "Noto Kufi Arabic", 13., 24.),
    ];
    let layout = layout_sized(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::HalfLeading,
        LineBoxSizing::LargestLineHeight,
        &spans,
        None,
    );
    let line = layout.get(0).unwrap();
    let roboto = run_fonts(&line)[0];
    let (height, baseline) = heights_and_baselines(&layout)[0];
    assert!((height - 25.2).abs() < EPSILON, "line height {height}");
    let half_leading = (25.2 - (roboto.ascent + roboto.descent)) / 2.;
    assert!((baseline - (roboto.ascent + half_leading)).abs() < EPSILON);
}

/// An inline box is an object, not a span, so it still grows the line box to fit.
#[test]
fn largest_line_height_grows_for_an_inline_box() {
    let text = "roboto";
    let layout = layout_sized(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        LineBoxSizing::LargestLineHeight,
        &[Span::roboto(0..6, 13., 15.6)],
        Some(InlineBox {
            id: 0,
            kind: InlineBoxKind::InFlow,
            index: 3,
            width: 10.,
            height: 50.,
            baseline: None,
            vertical_align: VerticalAlign::BASELINE,
        }),
    );
    let line = layout.get(0).unwrap();
    let roboto = run_fonts(&line)[0];
    let under = 15.6 * (1. - above_fraction(&roboto));
    let (height, baseline) = heights_and_baselines(&layout)[0];
    // The box sits on the baseline, so the line reaches its top above it.
    assert!((baseline - 50.).abs() < EPSILON, "baseline {baseline}");
    assert!(
        (height - (50. + under)).abs() < EPSILON,
        "line height {height}"
    );
}

/// A `normal` line height is the resolved line height of its span.
#[test]
fn largest_line_height_of_a_metrics_relative_span() {
    let text = "normal abs";
    let spans = [
        Span {
            range: 0..7,
            family: "Arimo",
            font_size: 20.,
            line_height: LineHeight::MetricsRelative(1.5),
        },
        Span::new(7..text.len(), "Noto Kufi Arabic", 20., 20.),
    ];
    let layout = layout_sized(
        text,
        LineHeight::Absolute(0.),
        LeadingDistribution::Proportional,
        LineBoxSizing::LargestLineHeight,
        &spans,
        None,
    );
    let line = layout.get(0).unwrap();
    let arimo = run_fonts(&line)[0];
    let line_height = 1.5 * (arimo.ascent + arimo.descent + arimo.leading);
    let (height, baseline) = heights_and_baselines(&layout)[0];
    assert!(
        (height - line_height).abs() < EPSILON,
        "line height {height}"
    );
    assert!((baseline - 1.5 * (arimo.ascent + arimo.leading / 2.)).abs() < EPSILON);
}
