// Copyright 2026 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Tests for [`AlignmentOptions::last_line_alignment`], the alignment of the paragraph's last line
//! and lines ending in a hard break.

use crate::test_name;
use crate::util::{ColorBrush, TestEnv};
use parley::{Alignment, AlignmentOptions, BreakReason, Layout, PositionedLayoutItem};

const LATIN: &str = "The quick brown fox jumps over the lazy dog and runs far away.";
const ARABIC: &str = "عند برمجة أجهزة الكمبيوتر، قد تجد نفسك فجأة في مواقف غريبة.";

/// Tolerance for comparing positions, in layout units.
const EPSILON: f32 = 0.01;

fn build_layout(
    env: &mut TestEnv,
    text: &str,
    width: f32,
    alignment: Alignment,
    last_line_alignment: Option<Alignment>,
) -> Layout<ColorBrush> {
    let builder = env.ranged_builder(text);
    let mut layout = builder.build(text);
    layout.break_all_lines(Some(width));
    layout.align(
        alignment,
        AlignmentOptions {
            last_line_alignment,
            ..AlignmentOptions::default()
        },
    );
    layout
}

/// The left and right edges of each line's visible content, i.e. of its clusters other than
/// whitespace and hard breaks.
fn content_extents(layout: &Layout<ColorBrush>) -> Vec<(f32, f32)> {
    layout
        .lines()
        .map(|line| {
            let mut extent = (f32::INFINITY, f32::NEG_INFINITY);
            for item in line.items() {
                let PositionedLayoutItem::GlyphRun(glyph_run) = item else {
                    continue;
                };
                let mut x = glyph_run.offset();
                for cluster in glyph_run.run().visual_clusters() {
                    let advance = cluster.advance();
                    if !cluster.is_space_or_nbsp() && !cluster.is_hard_line_break() {
                        extent.0 = extent.0.min(x);
                        extent.1 = extent.1.max(x + advance);
                    }
                    x += advance;
                }
            }
            extent
        })
        .collect()
}

fn assert_close(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() < EPSILON,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assert_fills(extent: (f32, f32), width: f32, line: usize) {
    assert_close(extent.0, 0.0, &format!("line {line} left edge"));
    assert_close(extent.1, width, &format!("line {line} right edge"));
}

fn assert_centered(extent: (f32, f32), width: f32, line: usize) {
    assert!(
        extent.0 > 1.0,
        "line {line} should not be start aligned: {extent:?}"
    );
    assert_close(
        extent.0,
        width - extent.1,
        &format!("line {line} free space"),
    );
}

#[test]
fn text_align_last_justify_center() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let layout = build_layout(
        &mut env,
        LATIN,
        width,
        Alignment::Justify,
        Some(Alignment::Center),
    );
    let extents = content_extents(&layout);

    assert_eq!(extents.len(), 3, "{extents:?}");
    assert_fills(extents[0], width, 0);
    assert_fills(extents[1], width, 1);
    assert_centered(extents[2], width, 2);

    env.check_layout_snapshot(&layout);
}

#[test]
fn text_align_last_justify_right() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let layout = build_layout(
        &mut env,
        LATIN,
        width,
        Alignment::Justify,
        Some(Alignment::Right),
    );
    let extents = content_extents(&layout);

    assert_eq!(extents.len(), 3, "{extents:?}");
    assert_fills(extents[0], width, 0);
    assert_fills(extents[1], width, 1);
    assert!(extents[2].0 > 1.0, "{extents:?}");
    assert_close(extents[2].1, width, "line 2 right edge");
}

#[test]
fn text_align_last_justify_justifies_last_line() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let layout = build_layout(
        &mut env,
        LATIN,
        width,
        Alignment::Justify,
        Some(Alignment::Justify),
    );
    let extents = content_extents(&layout);

    assert_eq!(extents.len(), 3, "{extents:?}");
    for (i, extent) in extents.into_iter().enumerate() {
        assert_fills(extent, width, i);
    }
}

#[test]
fn text_align_last_none_is_start() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let default_layout = {
        let builder = env.ranged_builder(LATIN);
        let mut layout = builder.build(LATIN);
        layout.break_all_lines(Some(width));
        layout.align(Alignment::Justify, AlignmentOptions::default());
        layout
    };
    let none_layout = build_layout(&mut env, LATIN, width, Alignment::Justify, None);
    let start_layout = build_layout(
        &mut env,
        LATIN,
        width,
        Alignment::Justify,
        Some(Alignment::Start),
    );

    let extents = content_extents(&none_layout);
    assert_eq!(extents, content_extents(&default_layout));
    assert_eq!(extents, content_extents(&start_layout));
    assert_eq!(extents.len(), 3, "{extents:?}");
    assert_fills(extents[0], width, 0);
    assert_fills(extents[1], width, 1);
    assert_close(extents[2].0, 0.0, "line 2 left edge");
    assert!(extents[2].1 < width - 1.0, "{extents:?}");
}

