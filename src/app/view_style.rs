use ratatui::style::Color;

pub(super) fn balance_color(
    seconds: isize,
    negative: Color,
    positive: Color,
    neutral: Color,
) -> Color {
    if seconds < 0 {
        negative
    } else if seconds > 0 {
        positive
    } else {
        neutral
    }
}
