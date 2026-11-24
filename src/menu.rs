use macroquad::prelude::*;

use crate::{
    app::AppState, autoload::LoadMode, experience::Experience, hardware::ram_pack::WobbleSeverity,
    memory::RamSize,
};

pub const BAR_HEIGHT: f32 = 36.0;
const ROW_HEIGHT: f32 = 30.0;
const MENU_WIDTH: f32 = 250.0;
const TABS: [(&str, f32); 6] = [
    ("File", 75.0),
    ("Settings", 105.0),
    ("Machine", 105.0),
    ("View", 75.0),
    ("Hardware", 110.0),
    ("Help", 75.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Open,
    OpenWav,
    OpenInstant,
    TapeSound,
    BrowseGamesLibrary(LoadMode),
    Save,
    SaveWav,
    Exit,
    Screenshot,
    Settings,
    Setting(usize),
    Pause,
    Step,
    Reset,
    PowerCycle,
    Fullscreen,
    VirtualKeyboard,
    Status,
    Debug,
    About,
    RamSize(RamSize),
    Wobble,
    WobbleSeverity(WobbleSeverity),
    TriggerWobble,
}

const FILE: &[(&str, Action)] = &[
    ("Load Program (.p)", Action::Open),
    ("Load WAV Tape...", Action::OpenWav),
    ("Open File Instantly", Action::OpenInstant),
    (
        "Games Library (Standard Load)",
        Action::BrowseGamesLibrary(LoadMode::Standard),
    ),
    (
        "Games Library (Instant Load)",
        Action::BrowseGamesLibrary(LoadMode::Instant),
    ),
    ("Save Program (.p)", Action::Save),
    ("Save Program as WAV...", Action::SaveWav),
    ("Save CRT Screenshot", Action::Screenshot),
    ("Exit", Action::Exit),
];
const SETTINGS: &[(&str, Action)] = &[
    ("Settings panel", Action::Settings),
    ("Fullscreen", Action::Fullscreen),
    ("Fullscreen confirmation", Action::Setting(8)),
    ("CRT warm-up", Action::Setting(0)),
    ("Scanlines", Action::Setting(1)),
    ("Reflections", Action::Setting(3)),
    ("Curvature", Action::Setting(4)),
    ("Bloom", Action::Setting(5)),
    ("Phosphor glow", Action::Setting(6)),
    ("Flicker", Action::Setting(7)),
    ("Tape loading sound", Action::TapeSound),
];
const MACHINE: &[(&str, Action)] = &[
    ("Pause / Resume", Action::Pause),
    ("Single step", Action::Step),
    ("Reset", Action::Reset),
    ("Power cycle", Action::PowerCycle),
    ("1K RAM", Action::RamSize(RamSize::OneK)),
    ("16K RAM", Action::RamSize(RamSize::SixteenK)),
];
const VIEW: &[(&str, Action)] = &[
    ("Fullscreen", Action::Fullscreen),
    ("Virtual keyboard", Action::VirtualKeyboard),
    ("Status panel", Action::Status),
    ("Debug overlay", Action::Debug),
];
const HELP: &[(&str, Action)] = &[("About Emulator-81", Action::About)];
const HARDWARE: &[(&str, Action)] = &[
    ("RAM Pack Wobble", Action::Wobble),
    (
        "Severity: Mild",
        Action::WobbleSeverity(WobbleSeverity::Mild),
    ),
    (
        "Severity: Realistic",
        Action::WobbleSeverity(WobbleSeverity::Realistic),
    ),
    (
        "Severity: Extreme",
        Action::WobbleSeverity(WobbleSeverity::Extreme),
    ),
    ("Trigger Wobble", Action::TriggerWobble),
];
const ITEMS: [&[(&str, Action)]; 6] = [FILE, SETTINGS, MACHINE, VIEW, HARDWARE, HELP];

pub fn content_rect(width: f32, height: f32) -> Rect {
    let padding = 12.0_f32.min(width.max(0.0) / 2.0);
    let top = BAR_HEIGHT.min(height.max(0.0));
    let vertical_padding = 12.0_f32.min((height - top).max(0.0) / 2.0);
    Rect::new(
        padding,
        top + vertical_padding,
        (width - 2.0 * padding).max(0.0),
        (height - top - 2.0 * vertical_padding).max(0.0),
    )
}

fn tab_rect(index: usize, width: f32) -> Rect {
    let scale = (width / TABS.iter().map(|(_, w)| w).sum::<f32>()).clamp(0.0, 1.0);
    let x = TABS[..index].iter().map(|(_, w)| w).sum::<f32>() * scale;
    Rect::new(x, 0.0, TABS[index].1 * scale, BAR_HEIGHT)
}

fn dropdown_rect(index: usize, width: f32) -> Rect {
    let menu_width = if index <= 1 { 310.0 } else { MENU_WIDTH };
    let w = menu_width.min(width.max(0.0));
    let x = tab_rect(index, width).x.min((width - w).max(0.0));
    Rect::new(x, BAR_HEIGHT, w, ROW_HEIGHT * ITEMS[index].len() as f32)
}

fn shortcut(action: Action) -> Option<&'static str> {
    match action {
        Action::Settings => Some("F1"),
        Action::Setting(1) => Some("F2"),
        Action::Setting(3) => Some("F3"),
        Action::Setting(4) => Some("F4"),
        Action::Setting(7) => Some("F6"),
        Action::Status => Some("F7"),
        Action::Pause => Some("F8"),
        Action::Step => Some("F9"),
        Action::TriggerWobble => Some("F9"),
        Action::Reset => Some("F10"),
        Action::Fullscreen => Some("F11"),
        Action::Debug => Some("F12"),
        Action::PowerCycle => Some("Ctrl+R"),
        _ => None,
    }
}

