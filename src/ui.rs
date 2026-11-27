use macroquad::prelude::*;

use crate::{
    app::AppState,
    experience::{Experience, ScreenState},
    scaling::Scaling,
};

pub(crate) const PANEL: Color = Color::from_rgba(27, 29, 31, 255);
pub(crate) const BAR: Color = Color::from_rgba(22, 24, 26, 255);
pub(crate) const HOVER: Color = Color::from_rgba(54, 57, 60, 255);
pub(crate) const BORDER: Color = Color::from_rgba(91, 94, 98, 255);
pub(crate) const TEXT: Color = Color::from_rgba(239, 239, 237, 255);
pub(crate) const MUTED: Color = Color::from_rgba(187, 190, 193, 255);
pub(crate) const ACCENT: Color = Color::from_rgba(245, 136, 124, 255);
const FONT_SIZE: u16 = 48;

pub(crate) fn text_width(text: &str, font: &Font, size: f32) -> f32 {
    measure_text(text, Some(font), FONT_SIZE, size / FONT_SIZE as f32).width
}

pub(crate) fn text(text: &str, x: f32, y: f32, size: f32, color: Color, font: &Font) {
    draw_text_ex(
        text,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: FONT_SIZE,
            font_scale: size / FONT_SIZE as f32,
            color,
            ..Default::default()
        },
    );
}

pub(crate) fn right_text(text: &str, right: f32, y: f32, size: f32, color: Color, font: &Font) {
    self::text(
        text,
        right - text_width(text, font, size),
        y,
        size,
        color,
        font,
    );
}

pub(crate) fn panel(rect: Rect, border_width: f32) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, PANEL);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, border_width, BORDER);
}

pub(crate) fn highlight(rect: Rect, accent_width: f32) {
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, HOVER);
    draw_rectangle(rect.x, rect.y, accent_width, rect.h, ACCENT);
}

pub enum SettingsAction {
    None,
    Reset,
    PowerCycle,
}

const SETTINGS_COUNT: usize = 11;
const SCANLINE_STRENGTH_SETTING: usize = 2;

pub fn settings_input(state: &mut AppState, experience: &mut Experience) -> SettingsAction {
    if is_key_pressed(KeyCode::Up) {
        state.settings_selection = (state.settings_selection + SETTINGS_COUNT - 1) % SETTINGS_COUNT;
    }
    if is_key_pressed(KeyCode::Down) {
        state.settings_selection = (state.settings_selection + 1) % SETTINGS_COUNT;
    }
    if !(is_key_pressed(KeyCode::Enter)
        || is_key_pressed(KeyCode::Space)
        || is_key_pressed(KeyCode::Left)
        || is_key_pressed(KeyCode::Right))
    {
        return SettingsAction::None;
    }
    if state.settings_selection == SCANLINE_STRENGTH_SETTING {
        adjust_scanline_strength(state, !is_key_pressed(KeyCode::Left));
        return SettingsAction::None;
    }
    activate_setting(state.settings_selection, state, experience)
}

fn adjust_scanline_strength(state: &mut AppState, increase: bool) {
    let current = (state.scanline_strength.clamp(0.0, 1.0) * 10.0).round() as i32;
    let step = if increase { 1 } else { -1 };
    state.scanline_strength = (current + step).clamp(0, 10) as f32 / 10.0;
}

pub fn activate_setting(
    index: usize,
    state: &mut AppState,
    experience: &mut Experience,
) -> SettingsAction {
    match index {
        0 => {
            experience.settings.enabled = !experience.settings.enabled;
            if !experience.settings.enabled && experience.screen == ScreenState::Booting {
                experience.reset();
            }
        }
        1 => state.scanlines_enabled = !state.scanlines_enabled,
        3 => state.crt_reflections_enabled = !state.crt_reflections_enabled,
        4 => state.crt_curvature_enabled = !state.crt_curvature_enabled,
        5 => state.bloom_enabled = !state.bloom_enabled,
        6 => state.glow_enabled = !state.glow_enabled,
        7 => state.flicker_enabled = !state.flicker_enabled,
        8 => state.fullscreen_confirmation = !state.fullscreen_confirmation,
        9 => return SettingsAction::Reset,
        10 => return SettingsAction::PowerCycle,
        _ => {}
    }
    SettingsAction::None
}

