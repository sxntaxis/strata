use ratatui::layout::Rect;

pub(super) fn centered_content_rect(
    terminal: Rect,
    desired_inner_width: u16,
    desired_inner_height: u16,
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

    let width = desired_inner_width.saturating_add(2).clamp(1, max_width);
    let height = desired_inner_height.saturating_add(2).clamp(1, max_height);
    let x = terminal.x + terminal.width.saturating_sub(width) / 2;
    let y = terminal.y + terminal.height.saturating_sub(height) / 2;

    Rect::new(x, y, width, height)
}

#[cfg(test)]
mod tests {
    use ratatui::layout::Rect;

    use super::centered_content_rect;

    #[test]
    fn content_rect_sizes_from_inner_content_and_centers_it() {
        let rect = centered_content_rect(Rect::new(0, 0, 100, 40), 47, 13, 2);
        assert_eq!(rect, Rect::new(25, 12, 49, 15));
    }

    #[test]
    fn content_rect_clamps_to_frame_margin_without_overflow() {
        let rect = centered_content_rect(Rect::new(5, 7, 30, 12), 80, 40, 2);
        assert_eq!(rect, Rect::new(7, 9, 26, 8));
    }
}
