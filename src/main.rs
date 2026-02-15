use std::{
    io,
    time::{Duration, Instant},
};

use chrono::{Duration as ChronoDuration, Local, TimeZone, Timelike};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use rand::Rng;

use ratatui::prelude::{Line, Span};
use ratatui::style::Stylize;
use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    style::Color,
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};

const COLORS: [Color; 12] = [
    Color::Rgb(0, 176, 80),
    Color::Rgb(128, 255, 0),
    Color::Rgb(255, 255, 0),
    Color::Rgb(255, 204, 0),
    Color::Rgb(255, 153, 0),
    Color::Rgb(255, 51, 0),
    Color::Rgb(255, 0, 0),
    Color::Rgb(153, 0, 255),
    Color::Rgb(102, 51, 255),
    Color::Rgb(0, 0, 255),
    Color::Rgb(0, 153, 255),
    Color::Rgb(0, 255, 255),
];

struct Category {
    name: String,
    color: Color,
}

struct App {
    grid: Vec<Vec<bool>>,
    width: u16,
    height: u16,
    total_mass: usize,
    overflow: usize,
    categories: Vec<Category>,
    active_category_index: Option<usize>,
    modal_open: bool,
    selected_index: usize,
    new_category_name: String,
    color_index: usize,
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut app = Self {
            grid: vec![],
            width,
            height,
            total_mass: 0,
            overflow: 0,
            categories: vec![Category {
                name: "none".to_string(),
                color: Color::White,
            }],
            active_category_index: None,
            modal_open: false,
            selected_index: 0,
            new_category_name: String::new(),
            color_index: 0,
        };

        app.resize(width, height);

        app.total_mass = Self::seconds_since_6am();
        app.rebuild_from_mass();

