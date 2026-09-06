use super::flowviz_unit::UNIT_SEGMENT_STEP;

const LONG_DROP_THRESHOLD_DOTS: f32 = 4.0;
const LONG_DROP_BASE_FRAMES: f32 = 5.0;
const LONG_DROP_SQRT_FRAME_SCALE: f32 = 0.9;
const LONG_DROP_MAX_FRAMES: f32 = 18.0;

pub(super) fn unit_segment_progress_step(start_y: f32, target_y: f32, distance_aware: bool) -> f32 {
    if !distance_aware {
        return UNIT_SEGMENT_STEP;
    }
    let downward_drop = target_y - start_y;
    if downward_drop <= LONG_DROP_THRESHOLD_DOTS {
        return UNIT_SEGMENT_STEP;
    }
    let extra_frames = (downward_drop.sqrt() * LONG_DROP_SQRT_FRAME_SCALE).ceil();
    let frames = (LONG_DROP_BASE_FRAMES + extra_frames).min(LONG_DROP_MAX_FRAMES);
    1.0 / frames
}

pub(super) fn unit_segment_vertical_fraction(
    progress: f32,
    start_y: f32,
    target_y: f32,
    distance_aware: bool,
) -> f32 {
    if distance_aware && target_y - start_y > LONG_DROP_THRESHOLD_DOTS {
        progress * progress
    } else {
        progress
    }
}
