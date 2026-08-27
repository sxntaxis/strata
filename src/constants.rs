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

pub const CATCHUP_SETTINGS: CatchupSettings = CatchupSettings {
    cadence_ms: 120,
    accelerated_multiplier: 24,
    bounded_catchup_after_secs: 8,
    visual_refresh_ms: 120,
    gauge_hold_ms: 300,
    repose_threshold: 2,
    relax_passes: 3,
};

pub const RUNTIME_LOOP_SETTINGS: RuntimeLoopSettings = RuntimeLoopSettings {
    keymap_poll_ms: 300,
    autosave_secs: 60,
    input_poll_ms: 1,
};

pub const APP_LAYOUT_SETTINGS: AppLayoutSettings = AppLayoutSettings {
    frame_margin: 2,
    modal_min_height: 10,
    report_breathing_room: 5,
    catchup_progress_min_anchor_ms: 10,
    catchup_gauge_width_num: 2,
    catchup_gauge_width_den: 5,
    catchup_gauge_min_width: 12,
};

pub const COMMAND_PALETTE_SETTINGS: CommandPaletteSettings = CommandPaletteSettings {
    rect_width_num: 5,
    rect_width_den: 6,
    rect_height_num: 1,
    rect_height_den: 2,
    min_height: 8,
    hint_width_divisor: 3,
    score_prefix: 2,
    score_word_prefix: 6,
    score_contains_base: 12,
    score_typo_base: 24,
    score_typo_distance_weight: 4,
    score_subsequence_base: 60,
};

pub const REPORT_MODAL_SETTINGS: ReportModalSettings = ReportModalSettings {
    log_detail_fallback_width: 16,
    log_detail_max_width: 40,
    summary_name_fallback_width: 12,
    summary_name_max_width: 28,
    expanded_inner_padding: 4,
    detail_metric_width_drift: 8,
    detail_metric_width_default: 9,
    summary_metric_width: 9,
    detail_time_width: 17,
    detail_date_width: 6,
    detail_date_preview_width: 7,
    min_tag_width: 4,
    summary_name_gap: 4,
    range_editor_min_width: 64,
    missed_activity_editor_min_width: 96,
};

pub const ATLAS_LAYOUT_SETTINGS: AtlasLayoutSettings = AtlasLayoutSettings {
    value_col_width: 30,
    action_col_width: 24,
};

pub const CATEGORY_SETTINGS: CategorySettings = CategorySettings {
    max_tags_per_category: 24,
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

pub struct CatchupSettings {
    pub cadence_ms: u64,
    pub accelerated_multiplier: u32,
    pub bounded_catchup_after_secs: u64,
    pub visual_refresh_ms: u64,
    pub gauge_hold_ms: u64,
    pub repose_threshold: usize,
    pub relax_passes: usize,
}

pub struct RuntimeLoopSettings {
    pub keymap_poll_ms: u64,
    pub autosave_secs: u64,
    pub input_poll_ms: u64,
}

pub struct AppLayoutSettings {
    pub frame_margin: u16,
    pub modal_min_height: u16,
    pub report_breathing_room: usize,
    pub catchup_progress_min_anchor_ms: u64,
    pub catchup_gauge_width_num: u16,
    pub catchup_gauge_width_den: u16,
    pub catchup_gauge_min_width: u16,
}

pub struct CommandPaletteSettings {
    pub rect_width_num: u16,
    pub rect_width_den: u16,
    pub rect_height_num: u16,
    pub rect_height_den: u16,
    pub min_height: u16,
    pub hint_width_divisor: usize,
    pub score_prefix: usize,
    pub score_word_prefix: usize,
    pub score_contains_base: usize,
    pub score_typo_base: usize,
    pub score_typo_distance_weight: usize,
    pub score_subsequence_base: usize,
}

pub struct ReportModalSettings {
    pub log_detail_fallback_width: usize,
    pub log_detail_max_width: usize,
    pub summary_name_fallback_width: usize,
    pub summary_name_max_width: usize,
    pub expanded_inner_padding: usize,
    pub detail_metric_width_drift: usize,
    pub detail_metric_width_default: usize,
    pub summary_metric_width: usize,
    pub detail_time_width: usize,
    pub detail_date_width: usize,
    pub detail_date_preview_width: usize,
    pub min_tag_width: usize,
    pub summary_name_gap: usize,
    pub range_editor_min_width: usize,
    pub missed_activity_editor_min_width: usize,
}

pub struct AtlasLayoutSettings {
    pub value_col_width: usize,
    pub action_col_width: usize,
}

pub struct CategorySettings {
    pub max_tags_per_category: usize,
}