#[derive(Default)]
pub struct Menu {
    open: Option<usize>,
}

impl Menu {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn input(&mut self, width: f32) -> (bool, Option<Action>) {
        if self.is_open() && is_key_pressed(KeyCode::Escape) {
            self.open = None;
            return (true, None);
        }
        if !is_mouse_button_pressed(MouseButton::Left) {
            return (false, None);
        }
        self.click(mouse_position().into(), width)
    }

    fn click(&mut self, position: Vec2, width: f32) -> (bool, Option<Action>) {
        for index in 0..TABS.len() {
            if tab_rect(index, width).contains(position) {
                self.open = if self.open == Some(index) {
                    None
                } else {
                    Some(index)
                };
                return (true, None);
            }
        }
        if let Some(index) = self.open.take() {
            let rect = dropdown_rect(index, width);
            if rect.contains(position) {
                let row = ((position.y - rect.y) / ROW_HEIGHT) as usize;
                return (true, ITEMS[index].get(row).map(|item| item.1));
            }
            return (true, None);
        }
        (position.y < BAR_HEIGHT, None)
    }

    pub fn draw(&self, width: f32, state: &AppState, experience: &Experience, paused: bool) {
        draw_rectangle(
            0.0,
            0.0,
            width,
            BAR_HEIGHT,
            Color::from_rgba(27, 29, 30, 255),
        );
        for (index, (title, _)) in TABS.iter().enumerate() {
            let rect = tab_rect(index, width);
            if self.open == Some(index) {
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, DARKGRAY);
            }
            draw_text(title, rect.x + 10.0, 24.0, 19.0, WHITE);
        }
        if state.program_loading && state.wav_loading.is_none() && width >= 600.0 {
            draw_text(
                "LOADING .P...  F10 CANCEL",
                width - 235.0,
                24.0,
                16.0,
                LIGHTGRAY,
            );
        }
        if let Some(index) = self.open {
            let rect = dropdown_rect(index, width);
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::from_rgba(34, 36, 37, 255),
            );
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, GRAY);
            let mouse: Vec2 = mouse_position().into();
            for (row, &(label, action)) in ITEMS[index].iter().enumerate() {
                let y = rect.y + row as f32 * ROW_HEIGHT;
                if Rect::new(rect.x, y, rect.w, ROW_HEIGHT).contains(mouse) {
                    draw_rectangle(rect.x, y, rect.w, ROW_HEIGHT, DARKGRAY);
                }
                let checked = match action {
                    Action::Setting(0) => experience.settings.enabled,
                    Action::Setting(1) => state.scanlines_enabled,
                    Action::Setting(3) => state.crt_reflections_enabled,
                    Action::Setting(4) => state.crt_curvature_enabled,
                    Action::Setting(5) => state.bloom_enabled,
                    Action::Setting(6) => state.glow_enabled,
                    Action::Setting(7) => state.flicker_enabled,
                    Action::Setting(8) => state.fullscreen_confirmation,
                    Action::TapeSound => state.tape_sound,
                    Action::Pause => paused,
                    Action::Fullscreen => state.fullscreen,
                    Action::VirtualKeyboard => state.show_virtual_keyboard,
                    Action::Status => state.show_status,
                    Action::Debug => state.show_debug,
                    Action::RamSize(ram_size) => state.ram_size == ram_size,
                    Action::Wobble => state.wobble_enabled,
                    Action::WobbleSeverity(severity) => state.wobble_severity == severity,
                    _ => false,
                };
                let is_setting = matches!(
                    action,
                    Action::Setting(_) | Action::TapeSound | Action::Wobble
                );
                if is_setting {
                    let value = if checked { "ON" } else { "OFF" };
                    draw_text(
                        value,
                        rect.x + rect.w - 10.0 - measure_text(value, None, 18, 1.0).width,
                        y + 21.0,
                        18.0,
                        WHITE,
                    );
                } else if checked {
                    draw_text("*", rect.x + 8.0, y + 22.0, 19.0, WHITE);
                }
                let right = rect.x + rect.w - if is_setting { 68.0 } else { 10.0 };
                let active_shortcut = match action {
                    Action::Step if state.wobble_enabled => None,
                    Action::TriggerWobble if !state.wobble_enabled => None,
                    _ => shortcut(action),
                };
                let label_right = if let Some(key) = active_shortcut {
                    let key_x = right - measure_text(key, None, 16, 1.0).width;
                    draw_text(key, key_x, y + 21.0, 16.0, LIGHTGRAY);
                    key_x - 8.0
                } else if is_setting {
                    rect.x + rect.w - 58.0
                } else {
                    right
                };
                let label_x = rect.x + 25.0;
                let label_width = measure_text(label, None, 18, 1.0).width;
                let font_scale = ((label_right - label_x).max(0.0) / label_width).min(1.0);
                draw_text_ex(
                    label,
                    label_x,
                    y + 21.0,
                    TextParams {
                        font_size: 18,
                        font_scale,
                        color: WHITE,
                        ..Default::default()
                    },
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_menu_items_and_dismisses_outside_clicks() {
        let mut menu = Menu::default();
        assert_eq!(menu.click(vec2(10.0, 10.0), 800.0), (true, None));
        assert!(menu.is_open());
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + 5.0), 800.0),
            (true, Some(Action::Open))
        );
        assert!(!menu.is_open());
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT + 5.0), 800.0),
            (true, Some(Action::OpenWav))
        );
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 2.0 + 5.0), 800.0),
            (true, Some(Action::OpenInstant))
        );
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 3.0 + 5.0), 800.0),
            (true, Some(Action::BrowseGamesLibrary(LoadMode::Standard)))
        );
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 4.0 + 5.0), 800.0),
            (true, Some(Action::BrowseGamesLibrary(LoadMode::Instant)))
        );
        assert!(!menu.is_open());
        menu.click(vec2(90.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(100.0, BAR_HEIGHT + ROW_HEIGHT * 4.0 + 5.0), 800.0),
            (true, Some(Action::Setting(1)))
        );
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(menu.click(vec2(500.0, 500.0), 800.0), (true, None));
        assert!(!menu.is_open());
        menu.click(vec2(10.0, 10.0), 800.0);
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 7.0 + 5.0), 800.0),
            (true, Some(Action::Screenshot))
        );
    }

    #[test]
    fn fullscreen_is_available_in_settings_and_view() {
        let mut menu = Menu::default();
        for (tab_x, row) in [(90.0, 1.0), (300.0, 0.0), (90.0, 1.0)] {
            assert_eq!(menu.click(vec2(tab_x, 10.0), 800.0), (true, None));
            let dropdown = dropdown_rect(menu.open.unwrap(), 800.0);
            assert_eq!(
                menu.click(
                    vec2(dropdown.x + 10.0, BAR_HEIGHT + ROW_HEIGHT * row + 5.0),
                    800.0
                ),
                (true, Some(Action::Fullscreen))
            );
            assert!(!menu.is_open());
        }
    }

    #[test]
    fn machine_menu_selects_ram_size() {
        let mut menu = Menu::default();
        assert_eq!(menu.click(vec2(200.0, 10.0), 800.0), (true, None));
        let dropdown = dropdown_rect(menu.open.unwrap(), 800.0);
        assert_eq!(
            menu.click(
                vec2(dropdown.x + 10.0, BAR_HEIGHT + ROW_HEIGHT * 4.0 + 5.0),
                800.0
            ),
            (true, Some(Action::RamSize(RamSize::OneK)))
        );
        assert_eq!(menu.click(vec2(200.0, 10.0), 800.0), (true, None));
        let dropdown = dropdown_rect(menu.open.unwrap(), 800.0);
        assert_eq!(
            menu.click(
                vec2(dropdown.x + 10.0, BAR_HEIGHT + ROW_HEIGHT * 5.0 + 5.0),
                800.0
            ),
            (true, Some(Action::RamSize(RamSize::SixteenK)))
        );
    }

    #[test]
    fn fullscreen_confirmation_uses_its_current_settings_index() {
        let mut menu = Menu::default();
        assert_eq!(menu.click(vec2(90.0, 10.0), 800.0), (true, None));
        assert_eq!(
            menu.click(vec2(100.0, BAR_HEIGHT + ROW_HEIGHT * 2.0 + 5.0), 800.0),
            (true, Some(Action::Setting(8)))
        );
    }

    #[test]
    fn shortcut_labels_follow_menu_actions() {
        for (action, expected) in [
            (Action::Settings, "F1"),
            (Action::Setting(1), "F2"),
            (Action::Setting(3), "F3"),
            (Action::Setting(4), "F4"),
            (Action::Setting(7), "F6"),
            (Action::Status, "F7"),
            (Action::Pause, "F8"),
            (Action::Step, "F9"),
            (Action::Reset, "F10"),
            (Action::Fullscreen, "F11"),
            (Action::Debug, "F12"),
            (Action::PowerCycle, "Ctrl+R"),
        ] {
            assert!(ITEMS
                .iter()
                .any(|items| items.iter().any(|(_, item)| *item == action)));
            assert_eq!(shortcut(action), Some(expected));
        }
        assert_eq!(shortcut(Action::VirtualKeyboard), None);
    }

    #[test]
    fn help_menu_opens_about() {
        let mut menu = Menu::default();
        assert_eq!(menu.click(vec2(490.0, 10.0), 800.0), (true, None));
        let dropdown = dropdown_rect(menu.open.unwrap(), 800.0);
        assert_eq!(
            menu.click(vec2(dropdown.x + 10.0, BAR_HEIGHT + 5.0), 800.0),
            (true, Some(Action::About))
        );
    }

    #[test]
    fn hardware_precedes_help_and_wav_export_is_selectable() {
        let mut menu = Menu::default();
        assert_eq!(menu.click(vec2(390.0, 10.0), 800.0), (true, None));
        let hardware = dropdown_rect(menu.open.unwrap(), 800.0);
        assert_eq!(
            menu.click(vec2(hardware.x + 10.0, BAR_HEIGHT + 5.0), 800.0),
            (true, Some(Action::Wobble))
        );
        assert_eq!(menu.click(vec2(10.0, 10.0), 800.0), (true, None));
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 6.0 + 5.0), 800.0),
            (true, Some(Action::SaveWav))
        );
        assert_eq!(menu.click(vec2(10.0, 10.0), 800.0), (true, None));
        assert_eq!(
            menu.click(vec2(30.0, BAR_HEIGHT + ROW_HEIGHT * 8.0 + 5.0), 800.0),
            (true, Some(Action::Exit))
        );
    }
}
