use std::cmp::Ordering;

use ratatui::style::Color;

use crate::constants::COLORS;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ThemeSwatch {
    pub(super) key: String,
    pub(super) color: Color,
    pub(super) l: f64,
    pub(super) a: f64,
    pub(super) b: f64,
    pub(super) c: f64,
    pub(super) h: f64,
}

impl ThemeSwatch {
    pub(super) fn new(key: String, color: Color) -> Result<Self, String> {
        let (r, g, b) = rgb_tuple(color).ok_or_else(|| "theme swatch must be RGB".to_string())?;
        let (l, a, lab_b) = srgb_to_oklab(r, g, b);
        let c = (a * a + lab_b * lab_b).sqrt();
        let mut h = lab_b.atan2(a).to_degrees();
        if h < 0.0 {
            h += 360.0;
        }
        Ok(Self {
            key,
            color,
            l,
            a,
            b: lab_b,
            c,
            h,
        })
    }

    pub(super) fn hue_order(left: &Self, right: &Self) -> Ordering {
        const NEUTRAL_CHROMA: f64 = 0.02;
        let left_neutral = left.c < NEUTRAL_CHROMA;
        let right_neutral = right.c < NEUTRAL_CHROMA;
        match (left_neutral, right_neutral) {
            (false, true) => Ordering::Less,
            (true, false) => Ordering::Greater,
            (true, true) => left
                .l
                .total_cmp(&right.l)
                .then_with(|| left.key.cmp(&right.key)),
            (false, false) => left
                .h
                .total_cmp(&right.h)
                .then_with(|| left.l.total_cmp(&right.l))
                .then_with(|| left.c.total_cmp(&right.c))
                .then_with(|| left.key.cmp(&right.key)),
        }
    }
}

pub(super) fn nearest_swatch(wheel: &[ThemeSwatch], anchor: Color) -> Option<&ThemeSwatch> {
    nearest_swatch_index(wheel, anchor).and_then(|index| wheel.get(index))
}

pub(super) fn nearest_swatch_index(wheel: &[ThemeSwatch], anchor: Color) -> Option<usize> {
    let (r, g, b) = rgb_tuple(anchor)?;
    let target = srgb_to_oklab(r, g, b);
    wheel
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            color_distance(target, (left.l, left.a, left.b))
                .total_cmp(&color_distance(target, (right.l, right.a, right.b)))
        })
        .map(|(index, _)| index)
}

pub(crate) fn nearest_legacy_color_index(anchor: Color) -> usize {
    let target = rgb_tuple(anchor).map(|(r, g, b)| srgb_to_oklab(r, g, b));
    let Some(target) = target else {
        return 0;
    };
    COLORS
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            color_distance(target, color_oklab(**left))
                .total_cmp(&color_distance(target, color_oklab(**right)))
        })
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn color_oklab(color: Color) -> (f64, f64, f64) {
    let (r, g, b) = rgb_tuple(color).unwrap_or((255, 255, 255));
    srgb_to_oklab(r, g, b)
}

fn color_distance(left: (f64, f64, f64), right: (f64, f64, f64)) -> f64 {
    let dl = left.0 - right.0;
    let da = left.1 - right.1;
    let db = left.2 - right.2;
    dl * dl + da * da + db * db
}

pub(super) fn rgb_tuple(color: Color) -> Option<(u8, u8, u8)> {
    match color {
        Color::Rgb(r, g, b) => Some((r, g, b)),
        Color::Black => Some((0, 0, 0)),
        Color::Red => Some((255, 0, 0)),
        Color::Green => Some((0, 128, 0)),
        Color::Yellow => Some((255, 255, 0)),
        Color::Blue => Some((0, 0, 255)),
        Color::Magenta => Some((255, 0, 255)),
        Color::Cyan => Some((0, 255, 255)),
        Color::Gray => Some((128, 128, 128)),
        Color::DarkGray => Some((85, 85, 85)),
        Color::LightRed => Some((255, 85, 85)),
        Color::LightGreen => Some((85, 255, 85)),
        Color::LightYellow => Some((255, 255, 85)),
        Color::LightBlue => Some((85, 85, 255)),
        Color::LightMagenta => Some((255, 85, 255)),
        Color::LightCyan => Some((85, 255, 255)),
        Color::White => Some((255, 255, 255)),
        Color::Reset | Color::Indexed(_) => None,
    }
}

fn srgb_to_oklab(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    fn linear(channel: u8) -> f64 {
        let value = f64::from(channel) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    }
    let r = linear(r);
    let g = linear(g);
    let b = linear(b);
    let l = (0.412_221_46 * r + 0.536_332_55 * g + 0.051_445_995 * b).cbrt();
    let m = (0.211_903_5 * r + 0.680_699_5 * g + 0.107_396_96 * b).cbrt();
    let s = (0.088_302_46 * r + 0.281_718_85 * g + 0.629_978_7 * b).cbrt();
    (
        0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
        1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
        0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
    )
}