        app
    }

    fn capacity(&self) -> usize {
        self.grid.len() * self.grid[0].len()
    }

    fn seconds_since_6am() -> usize {
        let now = Local::now();

        let today_6am = now.date_naive().and_hms_opt(6, 0, 0).unwrap();

        let anchor = if now.hour() < 6 {
            today_6am - ChronoDuration::days(1)
        } else {
            today_6am
        };

        let anchor_dt = Local.from_local_datetime(&anchor).unwrap();

        now.signed_duration_since(anchor_dt).num_seconds().max(0) as usize
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height * 4;

        let new_w = width as usize * 2;
        let new_h = self.height as usize;

        self.grid = vec![vec![false; new_w]; new_h];
        self.rebuild_from_mass();
    }

    fn rebuild_from_mass(&mut self) {
        let capacity = self.capacity();

        let visible = self.total_mass.min(capacity);
        self.overflow = self.total_mass.saturating_sub(capacity);

        for row in &mut self.grid {
            for cell in row {
                *cell = false;
            }
        }

        let mut grains = visible;
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h).rev() {
            for x in 0..w {
                if grains > 0 {
                    self.grid[y][x] = true;
                    grains -= 1;
                }
            }
        }

        self.settle(80);
    }

    fn spawn_one(&mut self) {
        let capacity = self.capacity();

        if self.total_mass > capacity {
            self.overflow += 1;
            return;
        }

        let mut rng = rand::thread_rng();
        let w = self.grid[0].len();
        let x = rng.gen_range(0..w);

        if !self.grid[0][x] {
            self.grid[0][x] = true;
        } else {
            self.overflow += 1;
        }
    }

    fn apply_gravity(&mut self) {
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h - 1).rev() {
            for x in 0..w {
                if self.grid[y][x] {
                    if !self.grid[y + 1][x] {
                        self.grid[y][x] = false;
                        self.grid[y + 1][x] = true;
                    } else {
                        let dir = if rand::random() { 1 } else { -1 };
                        let nx = x as isize + dir;

                        if nx >= 0 && nx < w as isize && !self.grid[y + 1][nx as usize] {
                            self.grid[y][x] = false;
                            self.grid[y + 1][nx as usize] = true;
                        }
                    }
                }
            }
        }
    }

    fn settle(&mut self, passes: usize) {
        for _ in 0..passes {
            self.apply_gravity();
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();

        let cell_w = self.width as usize;
        let cell_h = (self.height / 4) as usize;

        for cy in 0..cell_h {
            for cx in 0..cell_w {
                let mut dots = 0u8;

                for dy in 0..4 {
                    for dx in 0..2 {
                        let gx = cx * 2 + dx;
                        let gy = cy * 4 + dy;

                        if gy < self.grid.len() && gx < self.grid[0].len() {
                            if self.grid[gy][gx] {
                                let dot_index = match (dx, dy) {
                                    (0, 0) => 0,
                                    (0, 1) => 1,
                                    (0, 2) => 2,
                                    (1, 0) => 3,
                                    (1, 1) => 4,
                                    (1, 2) => 5,
                                    (0, 3) => 6,
                                    (1, 3) => 7,
                                    _ => 0,
                                };
                                dots |= 1 << dot_index;
                            }
                        }
                    }
                }

                let ch = char::from_u32(0x2800 + dots as u32).unwrap_or(' ');
                output.push(ch);
            }
            output.push('\n');
        }

        output
    }

    fn open_modal(&mut self) {
        self.modal_open = true;
        self.selected_index = self.active_category_index.unwrap_or(0);
        self.new_category_name = String::new();
        self.color_index = 0;
    }

    fn close_modal(&mut self) {
        self.modal_open = false;
    }

    fn is_on_insert_space(&self) -> bool {
        self.selected_index == self.categories.len()
    }

    fn add_category(&mut self) {
        if !self.new_category_name.is_empty() {
            let color = COLORS[self.color_index];
            let index = self.categories.len();
            self.categories.push(Category {
                name: self.new_category_name.clone(),
                color,
            });
            self.active_category_index = Some(index);
        }
    }

    fn delete_category(&mut self) {
        if !self.is_on_insert_space()
            && self.selected_index < self.categories.len()
            && self.selected_index > 0
        {
            self.categories.remove(self.selected_index);
            if self.selected_index > 0 && self.selected_index >= self.categories.len() {
                self.selected_index = self.categories.len();
            }
        }
    }

    fn get_selected_color(&self) -> Color {
        if self.is_on_insert_space() {
            COLORS[self.color_index]
        } else if self.selected_index < self.categories.len() {
            self.categories[self.selected_index].color
        } else {
            Color::White
        }
    }

    fn get_active_color(&self) -> Color {
        if let Some(idx) = self.active_category_index {
            if idx < self.categories.len() {
                return self.categories[idx].color;
            }
        }
        Color::White
    }

    fn text_color_for_bg(bg_color: Color) -> Color {
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

    fn render_modal(&self, f: &mut Frame, terminal_size: Rect) {
        let modal_width = terminal_size.width / 3;
        let modal_height = (terminal_size.height / 3).max(10);

        let modal_x = (terminal_size.width - modal_width) / 2;
        let modal_y = (terminal_size.height - modal_height) / 2;

        let modal_rect = Rect::new(modal_x, modal_y, modal_width, modal_height);

        let border_color = self.get_selected_color();

        let items: Vec<ListItem> = self
            .categories
            .iter()
            .enumerate()
            .map(|(i, cat)| {
                let is_selected = i == self.selected_index;

                if is_selected {
                    let text_color = Self::text_color_for_bg(cat.color);
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cat.color),
                        Span::raw(&cat.name).fg(text_color),
                    ]))
                    .style(
                        ratatui::style::Style::default()
                            .fg(text_color)
                            .bg(cat.color),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cat.color),
                        Span::raw(&cat.name).fg(Color::White),
                    ]))
                }
            })
            .chain(std::iter::once({
                let is_selected = self.is_on_insert_space();
                let cycling_color = COLORS[self.color_index];

                if is_selected {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        }),
                    ]))
                    .style(
                        ratatui::style::Style::default()
                            .fg(Color::Black)
                            .bg(Color::White),
                    )
                } else {
                    ListItem::new(Line::from(vec![
                        Span::raw("● ").fg(cycling_color),
                        Span::raw(if self.new_category_name.is_empty() {
                            "+ Add new..."
                        } else {
                            &self.new_category_name
                        })
                        .fg(Color::White),
                    ]))
                }
            }))
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(self.selected_index));

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("strata")
                    .title_alignment(ratatui::layout::Alignment::Center)
                    .border_style(ratatui::style::Style::default().fg(border_color)),
            )
            .highlight_style(ratatui::style::Style::default());

        f.render_widget(ratatui::widgets::Clear, modal_rect);
        f.render_stateful_widget(list, modal_rect, &mut list_state);
    }
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let physics_rate = Duration::from_millis(16);
    let mut last_physics = Instant::now();

    loop {
        let expected = App::seconds_since_6am();
        while app.total_mass < expected {
            app.total_mass += 1;
            app.spawn_one();
        }

        if last_physics.elapsed() >= physics_rate {
            app.apply_gravity();
            app.apply_gravity();
            last_physics = Instant::now();
        }

        terminal.draw(|f| {
            let size = f.size();

            let inner_width = size.width.saturating_sub(2);
            let inner_height = size.height.saturating_sub(2);

            if app.width != inner_width || app.height / 4 != inner_height {
                app.resize(inner_width, inner_height);
            }

            let sand = app.render();
            let time = Local::now().format("%H:%M:%S").to_string();
            let border_color = app.get_active_color();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(time)
                .title_alignment(ratatui::layout::Alignment::Center)
                .border_style(ratatui::style::Style::default().fg(border_color));
            let paragraph = Paragraph::new(sand).block(block);
            f.render_widget(paragraph, size);

            if app.modal_open {
                app.render_modal(f, size);
            }
        })?;

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if app.modal_open {
                    match key.code {
                        KeyCode::Esc => {
                            app.close_modal();
                        }
                        KeyCode::Up => {
                            if app.selected_index > 0 {
                                app.selected_index -= 1;
                            }
                        }
                        KeyCode::Down => {
                            let max_index = app.categories.len();
                            if app.selected_index < max_index {
                                app.selected_index += 1;
                            }
                        }
                        KeyCode::Left => {
                            if app.is_on_insert_space() {
                                app.color_index =
                                    (app.color_index + COLORS.len() - 1) % COLORS.len();
                            }
                        }
                        KeyCode::Right => {
                            if app.is_on_insert_space() {
                                app.color_index = (app.color_index + 1) % COLORS.len();
                            }
                        }
                        KeyCode::Enter => {
                            if app.is_on_insert_space() {
                                if !app.new_category_name.is_empty() {
                                    app.add_category();
                                    app.close_modal();
                                }
                            } else {
                                app.active_category_index = Some(app.selected_index);
                                app.close_modal();
                            }
                        }
                        KeyCode::Char('x') => {
                            if !app.is_on_insert_space() {
                                app.delete_category();
                            }
                        }
                        KeyCode::Char(c) => {
                            if app.is_on_insert_space() {
                                app.new_category_name.push(c);
                            }
                        }
                        KeyCode::Backspace => {
                            if app.is_on_insert_space() {
                                app.new_category_name.pop();
                            }
                        }
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Char('q') => {
                            break;
                        }
                        KeyCode::Enter => {
                            app.open_modal();
                        }
                        KeyCode::Esc => {
                            app.active_category_index = None;
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
