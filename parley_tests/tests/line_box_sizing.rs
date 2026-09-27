// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`LineBoxSizing`].

use parley::{
    Alignment, AlignmentOptions, FontFamily, InlineBox, InlineBoxKind, LeadingDistribution,
    LineBoxSizing, LineHeight, StyleProperty, VerticalAlign,
};

use crate::test_name;
use crate::util::TestEnv;
use crate::util::line_boxes::{
    EPSILON, FontExtents, Options, Span, assert_close, assert_glyphs_on_the_baseline,
    heights_and_baselines, layout, run_baselines, run_fonts,
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

/// Boxes added to a line before the line breaker reverts to an earlier break opportunity don't
/// size the line (see `lines_revert_restores_line_height`).
#[test]
fn line_box_sizing_largest_line_height_revert() {
    // "aaa " fits, the tall "BBBBBBBB" doesn't: its first atoms are added to the first line before
    // the line breaker reverts to the opportunity after the space.
    let text = "aaa BBBBBBBB";
    for distribution in [
        LeadingDistribution::HalfLeading,
        LeadingDistribution::Proportional,
    ] {
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(16.), distribution)
                .sizing(LineBoxSizing::LargestLineHeight)
                .max_advance(Some(60.)),
            &[Span::new(4..text.len(), "Roboto", 16., 64.)],
            None,
        );
        let heights: Vec<f32> = heights_and_baselines(&layout)
            .iter()
            .map(|(height, _)| *height)
            .collect();
        assert_eq!(heights, [16., 64.], "{distribution:?}");
        assert_eq!(
            layout.get(0).unwrap().text_range(),
            0..4,
            "{distribution:?}"
        );
    }
}

/// When quantizing, the line is as tall as the largest line height, and its baseline is a whole
/// pixel.
#[test]
fn line_box_sizing_largest_line_height_quantized() {
    let text = "arabic مرحبا arimo";
    let spans = [
        Span::new(0..17, "Noto Kufi Arabic", 20., 23.),
        Span::new(17..text.len(), "Arimo", 16., 25.),
    ];
    for distribution in [
        LeadingDistribution::HalfLeading,
        LeadingDistribution::Proportional,
    ] {
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(0.), distribution)
                .sizing(LineBoxSizing::LargestLineHeight)
                .quantize(true),
            &spans,
            None,
        );
        let line = layout.get(0).unwrap();
        let arimo = run_fonts(&line)[run_fonts(&line).len() - 1];
        let (height, baseline) = heights_and_baselines(&layout)[0];
        assert_eq!(height, 25., "{distribution:?}");
        // The Arimo box, placed as when quantizing: a whole-pixel ascent and a floored leading
        // above the baseline.
        let leading_above = match distribution {
            LeadingDistribution::HalfLeading => {
                (25. - (arimo.ascent.round() + arimo.descent.round())) / 2.
            }
            LeadingDistribution::Proportional => {
                25. * arimo.above_fraction() - arimo.ascent.round()
            }
        };
        assert_eq!(
            baseline,
            arimo.ascent.round() + leading_above.floor(),
            "{distribution:?}"
        );
        assert_glyphs_on_the_baseline(&line);
    }
}

/// Spans shifted by `vertical-align` are placed with their shift, and a `top` subtree is sized by
/// its own largest line height and grows the line box if it is taller.
#[test]
fn line_box_sizing_largest_line_height_shifted_and_top_spans() {
    let text = "base top sup";
    for (top_line_height, expected_height) in [(10., 30.), (40., 40.)] {
        let spans = [
            Span::new(5..8, "Roboto", 16., top_line_height).with_vertical_align(VerticalAlign::TOP),
            Span::new(9..12, "Roboto", 16., 30.).with_vertical_align(VerticalAlign::SUPER),
        ];
        let layout = layout(
            text,
            Options::new(LineHeight::Absolute(20.), LeadingDistribution::HalfLeading)
                .sizing(LineBoxSizing::LargestLineHeight),
            &spans,
            None,
        );
        let line = layout.get(0).unwrap();
        let roboto = run_fonts(&line)[0];
        let (height, baseline) = heights_and_baselines(&layout)[0];
        let case = format!("top line height {top_line_height}");
        assert_close(height, expected_height, &case);
        // The root subtree is sized by the `super` span's box of 30, which is shifted up by a
        // third of the root's font size.
        let over = 16. / 3. + roboto.ascent + (30. - (roboto.ascent + roboto.descent)) / 2.;
        assert_close(baseline, over, &case);

        // The `top` span's box is at the top of the line.
        let top_baseline = run_baselines(&line)[1] - line.metrics().block_min_coord;
        let top_over = roboto.ascent + (top_line_height - (roboto.ascent + roboto.descent)) / 2.;
        assert_close(top_baseline, top_over, &case);
    }
}

/// Under half-leading, the small Arimo text with a line height of 30 makes the union taller than
/// 30; the largest line height makes the first line exactly 30.
#[test]
fn line_box_sizing_mixed_fonts() {
    let text = "Kufi مرحبا Arimo text and a second line";
    let mut env = TestEnv::new(test_name!(), None);
    for (sizing, name) in [
        (LineBoxSizing::Union, "union"),
        (LineBoxSizing::LargestLineHeight, "largest_line_height"),
    ] {
        let mut builder = env.ranged_builder(text);
        builder.push_default(LineHeight::Absolute(0.));
        builder.push(StyleProperty::FontSize(24.), 5..15);
        builder.push(LineHeight::Absolute(24.), 5..15);
        builder.push(FontFamily::from("Arimo"), 16..21);
        builder.push(StyleProperty::FontSize(10.), 16..21);
        builder.push(LineHeight::Absolute(30.), 16..21);
        builder.push(LineHeight::Absolute(18.), 21..text.len());
        let mut layout = builder.build(text);
        layout.set_line_box_sizing(sizing);
        layout.break_all_lines(Some(200.));
        layout.align(Alignment::Start, AlignmentOptions::default());
        let height = layout.get(0).unwrap().metrics().line_height;
        match sizing {
            LineBoxSizing::Union => assert!(height > 31., "union line height {height}"),
            LineBoxSizing::LargestLineHeight => assert_eq!(height, 30.),
        }
        env.with_name(name).check_layout_snapshot(&layout);
    }
}
