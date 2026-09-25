// Copyright 2024 the Parley Authors
// SPDX-License-Identifier: Apache-2.0 OR MIT

use super::BreakReason;
use crate::data::LayoutData;
use crate::style::Brush;

/// Alignment of a layout.
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Alignment {
    /// This is [`Alignment::Left`] for LTR text and [`Alignment::Right`] for RTL text.
    #[default]
    Start,
    /// This is [`Alignment::Right`] for LTR text and [`Alignment::Left`] for RTL text.
    End,
    /// Align content to the left edge.
    ///
    /// For alignment that should be aware of text direction, use [`Alignment::Start`] or
    /// [`Alignment::End`] instead.
    Left,
    /// Align each line centered within the container.
    Center,
    /// Align content to the right edge.
    ///
    /// For alignment that should be aware of text direction, use [`Alignment::Start`] or
    /// [`Alignment::End`] instead.
    Right,
    /// Justify each line by spacing out content, except for the last line.
    ///
    /// The last line of a paragraph and each line ending in a hard break are aligned by
    /// [`AlignmentOptions::last_line_alignment`], and start aligned by default.
    ///
    /// A line with no justification opportunity, such as a single long word, can't be spaced
    /// out and is start aligned.
    Justify,
}

/// Additional options to fine tune alignment
#[derive(Debug, Clone, Copy)]
pub struct AlignmentOptions {
    /// If set to `true`, "end" and "center" alignment will apply even if the line contents are
    /// wider than the alignment width. If it is set to `false`, all overflowing lines will be
    /// [`Alignment::Start`] aligned.
    pub align_when_overflowing: bool,
    /// The alignment of last lines, modelled on the CSS `text-align-last` property.
    ///
    /// The last lines are the last line of the paragraph and each line ending in a hard break.
    /// `None` is CSS `auto`: they are aligned like the other lines, except that under
    /// [`Alignment::Justify`] they are start aligned. `Some(alignment)` aligns them with
    /// `alignment`, whatever the alignment of the other lines; `Some(Alignment::Justify)`
    /// justifies them too.
    ///
    /// Lines with no justification opportunity are start aligned, whether or not they are last
    /// lines: this applies to the other lines under [`Alignment::Justify`], and to last lines
    /// under `Some(Alignment::Justify)`. CSS aligns them by `text-align-last` instead (centred if
    /// that is `justify`); browsers and Skia start align them.
    pub last_line_alignment: Option<Alignment>,
}

#[expect(
    clippy::derivable_impls,
    reason = "Make default values explicit rather than relying on the implicit default value of bool"
)]
impl Default for AlignmentOptions {
    fn default() -> Self {
        Self {
            align_when_overflowing: false,
            last_line_alignment: None,
        }
    }
}

/// Align the layout.
pub(crate) fn align<B: Brush>(
    layout: &mut LayoutData<B>,
    alignment: Alignment,
    options: AlignmentOptions,
) {
    layout.alignment = Some(alignment);

    let is_rtl = layout.base_level.is_rtl();

    // Apply alignment to line items
    for line in &mut layout.lines {
        line.justification.amount_per_opportunity = 0.;

        let indent = line.indent;

        if is_rtl {
            // In RTL text, trailing whitespace is on the left. As we hang that whitespace, offset
            // the line to the left. Note: indent is not subtracted here because `free_space` below
            // already accounts for it.
            line.metrics.offset = -line.metrics.hanging_advance;
        } else {
            line.metrics.offset = indent;
        }

        // Compute free space.
        let line_width = line.metrics.inline_max_coord - line.metrics.inline_min_coord;
        let free_space = line_width - indent - line.metrics.advance + line.metrics.hanging_advance;

        if !options.align_when_overflowing && free_space <= 0.0 {
            if is_rtl {
                // In RTL text, right-align on overflow.
                line.metrics.offset += free_space;
            }
            continue;
        }

        // The paragraph's last line (`BreakReason::None`) and lines ending in a hard break
        // (`BreakReason::Explicit`).
        let is_last_line = matches!(line.break_reason, BreakReason::None | BreakReason::Explicit);
        let line_alignment = match options.last_line_alignment {
            Some(last_line_alignment) if is_last_line => last_line_alignment,
            _ => alignment,
        };

        match (line_alignment, is_rtl) {
            (Alignment::Left, _) | (Alignment::Start, false) | (Alignment::End, true) => {
                // Do nothing
            }
            (Alignment::Right, _) | (Alignment::Start, true) | (Alignment::End, false) => {
                line.metrics.offset += free_space;
            }
            (Alignment::Center, _) => {
                line.metrics.offset += free_space * 0.5;
            }
            (Alignment::Justify, _) => {
                // Justified alignment doesn't have any effect if free_space is negative or zero
                if free_space <= 0.0 {
                    continue;
                }

                // Justified alignment doesn't apply to last lines unless `last_line_alignment`
                // asks for it, or if there are no whitespace gaps to adjust. In that case,
                // start-align, i.e., left-align for LTR text and right-align for RTL text.
                if (is_last_line && options.last_line_alignment.is_none())
                    || line.num_justification_opportunities == 0
                {
                    if is_rtl {
                        line.metrics.offset += free_space;
                    }
                    continue;
                }

                line.justification.amount_per_opportunity =
                    free_space / line.num_justification_opportunities as f32;
            }
        }
    }
}
