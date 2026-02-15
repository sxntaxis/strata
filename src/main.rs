use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};

use rand::Rng;

use ratatui::{
    backend::CrosstermBackend,
    layout::Rect,
    widgets::Paragraph,
    Terminal,
};

struct App {
    grid: Vec<Vec<bool>>,
    width: u16,
    height: u16, // logical (braille scaled)
    total_grains: usize,     // authoritative mass
    overflow_grains: usize,  // grains that don't fit visually
}

impl App {
    fn new(width: u16, height: u16) -> Self {
        let mut app = Self {
            grid: vec![],
            width,
            height,
            total_grains: 0,
            overflow_grains: 0,
        };
        app.resize(width, height);
        app
    }

    fn capacity(&self) -> usize {
        self.grid.len() * self.grid[0].len()
    }

    fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height * 4;

        let new_w = width as usize * 2;
        let new_h = self.height as usize;

        self.grid = vec![vec![false; new_w]; new_h];

        self.rebuild_from_mass();
    }

    fn spawn(&mut self) {
        self.total_grains += 1;
    }

    fn rebuild_from_mass(&mut self) {
        let capacity = self.capacity();

        let visible = self.total_grains.min(capacity);
        self.overflow_grains = self.total_grains.saturating_sub(capacity);

        // Clear grid
        for row in &mut self.grid {
            for cell in row {
                *cell = false;
            }
        }

        // Fill bottom-up proportionally
        let mut grains_left = visible;
        let h = self.grid.len();
        let w = self.grid[0].len();

        for y in (0..h).rev() {
            for x in 0..w {
                if grains_left > 0 {
                    self.grid[y][x] = true;
                    grains_left -= 1;
                } else {
                    break;
                }
            }
        }

        // Let gravity shape it naturally
        self.settle(60);
    }

    fn update(&mut self) {
        let capacity = self.capacity();

        if self.total_grains > capacity {
            self.overflow_grains = self.total_grains - capacity;
        } else {
            self.overflow_grains = 0;
        }

        // Add grain if space
        if self.total_grains - self.overflow_grains > 0 {
            let mut rng = rand::thread_rng();
            let w = self.grid[0].len();
            let x = rng.gen_range(0..w);

            if !self.grid[0][x] {
                self.grid[0][x] = true;
            }
        }

        self.apply_gravity();
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
                        if nx >= 0
                            && nx < w as isize
                            && !self.grid[y + 1][nx as usize]
                        {
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
}

fn main() -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let size = terminal.size()?;
    let mut app = App::new(size.width, size.height);

    let tick_rate = Duration::from_millis(30);
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|f| {
            let size = f.size();

            if size.width != app.width || size.height != app.height / 4 {
                app.resize(size.width, size.height);
            }

            let sand = app.render();
            let paragraph = Paragraph::new(sand);
            f.render_widget(paragraph, Rect::new(0, 0, size.width, size.height));
        })?;

        if event::poll(Duration::from_millis(1))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            app.spawn();
            app.update();
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
