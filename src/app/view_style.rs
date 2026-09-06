use ratatui::{
    prelude::Span,
    style::{Color, Modifier, Style},
};

pub(super) fn report_period_label_span(
    label: &str,
    active: bool,
    foreground: Color,
    status: Color,
) -> Span<'static> {
    let style = if active {
        Style::default().fg(foreground).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(status).add_modifier(Modifier::DIM)
    };

    Span::styled(label.to_string(), style)
}

pub(super) fn text_color_for_bg(bg_color: Color) -> Color {
    if let Color::Rgb(r, g, b) = bg_color {
        let brightness = (299 * r as u32 + 587 * g as u32 + 114 * b as u32) / 1000;
        if brightness > 128 {
            Color::Black
        } else {
            Color::White
        }
    } else {
        Color::White
    }
}

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
