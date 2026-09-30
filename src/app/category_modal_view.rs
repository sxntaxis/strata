use ratatui::prelude::{Line, Span};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState},
};

use crate::constants::APP_LAYOUT_SETTINGS;

use super::{App, overlay_layout};

impl App {
    pub(super) fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let categories = self.categories_for_render();
        let minimum_content_height =
            categories.len().saturating_add(1).min(u16::MAX as usize) as u16;
        let category_width = categories
            .iter()
            .map(|category| {
                2usize.saturating_add(self.display_layer_name(&category.name).chars().count())
            })
            .max()
            .unwrap_or(0);
        let insert_label_width = if self.new_category_name.is_empty() {
            "+ Forge new layer...".chars().count()
        } else {
            self.new_category_name.chars().count().saturating_add(1)
        };
        let insert_width = 2usize.saturating_add(insert_label_width);
        let selected_width = categories
            .get(self.selected_index)
            .map(|category| {
                let layer_name = if self.modal_renaming_category {
                    self.modal_category_name_draft.as_str()
                } else {
                    category.name.as_str()
                };
                let layer_width = 2usize.saturating_add(layer_name.chars().count());
                if self.modal_renaming_category {
                    layer_width.saturating_add(1)
                } else if let Some(completion) =
                    self.tag_completion_for_category(category.id, &self.modal_description)
                {
                    let suffix_width = completion.tag.chars().skip(completion.typed_chars).count();
                    layer_width
                        .saturating_add(1)
                        .saturating_add(self.modal_description.chars().count())
                        .saturating_add(suffix_width)
                        .saturating_add(if self.modal_tag_text_editing { 1 } else { 0 })
                } else if self.modal_description.is_empty() {
                    layer_width.saturating_add(if self.modal_tag_text_editing { 2 } else { 0 })
                } else {
                    self.modal_description
                        .chars()
                        .count()
                        .saturating_add(layer_width)
                        .saturating_add(1)
                        .saturating_add(if self.modal_tag_text_editing { 1 } else { 0 })
                }
            })
            .unwrap_or(0);
        let minimum_content_width = category_width
            .max(insert_width)
            .max(selected_width)
            .min(u16::MAX as usize) as u16;
        let minimum_inner_width = minimum_content_width.saturating_add(2);
        let minimum_inner_height = minimum_content_height.saturating_add(2);
        let modal_rect = overlay_layout::centered_overlay_rect(
            terminal_size,
            minimum_inner_width,
            minimum_inner_height,
            1,
            3,
            APP_LAYOUT_SETTINGS.frame_margin,
        );

        let border_color = self.get_selected_color();

        let items: Vec<ListItem> = categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;
                let dot = if cat.balance_effect < 0 {
                    "◯ "
                } else {
                    "● "
                };

