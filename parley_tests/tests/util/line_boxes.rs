// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Helpers for tests of [`LeadingDistribution`] and [`LineBoxSizing`], which lay out text
//! without quantization and compare line metrics against font metrics.

use std::borrow::Cow;
use std::ops::Range;

use parley::{
    FontFamily, FontFamilyName, InlineBox, Layout, LayoutContext, LeadingDistribution, Line,
    LineBoxSizing, LineHeight, PositionedLayoutItem, StyleProperty, VerticalAlign,
};

use super::ColorBrush;
use super::env::{FONT_FAMILY_LIST, create_font_context};

/// Tolerance for comparing positions, in layout units.
pub(crate) const EPSILON: f32 = 1e-4;

/// One span: a byte range with its font family, font size and line height, and optionally a
/// vertical alignment.
pub(crate) struct Span {
    pub(crate) range: Range<usize>,
    pub(crate) family: &'static str,
    pub(crate) font_size: f32,
    pub(crate) line_height: LineHeight,
    pub(crate) vertical_align: Option<VerticalAlign>,
}

impl Span {
    pub(crate) fn new(
        range: Range<usize>,
        family: &'static str,
        font_size: f32,
        line_height: f32,
    ) -> Self {
        Self {
            range,
            family,
            font_size,
            line_height: LineHeight::Absolute(line_height),
            vertical_align: None,
        }
    }

    pub(crate) fn with_line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = line_height;
        self
    }

    pub(crate) fn with_vertical_align(mut self, vertical_align: VerticalAlign) -> Self {
        self.vertical_align = Some(vertical_align);
        self
    }
}

/// The options of [`layout`] other than the text and the spans.
#[derive(Clone, Copy)]
pub(crate) struct Options {
    pub(crate) root: LineHeight,
    pub(crate) distribution: LeadingDistribution,
    pub(crate) sizing: LineBoxSizing,
    pub(crate) quantize: bool,
    pub(crate) max_advance: Option<f32>,
}

impl Options {
    /// A root line height of `root`, `distribution`, [`LineBoxSizing::Union`], no quantization
    /// and no wrapping.
    pub(crate) fn new(root: LineHeight, distribution: LeadingDistribution) -> Self {
        Self {
            root,
            distribution,
            sizing: LineBoxSizing::Union,
            quantize: false,
            max_advance: None,
        }
    }

    pub(crate) fn sizing(mut self, sizing: LineBoxSizing) -> Self {
        self.sizing = sizing;
        self
    }

    pub(crate) fn quantize(mut self, quantize: bool) -> Self {
        self.quantize = quantize;
        self
    }

    pub(crate) fn max_advance(mut self, max_advance: Option<f32>) -> Self {
        self.max_advance = max_advance;
        self
    }
}

/// Lays out `text` with `spans`, which inherit the root style's leading distribution, and
/// `inline_box` if any.
pub(crate) fn layout(
    text: &str,
    options: Options,
    spans: &[Span],
    inline_box: Option<InlineBox>,
) -> Layout<ColorBrush> {
    let mut fcx = create_font_context();
    let mut lcx: LayoutContext<ColorBrush> = LayoutContext::new();
    let mut builder = lcx.ranged_builder(&mut fcx, text, 1., options.quantize);
    builder.push_default(FontFamily::from(FONT_FAMILY_LIST));
    builder.push_default(options.root);
    builder.push_default(options.distribution);
    for span in spans {
        builder.push(
            FontFamilyName::Named(Cow::Borrowed(span.family)),
            span.range.clone(),
        );
        builder.push(StyleProperty::FontSize(span.font_size), span.range.clone());
        builder.push(span.line_height, span.range.clone());
        if let Some(vertical_align) = span.vertical_align {
            builder.push(vertical_align, span.range.clone());
        }
    }
    if let Some(inline_box) = inline_box {
        builder.push_inline_box(inline_box);
    }
    let mut layout = builder.build(text);
    layout.set_line_box_sizing(options.sizing);
    layout.break_all_lines(options.max_advance);
    layout
}

/// The extents of a run's font.
#[derive(Clone, Copy, Debug)]
pub(crate) struct FontExtents {
    pub(crate) ascent: f32,
    pub(crate) descent: f32,
    pub(crate) leading: f32,
}

impl FontExtents {
    /// The fraction of a line height the proportional box of the font puts above the baseline.
    pub(crate) fn above_fraction(&self) -> f32 {
        (self.ascent + self.leading / 2.) / (self.ascent + self.descent + self.leading)
    }
}

/// The font extents of each run of the line.
pub(crate) fn run_fonts(line: &Line<'_, ColorBrush>) -> Vec<FontExtents> {
    line.runs()
        .map(|run| {
            let metrics = run.font_metrics();
            FontExtents {
                ascent: metrics.ascent,
                descent: metrics.descent,
                leading: metrics.leading,
            }
        })
        .collect()
}

/// The line's baseline, measured from the top of its line box.
pub(crate) fn baseline_from_top(line: &Line<'_, ColorBrush>) -> f32 {
    line.metrics().baseline - line.metrics().block_min_coord
}

/// The line height and the baseline below the top of each line.
pub(crate) fn heights_and_baselines(layout: &Layout<ColorBrush>) -> Vec<(f32, f32)> {
    layout
        .lines()
        .map(|line| (line.metrics().line_height, baseline_from_top(&line)))
        .collect()
}

/// The baseline of each glyph run of the line.
pub(crate) fn run_baselines(line: &Line<'_, ColorBrush>) -> Vec<f32> {
    line.items()
        .filter_map(|item| match item {
            PositionedLayoutItem::GlyphRun(run) => Some(run.baseline()),
            PositionedLayoutItem::InlineBox(_) => None,
        })
        .collect()
}

/// Asserts that every glyph run of `line` is drawn on the line's baseline.
pub(crate) fn assert_glyphs_on_the_baseline(line: &Line<'_, ColorBrush>) {
    for baseline in run_baselines(line) {
        assert_eq!(
            baseline,
            line.metrics().baseline,
            "a glyph run is off the line's baseline"
        );
    }
}

/// Asserts that `actual` is within [`EPSILON`] of `expected`.
#[track_caller]
pub(crate) fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < EPSILON,
        "{what}: expected {expected}, got {actual}"
    );
}
