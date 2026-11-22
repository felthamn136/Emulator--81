use macroquad::prelude::*;

use crate::{
    app::AppState,
    experience::{Experience, ScreenState},
    scaling::Scaling,
};

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

pub fn draw_settings(scaling: &Scaling, state: &AppState, experience: &Experience) {
    let panel = scaling.rect(Rect::new(470.0, 100.0, 660.0, 700.0));
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(13, 14, 14, 250),
    );
    draw_rectangle_lines(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        3.0 * scaling.scale,
        GRAY,
    );
    let x = panel.x + 35.0 * scaling.scale;
    let mut y = panel.y + 48.0 * scaling.scale;
    let font = 26.0 * scaling.scale;
    draw_text("ZX81 SETTINGS", x, y, font * 1.2, WHITE);
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
            draw_rectangle(
                x - 9.0 * scaling.scale,
                y - 22.0 * scaling.scale,
                panel.w - 52.0 * scaling.scale,
                32.0 * scaling.scale,
                Color::from_rgba(70, 72, 70, 255),
            );
        }
        let color = if heading { GRAY } else { WHITE };
        draw_text(row, x, y, font, color);
        if let Some(value) = value {
            draw_text(
                if value { "ON" } else { "OFF" },
                panel.x + panel.w - 120.0 * scaling.scale,
                y,
                font,
                color,
            );
        } else if row == "SCANLINE STRENGTH" {
            draw_text(
                format!("{:.0}%", state.scanline_strength * 100.0),
                panel.x + panel.w - 120.0 * scaling.scale,
                y,
                font,
                color,
            );
        }
        if !heading {
            selection += 1;
        }
        y += 40.0 * scaling.scale;
    }
    draw_text(
        "UP/DOWN SELECT   ENTER CHANGE   LEFT/RIGHT ADJUST   ESC CLOSE",
        x,
        panel.y + panel.h - 28.0 * scaling.scale,
        font * 0.7,
        LIGHTGRAY,
    );
}

pub fn draw_fullscreen_confirmation(target_fullscreen: bool, selection: usize) {
    let width = screen_width();
    let height = screen_height();
    let panel = Rect::new(width * 0.5 - 210.0, height * 0.5 - 90.0, 420.0, 180.0);

    draw_rectangle(0.0, 0.0, width, height, Color::from_rgba(0, 0, 0, 170));
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(13, 14, 14, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 3.0, GRAY);

    let action = if target_fullscreen { "enter" } else { "leave" };
    draw_text(
        format!("CONFIRM FULLSCREEN - {action}"),
        panel.x + 28.0,
        panel.y + 48.0,
        22.0,
        WHITE,
    );

    let yes = Rect::new(panel.x + 55.0, panel.y + 115.0, 130.0, 42.0);
    let no = Rect::new(panel.x + 235.0, panel.y + 115.0, 130.0, 42.0);
    for (rect, label, selected) in [(yes, "YES", selection == 0), (no, "NO", selection == 1)] {
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if selected {
                DARKGRAY
            } else {
                Color::from_rgba(34, 36, 37, 255)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            2.0,
            if selected { WHITE } else { GRAY },
        );
        draw_text(label, rect.x + 45.0, rect.y + 29.0, 20.0, WHITE);
    }
}

pub fn draw_status(state: &AppState, experience: &Experience) {
    let width = screen_width().min(250.0);
    let r = Rect::new(
        screen_width() - width,
        screen_height() - 112.0,
        width,
        112.0,
    );
    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.85));
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
    for (index, line) in [
        format!("POWER: {power}"),
        format!("CRT: {crt}"),
        format!("PROGRAM: {program}"),
    ]
    .iter()
    .enumerate()
    {
        draw_text(
            line,
            r.x + 12.0,
            r.y + (index as f32 + 1.0) * 29.0,
            size,
            WHITE,
        );
    }
}

pub fn draw_notification(message: &str) {
    let size = (screen_height() * 0.025).clamp(14.0, 24.0);
    draw_rectangle(
        8.0,
        screen_height() - size * 2.0,
        screen_width() - 16.0,
        size * 1.6,
        BLACK,
    );
    draw_text(message, 16.0, screen_height() - size * 0.6, size, WHITE);
}

#[cfg(test)]
mod tests {
    use super::*;

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