pub fn draw_settings(
    scaling: &Scaling,
    state: &AppState,
    experience: &Experience,
    font_face: &Font,
) {
    let panel = scaling.rect(Rect::new(470.0, 100.0, 660.0, 700.0));
    self::panel(panel, scaling.scale.max(1.0));
    let x = panel.x + 35.0 * scaling.scale;
    let mut y = panel.y + 48.0 * scaling.scale;
    let font = 26.0 * scaling.scale;
    text("ZX81 SETTINGS", x, y, font * 1.2, TEXT, font_face);
    draw_line(
        x,
        y + 13.0 * scaling.scale,
        panel.x + panel.w - 35.0 * scaling.scale,
        y + 13.0 * scaling.scale,
        scaling.scale.max(1.0),
        ACCENT,
    );
    y += 40.0 * scaling.scale;
    let rows = [
        ("DISPLAY", None),
        ("CRT WARM-UP", Some(experience.settings.enabled)),
        ("SCANLINES", Some(state.scanlines_enabled)),
        ("SCANLINE STRENGTH", None),
        ("REFLECTIONS", Some(state.crt_reflections_enabled)),
        ("CURVATURE", Some(state.crt_curvature_enabled)),
        ("BLOOM", Some(state.bloom_enabled)),
        ("PHOSPHOR GLOW", Some(state.glow_enabled)),
        ("FLICKER", Some(state.flicker_enabled)),
        ("FULLSCREEN CONFIRM", Some(state.fullscreen_confirmation)),
        ("SYSTEM", None),
        ("RESET", None),
        ("POWER CYCLE", None),
    ];
    let mut selection = 0;
    for (row, value) in rows {
        let heading = matches!(row, "DISPLAY" | "SYSTEM");
        let selected = !heading && selection == state.settings_selection;
        if selected {
            highlight(
                Rect::new(
                    x - 9.0 * scaling.scale,
                    y - 22.0 * scaling.scale,
                    panel.w - 52.0 * scaling.scale,
                    32.0 * scaling.scale,
                ),
                (3.0 * scaling.scale).max(1.0),
            );
        }
        let color = if heading { MUTED } else { TEXT };
        text(row, x, y, font, color, font_face);
        if let Some(value) = value {
            right_text(
                if value { "ON" } else { "OFF" },
                panel.x + panel.w - 35.0 * scaling.scale,
                y,
                font,
                if value { ACCENT } else { MUTED },
                font_face,
            );
        } else if row == "SCANLINE STRENGTH" {
            right_text(
                &format!("{:.0}%", state.scanline_strength * 100.0),
                panel.x + panel.w - 35.0 * scaling.scale,
                y,
                font,
                color,
                font_face,
            );
        }
        if !heading {
            selection += 1;
        }
        y += 40.0 * scaling.scale;
    }
    text(
        "UP/DOWN SELECT   ENTER CHANGE   LEFT/RIGHT ADJUST   ESC CLOSE",
        x,
        panel.y + panel.h - 28.0 * scaling.scale,
        font * 0.65,
        MUTED,
        font_face,
    );
}

pub fn draw_fullscreen_confirmation(target_fullscreen: bool, selection: usize, font: &Font) {
    let width = screen_width();
    let height = screen_height();
    let panel = Rect::new(width * 0.5 - 210.0, height * 0.5 - 90.0, 420.0, 180.0);

    draw_rectangle(0.0, 0.0, width, height, Color::from_rgba(0, 0, 0, 170));
    self::panel(panel, 1.0);
    draw_rectangle(panel.x, panel.y, panel.w, 3.0, ACCENT);

    let action = if target_fullscreen { "enter" } else { "leave" };
    text(
        &format!("CONFIRM FULLSCREEN - {action}"),
        panel.x + 28.0,
        panel.y + 48.0,
        22.0,
        TEXT,
        font,
    );

    let yes = Rect::new(panel.x + 55.0, panel.y + 115.0, 130.0, 42.0);
    let no = Rect::new(panel.x + 235.0, panel.y + 115.0, 130.0, 42.0);
    for (rect, label, selected) in [(yes, "YES", selection == 0), (no, "NO", selection == 1)] {
        let hovered = rect.contains(mouse_position().into());
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if selected || hovered { HOVER } else { BAR },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            if selected { ACCENT } else { BORDER },
        );
        text(
            label,
            rect.x + (rect.w - text_width(label, font, 20.0)) / 2.0,
            rect.y + 29.0,
            20.0,
            TEXT,
            font,
        );
    }
}

