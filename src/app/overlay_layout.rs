use ratatui::layout::Rect;

const HORIZONTAL_CONTENT_INSET: u16 = 1;
const MAX_VERTICAL_CONTENT_INSET: u16 = 2;

pub(super) fn centered_overlay_rect(
    terminal: Rect,
    minimum_inner_width: u16,
    minimum_inner_height: u16,
    comfort_numerator: u16,
    comfort_denominator: u16,
    frame_margin: u16,
) -> Rect {
    let max_width = terminal
        .width
        .saturating_sub(frame_margin.saturating_mul(2))
        .max(1);
    let max_height = terminal
        .height
        .saturating_sub(frame_margin.saturating_mul(2))
        .max(1);

    let denominator = comfort_denominator.max(1);
    let comfort_width = terminal.width.saturating_mul(comfort_numerator) / denominator;
    let comfort_height = terminal.height.saturating_mul(comfort_numerator) / denominator;
    let minimum_width = minimum_inner_width.saturating_add(2);
    let minimum_height = minimum_inner_height.saturating_add(2);

    let width = comfort_width.max(minimum_width).clamp(1, max_width);
    let height = comfort_height.max(minimum_height).clamp(1, max_height);
    let x = terminal.x + terminal.width.saturating_sub(width) / 2;
    let y = terminal.y + terminal.height.saturating_sub(height) / 2;

    Rect::new(x, y, width, height)
}

/// Inset ordinary modal content without turning the modal into a fixed-width card.
///
/// Horizontal content inset is deliberately fixed at one terminal cell per side.
/// Modal geometry already owns the hard content minimum, so wider overlays keep
/// their spare width inside the content span instead of converting it into larger
/// side padding. Only physically tiny areas that cannot retain a content cell drop
/// the inset. Vertical padding remains optional and surplus-driven because surfaces
/// such as Balance own their own section rhythm.
pub(super) fn modal_content_rect(
    area: Rect,
    minimum_content_height: u16,
    vertical_padding: bool,
) -> Rect {
    let horizontal = if area.width
        >= HORIZONTAL_CONTENT_INSET
            .saturating_mul(2)
            .saturating_add(1)
    {
        HORIZONTAL_CONTENT_INSET
    } else {
        0
    };

    let vertical = if vertical_padding {
        (area.height.saturating_sub(minimum_content_height) / 2).min(MAX_VERTICAL_CONTENT_INSET)
    } else {
        0
    };

    Rect::new(
        area.x.saturating_add(horizontal),
        area.y.saturating_add(vertical),
        area.width.saturating_sub(horizontal.saturating_mul(2)),
        area.height.saturating_sub(vertical.saturating_mul(2)),
    )
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::{centered_overlay_rect, modal_content_rect};

    #[test]
    fn proportional_comfort_floor_wins_when_content_is_smaller() {
        let rect = centered_overlay_rect(Rect::new(0, 0, 120, 60), 20, 8, 1, 3, 2);
        assert_eq!(rect, Rect::new(40, 20, 40, 20));
    }

    #[test]
    fn content_minimum_wins_when_larger_than_comfort_floor() {
        let rect = centered_overlay_rect(Rect::new(0, 0, 120, 60), 47, 23, 1, 3, 2);
        assert_eq!(rect, Rect::new(35, 17, 49, 25));
    }

    #[test]
    fn overlay_clamps_to_frame_margin_without_overflow() {
        let rect = centered_overlay_rect(Rect::new(5, 7, 30, 12), 80, 40, 1, 3, 2);
        assert_eq!(rect, Rect::new(7, 9, 26, 8));
    }

    #[test]
    fn tiny_terminal_remains_bounded() {
        let rect = centered_overlay_rect(Rect::new(0, 0, 2, 1), 80, 40, 1, 3, 2);
        assert_eq!(rect, Rect::new(0, 0, 1, 1));
    }

    #[test]
    fn non_zero_terminal_origin_is_preserved_when_centering() {
        let rect = centered_overlay_rect(Rect::new(10, 20, 90, 45), 20, 8, 1, 3, 2);
        assert_eq!(rect, Rect::new(40, 35, 30, 15));
    }

    #[test]
    fn modal_content_inset_is_exactly_one_cell_when_possible() {
        assert_eq!(
            modal_content_rect(Rect::new(10, 20, 49, 13), 13, false),
            Rect::new(11, 20, 47, 13)
        );
        assert_eq!(
            modal_content_rect(Rect::new(10, 20, 59, 13), 13, false),
            Rect::new(11, 20, 57, 13)
        );
    }

    #[test]
    fn modal_content_inset_drops_only_when_one_cell_per_side_is_impossible() {
        assert_eq!(
            modal_content_rect(Rect::new(3, 4, 2, 8), 8, true),
            Rect::new(3, 4, 2, 8)
        );
        assert_eq!(
            modal_content_rect(Rect::new(3, 4, 3, 8), 8, true),
            Rect::new(4, 4, 1, 8)
        );
    }

    #[test]
    fn surplus_vertical_inset_keeps_existing_calm() {
        let rect = modal_content_rect(Rect::new(10, 20, 40, 14), 10, true);
        assert_eq!(rect, Rect::new(11, 22, 38, 10));
    }
}