                if is_selected {
                    let selection_background = cat.color;
                    let text_color = crate::appearance::contrasting_text_color(
                        selection_background,
                        self.theme_foreground(),
                    );
                    let input_style = Style::default()
                        .fg(self.theme_selection_text())
                        .bg(self.theme_selection());
                    let layer_name = if self.modal_renaming_category {
                        self.modal_category_name_draft.clone()
                    } else {
                        self.display_layer_name(&cat.name)
                    };
                    let description_style = Style::default()
                        .fg(text_color)
                        .add_modifier(ratatui::style::Modifier::ITALIC);
                    let mut line_spans = vec![Span::raw(dot).fg(text_color)];
                    if self.modal_renaming_category {
                        line_spans.push(Span::styled(format!("{layer_name} "), input_style));
                    } else {
                        line_spans.push(Span::raw(layer_name).fg(text_color));
                        if let Some(completion) =
                            self.tag_completion_for_category(cat.id, &self.modal_description)
                        {
                            let suffix = completion
                                .tag
                                .chars()
                                .skip(completion.typed_chars)
                                .collect::<String>();
                            if self.modal_tag_text_editing {
                                line_spans.push(Span::raw(" ").fg(text_color));
                                line_spans.push(Span::styled(
                                    self.modal_description.clone(),
                                    input_style,
                                ));
                                line_spans.push(Span::styled(
                                    suffix,
                                    input_style.add_modifier(ratatui::style::Modifier::DIM),
                                ));
                                line_spans.push(Span::styled(" ", input_style));
                            } else {
                                line_spans.push(Span::styled(
                                    format!(" {}", self.modal_description),
                                    description_style,
                                ));
                                line_spans.push(Span::styled(
                                    suffix,
                                    description_style.add_modifier(ratatui::style::Modifier::DIM),
                                ));
                            }
                        } else if !self.modal_description.is_empty() {
                            if self.modal_tag_text_editing {
                                line_spans.push(Span::raw(" ").fg(text_color));
                                line_spans.push(Span::styled(
                                    format!("{} ", self.modal_description),
                                    input_style,
                                ));
                            } else {
                                line_spans.push(Span::styled(
                                    format!(" {}", self.modal_description),
                                    description_style,
                                ));
                            }
                        } else if self.modal_tag_text_editing {
                            line_spans.push(Span::raw(" ").fg(text_color));
                            line_spans.push(Span::styled(" ", input_style));
                        }
                    }
                    ListItem::new(Line::from(line_spans))
                        .style(Style::default().fg(text_color).bg(selection_background))
                } else {
                    let layer_name = self.display_layer_name(&cat.name);
                    ListItem::new(Line::from(vec![
                        Span::raw(dot).fg(cat.color),
                        Span::raw(layer_name).fg(self.theme_foreground()),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = self
                    .appearance
                    .sand_color_at(self.new_category_color_cursor);

                if is_selected {
                    let text_color = crate::appearance::contrasting_text_color(
                        cycling_color,
                        self.theme_foreground(),
                    );
                    let input_style = Style::default()
                        .fg(self.theme_selection_text())
                        .bg(self.theme_selection());
                    let input = if self.new_category_name.is_empty() {
                        "+ Forge new layer...".to_string()
                    } else {
                        format!("{} ", self.new_category_name)
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(text_color),
                        Span::styled(input, input_style),
                    ]))
                    .style(Style::default().fg(text_color).bg(cycling_color))
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Forge new layer..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(self.theme_foreground()),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .style(Style::default().bg(self.theme_background()))
            .highlight_style(Style::default());
        let frame_block = Block::default()
            .style(Style::default().bg(self.theme_background()))
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(Line::from(Span::styled(
                "Strata",
                Style::default().fg(self.theme_foreground()),
            )))
            .title_alignment(ratatui::layout::Alignment::Center)
            .border_style(Style::default().fg(border_color));
        let inner = frame_block.inner(modal_rect);
        let list_area = overlay_layout::modal_content_rect(inner, minimum_content_height, true);

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_widget(frame_block, modal_rect);
        f.render_stateful_widget(list, list_area, &mut list_state);

        let visible_row = self.selected_index.saturating_sub(list_state.offset());
        if visible_row < usize::from(list_area.height) {
            let cursor_y = list_area.y.saturating_add(visible_row as u16);
            let cursor_x = if self.modal_renaming_category {
                Some(
                    list_area
                        .x
                        .saturating_add(2)
                        .saturating_add(self.modal_category_name_draft.chars().count() as u16),
                )
            } else if self.is_on_insert_space() {
                Some(
                    list_area
                        .x
                        .saturating_add(2)
                        .saturating_add(self.new_category_name.chars().count() as u16),
                )
            } else if self.modal_tag_text_editing
                && let Some(category) = categories.get(self.selected_index)
            {
                Some(
                    list_area
                        .x
                        .saturating_add(2)
                        .saturating_add(self.display_layer_name(&category.name).chars().count() as u16)
                        .saturating_add(1)
                        .saturating_add(self.modal_description.chars().count() as u16),
                )
            } else {
                None
            };
            if let Some(cursor_x) = cursor_x
                && cursor_x < list_area.x.saturating_add(list_area.width)
            {
                #[allow(deprecated)]
                f.set_cursor(cursor_x, cursor_y);
            }
        }
    }
}