pub fn draw_status(state: &AppState, experience: &Experience, font: &Font) {
    let width = screen_width().min(250.0);
    let r = Rect::new(
        screen_width() - width,
        screen_height() - 112.0,
        width,
        112.0,
    );
    panel(r, 1.0);
    draw_rectangle(r.x, r.y, 3.0, r.h, ACCENT);
    let power = if matches!(
        experience.screen,
        ScreenState::PowerOff | ScreenState::Shutdown
    ) {
        "OFF"
    } else {
        "ON"
    };
    let crt = if experience.screen == ScreenState::Booting {
        "WARMING"
    } else if power == "OFF" {
        "OFF"
    } else {
        "READY"
    };
    let program = if state.program_loading {
        "LOADING"
    } else {
        "READY"
    };
    let size = 18.0;
    for (index, (label, value)) in [("POWER", power), ("CRT", crt), ("PROGRAM", program)]
        .iter()
        .enumerate()
    {
        text(
            label,
            r.x + 12.0,
            r.y + (index as f32 + 1.0) * 29.0,
            size,
            MUTED,
            font,
        );
        right_text(
            value,
            r.x + r.w - 12.0,
            r.y + (index as f32 + 1.0) * 29.0,
            size,
            TEXT,
            font,
        );
    }
}

pub fn draw_notification(message: &str, font: &Font) {
    let size = (screen_height() * 0.025).clamp(14.0, 24.0);
    let rect = Rect::new(
        8.0,
        screen_height() - size * 2.0,
        (screen_width() - 16.0).max(0.0),
        size * 1.6,
    );
    panel(rect, 1.0);
    draw_rectangle(rect.x, rect.y, 3.0, rect.h, ACCENT);
    text(
        message,
        16.0,
        screen_height() - size * 0.6,
        size,
        TEXT,
        font,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_text_has_readable_contrast_on_panel_and_focus() {
        fn luminance(color: Color) -> f32 {
            fn linear(value: f32) -> f32 {
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            }
            0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
        }
        for background in [PANEL, BAR, HOVER] {
            for foreground in [TEXT, MUTED, ACCENT] {
                let contrast = (luminance(foreground) + 0.05) / (luminance(background) + 0.05);
                assert!(contrast >= 4.5, "contrast ratio {contrast}");
            }
        }
    }

    #[test]
    fn scanline_strength_adjustment_stays_within_zero_to_one() {
        let mut state = AppState::default();
        for _ in 0..12 {
            adjust_scanline_strength(&mut state, false);
        }
        assert_eq!(state.scanline_strength, 0.0);

        for _ in 0..12 {
            adjust_scanline_strength(&mut state, true);
        }
        assert_eq!(state.scanline_strength, 1.0);
    }

    #[test]
    fn settings_indices_match_the_displayed_rows() {
        let mut state = AppState::default();
        let mut experience = Experience::default();

        activate_setting(3, &mut state, &mut experience);
        activate_setting(4, &mut state, &mut experience);
        activate_setting(8, &mut state, &mut experience);
        assert!(!state.crt_reflections_enabled);
        assert!(state.crt_curvature_enabled);
        assert!(!state.fullscreen_confirmation);
        assert!(matches!(
            activate_setting(9, &mut state, &mut experience),
            SettingsAction::Reset
        ));
        assert!(matches!(
            activate_setting(10, &mut state, &mut experience),
            SettingsAction::PowerCycle
        ));
    }
}
