use ratatui::layout::Rect;

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

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::centered_overlay_rect;

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
}
