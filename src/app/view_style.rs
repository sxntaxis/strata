use ratatui::{
    prelude::Span,
    style::{Color, Modifier, Style},
};

pub(super) fn report_period_label_span(label: &str, active: bool) -> Span<'static> {
    let style = if active {
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
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

pub(super) fn karma_color(seconds: isize) -> Color {
    if seconds < 0 {
        Color::Red
    } else if seconds > 0 {
        Color::Green
    } else {
        Color::Gray
    }
}
