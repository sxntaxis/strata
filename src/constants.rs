use ratatui::style::Color;

pub const COLORS: [Color; 12] = [
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

pub const TIME_SETTINGS: TimeSettings = TimeSettings {
    tick_ms: 1000,
    physics_ms: 32,
    target_fps: 24,
};

pub const SAND_ENGINE: SandEngineSettings = SandEngineSettings {
    braille_base: 0x2800,
    dot_height: 4,
    dot_width: 2,
};

pub const BLINK_SETTINGS: BlinkSettings = BlinkSettings {
    interval_min_frames: 150,
    interval_max_frames: 300,
    duration_min_frames: 10,
    duration_max_frames: 17,
};

pub const FACE_SETTINGS: FaceSettings = FaceSettings {
    thresholds: &[120, 300, 600, 1200, 2400, 3600, 5400],
    faces: &[
        "(o_o)",
        "(¬_¬)",
        "(O_O)",
        "(⊙_⊙)",
        "(ಠ_ಠ)",
        "(ಥ_ಥ)",
        "(T_T)",
        "(x_x)",
    ],
};

pub struct TimeSettings {
    pub tick_ms: u64,
    pub physics_ms: u64,
    pub target_fps: u64,
}

pub struct SandEngineSettings {
    pub braille_base: u32,
    pub dot_height: usize,
    pub dot_width: usize,
}

pub struct BlinkSettings {
    pub interval_min_frames: i32,
    pub interval_max_frames: i32,
    pub duration_min_frames: i32,
    pub duration_max_frames: i32,
}

pub struct FaceSettings {
    pub thresholds: &'static [usize],
    pub faces: &'static [&'static str],
}
