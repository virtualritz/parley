// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`LineBoxSizing`].

use parley::{
    InlineBox, InlineBoxKind, LeadingDistribution, LineBoxSizing, LineHeight, VerticalAlign,
};

use crate::util::line_boxes::{
    EPSILON, FontExtents, Options, Span, assert_glyphs_on_the_baseline, heights_and_baselines,
    layout, run_fonts,
};

/// so its box reaches further below. The baseline is where the 25 span, in Arimo, puts it.
#[test]
fn line_box_sizing_largest_line_height_of_three_fonts() {
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
        layout(
            text,
            Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
                .sizing(sizing),
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
        (baseline - 25. * arimo_font.above_fraction()).abs() < EPSILON,
        "baseline {baseline}"
    );
    assert_glyphs_on_the_baseline(&line);

    // The union of the same boxes is taller.
    let union = sized(LineBoxSizing::Union);
    let (union_height, union_baseline) = heights_and_baselines(&union)[0];
    assert!(union_height > 27., "union line height {union_height}");
    assert!((union_baseline - baseline).abs() < EPSILON);
}

/// tallest box: both sizings give the same line heights and baselines.
#[test]
fn line_box_sizing_largest_line_height_of_one_font_matches_the_proportional_union() {
    for (first, second) in [
        ((21., 25.2), (13., 24.)),
        ((13., 15.6), (21., 25.2)),
        ((13., 30.), (21., 25.2)),
    ] {
        let text = "first second\nthird";
        let spans = [
            Span::new(0..6, "Roboto", first.0, first.1),
            Span::new(6..13, "Roboto", second.0, second.1),
            Span::new(13..text.len(), "Roboto", 10., 12.),
        ];
        let sized = |sizing| {
            heights_and_baselines(&layout(
                text,
                Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
                    .sizing(sizing),
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

/// sizes the line, whatever their order.
#[test]
fn line_box_sizing_largest_line_height_tie_takes_the_box_reaching_highest() {
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
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
                .sizing(LineBoxSizing::LargestLineHeight),
            &spans,
            None,
        );
        let line = layout.get(0).unwrap();
        let highest = run_fonts(&line)
            .iter()
            .map(FontExtents::above_fraction)
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

/// line of exactly their line height.
#[test]
fn line_box_sizing_largest_line_height_of_mixed_fonts() {
    let text = "abc مرحبا";
    let spans = [
        Span::new(0..4, "Roboto", 20., 30.),
        Span::new(4..text.len(), "Noto Kufi Arabic", 20., 30.),
    ];
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
            .sizing(LineBoxSizing::LargestLineHeight),
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
fn line_box_sizing_largest_line_height_per_line() {
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
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
            .sizing(LineBoxSizing::LargestLineHeight),
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
fn line_box_sizing_largest_line_height_includes_the_root() {
    let text = "roboto";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(40.), LeadingDistribution::Proportional)
            .sizing(LineBoxSizing::LargestLineHeight),
        &[Span::new(0..6, "Roboto", 13., 15.6)],
        None,
    );
    let (height, _) = heights_and_baselines(&layout)[0];
    assert!((height - 40.).abs() < EPSILON, "line height {height}");
}

/// The tallest span's box is placed by its own leading distribution, here half-leading.
#[test]
fn line_box_sizing_largest_line_height_with_half_leading() {
    let text = "roboto مرحبا";
    let spans = [
        Span::new(0..7, "Roboto", 21., 25.2),
        Span::new(7..text.len(), "Noto Kufi Arabic", 13., 24.),
    ];
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::HalfLeading)
            .sizing(LineBoxSizing::LargestLineHeight),
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
fn line_box_sizing_largest_line_height_grows_for_an_inline_box() {
    let text = "roboto";
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
            .sizing(LineBoxSizing::LargestLineHeight),
        &[Span::new(0..6, "Roboto", 13., 15.6)],
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
    let under = 15.6 * (1. - roboto.above_fraction());
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
fn line_box_sizing_largest_line_height_of_a_metrics_relative_span() {
    let text = "normal abs";
    let spans = [
        Span::new(0..7, "Arimo", 20., 0.).with_line_height(LineHeight::MetricsRelative(1.5)),
        Span::new(7..text.len(), "Noto Kufi Arabic", 20., 20.),
    ];
    let layout = layout(
        text,
        Options::new(LineHeight::Absolute(0.), LeadingDistribution::Proportional)
            .sizing(LineBoxSizing::LargestLineHeight),
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