#[test]
fn text_align_last_hard_break() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let text = "The quick brown fox jumps over the lazy dog\nand runs away from the hunter into the woods.";
    let layout = build_layout(
        &mut env,
        text,
        width,
        Alignment::Justify,
        Some(Alignment::Center),
    );
    let extents = content_extents(&layout);
    let break_reasons: Vec<_> = layout.lines().map(|line| line.break_reason()).collect();

    assert_eq!(
        break_reasons,
        [
            BreakReason::Regular,
            BreakReason::Explicit,
            BreakReason::Regular,
            BreakReason::None
        ],
        "{extents:?}"
    );
    assert_fills(extents[0], width, 0);
    assert_centered(extents[1], width, 1);
    assert_fills(extents[2], width, 2);
    assert_centered(extents[3], width, 3);
}

#[test]
fn text_align_last_rtl() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;

    // Right alignment of RTL text is start alignment, `None` included.
    for last_line_alignment in [None, Some(Alignment::Start), Some(Alignment::Right)] {
        let layout = build_layout(
            &mut env,
            ARABIC,
            width,
            Alignment::Justify,
            last_line_alignment,
        );
        let extents = content_extents(&layout);
        let last = extents.len() - 1;
        assert!(last >= 2, "{extents:?}");
        for (i, extent) in extents[..last].iter().enumerate() {
            assert_fills(*extent, width, i);
        }
        assert!(extents[last].0 > 1.0, "{extents:?}");
        assert_close(extents[last].1, width, "last line right edge");
    }

    let layout = build_layout(
        &mut env,
        ARABIC,
        width,
        Alignment::Justify,
        Some(Alignment::Center),
    );
    let extents = content_extents(&layout);
    let last = extents.len() - 1;
    for (i, extent) in extents[..last].iter().enumerate() {
        assert_fills(*extent, width, i);
    }
    assert_centered(extents[last], width, last);

    // End alignment of RTL text is left alignment.
    let layout = build_layout(
        &mut env,
        ARABIC,
        width,
        Alignment::Justify,
        Some(Alignment::End),
    );
    let extents = content_extents(&layout);
    let last = extents.len() - 1;
    for (i, extent) in extents[..last].iter().enumerate() {
        assert_fills(*extent, width, i);
    }
    assert_close(extents[last].0, 0.0, "last line left edge");
    assert!(extents[last].1 < width - 1.0, "{extents:?}");

    env.check_layout_snapshot(&layout);
}

#[test]
fn text_align_last_without_justify() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let layout = build_layout(
        &mut env,
        LATIN,
        width,
        Alignment::Left,
        Some(Alignment::Right),
    );
    let extents = content_extents(&layout);

    assert_eq!(extents.len(), 3, "{extents:?}");
    for (i, extent) in extents[..2].iter().enumerate() {
        assert_close(extent.0, 0.0, &format!("line {i} left edge"));
        assert!(extent.1 < width - 1.0, "{extents:?}");
    }
    assert!(extents[2].0 > 1.0, "{extents:?}");
    assert_close(extents[2].1, width, "line 2 right edge");
}

/// A line with no justification opportunity is start aligned: a line that isn't a last line
/// ignores `last_line_alignment`, and a last line that `last_line_alignment` asks to justify
/// falls back to start alignment.
#[test]
fn text_align_last_unjustifiable_line_is_start_aligned() {
    let mut env = TestEnv::new(test_name!(), None);
    let width = 200.0;
    let text = "Incomprehensibilities antidisestablishment";
    let layout = build_layout(
        &mut env,
        text,
        width,
        Alignment::Justify,
        Some(Alignment::Center),
    );
    let extents = content_extents(&layout);
    let break_reasons: Vec<_> = layout.lines().map(|line| line.break_reason()).collect();

    assert_eq!(
        break_reasons,
        [BreakReason::Regular, BreakReason::None],
        "{extents:?}"
    );
    assert_close(extents[0].0, 0.0, "line 0 left edge");
    assert!(extents[0].1 < width - 1.0, "{extents:?}");
    assert_centered(extents[1], width, 1);

    let layout = build_layout(
        &mut env,
        text,
        width,
        Alignment::Justify,
        Some(Alignment::Justify),
    );
    let extents = content_extents(&layout);
    assert_eq!(extents.len(), 2, "{extents:?}");
    for (i, extent) in extents.into_iter().enumerate() {
        assert_close(extent.0, 0.0, &format!("line {i} left edge"));
        assert!(extent.1 < width - 1.0, "line {i}: {extent:?}");
    }
}
