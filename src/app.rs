use image::ImageEncoder;
use macroquad::prelude::*;
use std::io::Write;

use crate::{
    about::About,
    assets::{AssetManager, TvAssets},
    autoload::AutoLoad,
    bezel,
    config::{RenderLayer, INITIAL_WINDOW_HEIGHT, INITIAL_WINDOW_WIDTH, RENDER_ORDER},
    crt::{self, Crt},
    debug,
    emulator::{EmulatorCore, CPU_HZ},
    experience::{Experience, ScreenState},
    menu::{self, Action, Menu},
    scaling::Scaling,
    ui::{self, SettingsAction},
    viewport,
    virtual_keyboard::{Event as KeyboardEvent, KeyboardWindow},
};

const KEYS: [[KeyCode; 5]; 8] = [
    [
        KeyCode::LeftShift,
        KeyCode::Z,
        KeyCode::X,
        KeyCode::C,
        KeyCode::V,
    ],
    [KeyCode::A, KeyCode::S, KeyCode::D, KeyCode::F, KeyCode::G],
    [KeyCode::Q, KeyCode::W, KeyCode::E, KeyCode::R, KeyCode::T],
    [
        KeyCode::Key1,
        KeyCode::Key2,
        KeyCode::Key3,
        KeyCode::Key4,
        KeyCode::Key5,
    ],
    [
        KeyCode::Key0,
        KeyCode::Key9,
        KeyCode::Key8,
        KeyCode::Key7,
        KeyCode::Key6,
    ],
    [KeyCode::P, KeyCode::O, KeyCode::I, KeyCode::U, KeyCode::Y],
    [
        KeyCode::Enter,
        KeyCode::L,
        KeyCode::K,
        KeyCode::J,
        KeyCode::H,
    ],
    [
        KeyCode::Space,
        KeyCode::Period,
        KeyCode::M,
        KeyCode::N,
        KeyCode::B,
    ],
];

fn host_key_down(key: KeyCode, is_down: impl Fn(KeyCode) -> bool) -> bool {
    is_down(key)
        || (key == KeyCode::LeftShift && is_down(KeyCode::RightShift))
        || (matches!(key, KeyCode::LeftShift | KeyCode::Period) && is_down(KeyCode::Comma))
}

pub struct AppState {
    pub fullscreen: bool,
    pub fullscreen_confirmation: bool,
    pub show_virtual_keyboard: bool,
    pub virtual_keys: [[bool; 5]; 8],
    pub show_debug: bool,
    pub show_settings: bool,
    pub settings_selection: usize,
    pub show_status: bool,
    pub scanlines_enabled: bool,
    pub crt_reflections_enabled: bool,
    pub crt_curvature_enabled: bool,
    pub bloom_enabled: bool,
    pub glow_enabled: bool,
    pub flicker_enabled: bool,
    pub program_loading: bool,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            fullscreen: false,
            fullscreen_confirmation: true,
            show_virtual_keyboard: false,
            virtual_keys: [[false; 5]; 8],
            show_debug: false,
            show_settings: false,
            settings_selection: 0,
            show_status: false,
            scanlines_enabled: true,
            crt_reflections_enabled: true,
            crt_curvature_enabled: false,
            bloom_enabled: true,
            glow_enabled: true,
            flicker_enabled: true,
            program_loading: false,
        }
    }
}

pub async fn run() {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(time) => macroquad::rand::srand(time.as_nanos() as u64),
        Err(error) => eprintln!("Could not seed startup variation from the system clock: {error}"),
    }
    let manager = AssetManager::new();
    let mut app = App {
        bezel: manager.load_bezel_assets(),
        background: manager.load_texture_safe("red_grid.png"),
        title: manager.load_texture_safe("title.png"),
        crt: Crt::new(),
        state: AppState::default(),
        experience: Experience::default(),
        emulator: EmulatorCore::new(),
        menu: Menu::default(),
        autoload: None,
        notification: None,
        paused: false,
        cycle_credit: 0.0,
        display_error_reported: false,
        discard_frame_time: false,
        pending_fullscreen: None,
        confirmation_selection: 0,
        keyboard_window: None,
        screenshot_requested: false,
        pending_screenshot: None,
        about: About::new(),
    };
    let mut args = std::env::args_os().skip(1);
    if let Some(argument) = args.next() {
        if argument == "--rom" {
            if let Some(path) = args.next() {
                if let Err(error) = app.emulator.load_rom(&path) {
                    app.notify(format!(
                        "Could not load ROM {}: {error}",
                        std::path::Path::new(&path).display()
                    ));
                }
            } else {
                app.notify("Missing ROM path after --rom.");
            }
        } else {
            app.notify("Unknown argument; use --rom <path>.");
        }
        if args.next().is_some() {
            app.notify("Unexpected extra command-line arguments.");
        }
    } else if let Some(path) = default_rom_path() {
        if let Err(error) = app.emulator.load_rom(&path) {
            app.notify(format!("Could not load ROM {}: {error}", path.display()));
        }
    }
    loop {
        app.update();
        app.draw();
        next_frame().await;
    }
}

fn default_rom_path() -> Option<std::path::PathBuf> {
    let mut candidates = vec![std::path::PathBuf::from("rom/zx81.rom")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("rom/zx81.rom"));
        }
    }
    candidates.push(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("rom/zx81.rom"));
    candidates.into_iter().find(|path| path.is_file())
}

fn accrue_cycles(credit: &mut f64, delta: f32) -> u32 {
    *credit += f64::from(delta) * f64::from(CPU_HZ);
    // Bound work per render without discarding elapsed emulated time.
    credit.floor().clamp(0.0, f64::from(CPU_HZ / 10)) as u32
}

struct App {
    bezel: TvAssets,
    background: Option<Texture2D>,
    title: Option<Texture2D>,
    crt: Crt,
    state: AppState,
    experience: Experience,
    emulator: EmulatorCore,
    menu: Menu,
    autoload: Option<AutoLoad>,
    notification: Option<(String, f32)>,
    paused: bool,
    cycle_credit: f64,
    display_error_reported: bool,
    discard_frame_time: bool,
    pending_fullscreen: Option<bool>,
    confirmation_selection: usize,
    keyboard_window: Option<KeyboardWindow>,
    screenshot_requested: bool,
    pending_screenshot: Option<image::RgbaImage>,
    about: About,
}

impl App {
    fn notify(&mut self, message: impl Into<String>) {
        let message = message.into();
        eprintln!("{message}");
        self.notification = Some((message, 4.0));
    }

    fn request_fullscreen(&mut self, target: bool) {
        if target == self.state.fullscreen {
            return;
        }
        if self.state.fullscreen_confirmation {
            self.pending_fullscreen = Some(target);
            self.confirmation_selection = 0;
        } else {
            self.apply_fullscreen(target);
        }
    }

    fn apply_fullscreen(&mut self, target: bool) {
        self.state.fullscreen = target;
        set_fullscreen(target);
    }

    fn update_fullscreen_confirmation(&mut self) {
        if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::Right) {
            self.confirmation_selection = 1 - self.confirmation_selection;
        }
        if is_key_pressed(KeyCode::Escape) {
            self.pending_fullscreen = None;
            return;
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space) {
            let target = self.pending_fullscreen.take();
            if self.confirmation_selection == 0 {
                if let Some(target) = target {
                    self.apply_fullscreen(target);
                }
            }
            return;
        }

        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            let width = screen_width();
            let height = screen_height();
            let panel = Rect::new(width * 0.5 - 210.0, height * 0.5 - 90.0, 420.0, 180.0);
            let yes = Rect::new(panel.x + 55.0, panel.y + 115.0, 130.0, 42.0);
            let no = Rect::new(panel.x + 235.0, panel.y + 115.0, 130.0, 42.0);
            if yes.contains(vec2(mouse_x, mouse_y)) {
                if let Some(target) = self.pending_fullscreen.take() {
                    self.apply_fullscreen(target);
                }
            } else if no.contains(vec2(mouse_x, mouse_y)) {
                self.pending_fullscreen = None;
            }
        }
    }

    fn update_virtual_keyboard(&mut self) {
        let event = match &mut self.keyboard_window {
            Some(window) => window.poll(),
            None => return,
        };
        match event {
            Ok(KeyboardEvent::State(mask)) => {
                for (row, keys) in self.state.virtual_keys.iter_mut().enumerate() {
                    for (column, pressed) in keys.iter_mut().enumerate() {
                        *pressed = mask & (1 << (row * 5 + column)) != 0;
                    }
                }
            }
            Ok(KeyboardEvent::None) => {}
            Ok(KeyboardEvent::Closed) => self.close_virtual_keyboard(),
            Err(error) => {
                self.close_virtual_keyboard();
                self.notify(error);
            }
        }
    }

    fn close_virtual_keyboard(&mut self) {
        self.keyboard_window = None;
        self.state.show_virtual_keyboard = false;
        self.state.virtual_keys = [[false; 5]; 8];
    }

    fn power_cycle(&mut self) {
        self.experience.start_shutdown(true);
        self.state.program_loading = false;
        self.autoload = None;
        self.emulator.reset();
        self.cycle_credit = 0.0;
        self.display_error_reported = false;
    }

    fn reset(&mut self) {
        self.emulator.reset();
        self.cycle_credit = 0.0;
        self.display_error_reported = false;
        self.experience.reset();
        self.state.program_loading = false;
        self.autoload = None;
    }

    fn open_program(&mut self, path: &std::path::Path) {
        if !self.emulator.is_loaded() {
            self.notify("A ZX81 ROM is required to run .P files; start with --rom.");
        } else if self.autoload.is_some() {
            self.notify("A program is already loading.");
        } else if self.paused {
            self.notify("Resume the emulator with F8 before loading a program.");
        } else if matches!(
            self.experience.screen,
            ScreenState::PowerOff | ScreenState::Shutdown
        ) {
            self.notify("Wait until the television is on before loading a program.");
        } else {
            match AutoLoad::from_path(&mut self.emulator, path) {
                Ok(loader) => {
                    self.autoload = Some(loader);
                    self.cycle_credit = 0.0;
                    self.display_error_reported = false;
                    self.notify(format!("Loading {}...", path.display()));
                }
                Err(error) => self.notify(format!("Cannot load {}: {error}", path.display())),
            }
        }
    }

    fn file_dialog(&mut self) -> rfd::FileDialog {
        self.discard_frame_time = true;
        let dialog = rfd::FileDialog::new().add_filter("ZX81 program", &["p", "P"]);
        if let Some(home) = dirs::home_dir() {
            dialog.set_directory(home)
        } else {
            eprintln!("Cannot find the home directory; using the file dialog's default folder.");
            dialog
        }
    }

    fn browse_games_library(&mut self) {
        let directory = match games_library_dir() {
            Ok(directory) => directory,
            Err(error) => {
                self.notify(format!("Cannot open games library: {error}"));
                return;
            }
        };
        let dialog = rfd::FileDialog::new()
            .set_title("Browse Emulator Games Library")
            .add_filter("ZX81 program", &["p", "P"])
            .set_directory(&directory);
        #[cfg(target_os = "linux")]
        let dialog = match linux_library_filename(&directory) {
            Ok(filename) => dialog.set_file_name(filename),
            Err(error) => {
                self.notify(format!("Cannot open games library: {error}"));
                return;
            }
        };
        self.discard_frame_time = true;
        if let Some(path) = dialog.pick_file() {
            self.open_program(&path);
        }
    }

    fn save_program(&mut self) {
        if self.autoload.is_some() {
            self.notify("Wait for the program to finish loading before saving.");
            return;
        }
        let bytes = match self.emulator.save_program_bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                self.notify(format!("Cannot save program: {error}"));
                return;
            }
        };
        let path = self.file_dialog().set_file_name("program.p").save_file();
        if let Some(path) = path {
            if !path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("p"))
            {
                self.notify("Cannot save program: the filename must end in .p.");
            } else if let Err(error) = write_file_atomically(&path, &bytes) {
                self.notify(format!("Cannot save {}: {error}", path.display()));
            } else {
                self.notify(format!("Saved {}.", path.display()));
            }
        }
    }

    fn save_screenshot(&mut self, screenshot: image::RgbaImage) {
        self.discard_frame_time = true;
        let dialog = rfd::FileDialog::new()
            .set_title("Save CRT Screenshot")
            .add_filter("PNG image", &["png"])
            .set_file_name("screenshot.png");
        let Some(path) = dialog.save_file() else {
            return;
        };
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
        {
            self.notify("Cannot save screenshot: the filename must end in .png.");
        } else if let Err(error) = write_screenshot_file(&path, &screenshot) {
            self.notify(format!(
                "Cannot save screenshot {}: {error}",
                path.display()
            ));
        } else {
            self.notify(format!("Saved screenshot {}.", path.display()));
        }
    }

    fn update(&mut self) {
        let delta = if std::mem::take(&mut self.discard_frame_time) {
            0.0
        } else {
            get_frame_time().max(0.0)
        };
        if let Some(screenshot) = self.pending_screenshot.take() {
            self.save_screenshot(screenshot);
        }
        self.update_virtual_keyboard();
        if self.about.open {
            self.about.update();
            self.advance_emulation(delta, true, false);
            return;
        }
        if self.pending_fullscreen.is_some() {
            self.update_fullscreen_confirmation();
            self.experience.update(delta);
            self.state.program_loading = self.autoload.is_some();
            return;
        }
        let (menu_consumed, action) = self.menu.input(screen_width());
        let mut step_requested = false;
        match action {
            Some(Action::Open) => {
                if let Some(path) = self.file_dialog().pick_file() {
                    self.open_program(&path);
                }
            }
            Some(Action::BrowseGamesLibrary) => self.browse_games_library(),
            Some(Action::Save) => self.save_program(),
            Some(Action::Screenshot) => self.screenshot_requested = true,
            Some(Action::About) => self.about.open = true,
            Some(Action::Settings) => self.state.show_settings = !self.state.show_settings,
            Some(Action::Setting(index)) => {
                ui::activate_setting(index, &mut self.state, &mut self.experience);
            }
            Some(Action::Pause) => {
                if self.emulator.is_loaded() {
                    self.paused = !self.paused;
                    self.cycle_credit = 0.0;
                } else {
                    self.notify("A ZX81 ROM is required to control the machine.");
                }
            }
            Some(Action::Step) if !self.emulator.is_loaded() => {
                self.notify("A ZX81 ROM is required to control the machine.");
            }
            Some(Action::Step) if self.paused => step_requested = true,
            Some(Action::Step) => self.notify("Pause the emulator before single-stepping."),
            Some(Action::Reset) => self.reset(),
            Some(Action::PowerCycle) => self.power_cycle(),
            Some(Action::Fullscreen) => {
                self.request_fullscreen(!self.state.fullscreen);
            }
            Some(Action::VirtualKeyboard) => {
                if self.keyboard_window.is_some() {
                    self.close_virtual_keyboard();
                } else {
                    match KeyboardWindow::open() {
                        Ok(window) => {
                            self.keyboard_window = Some(window);
                            self.state.show_virtual_keyboard = true;
                        }
                        Err(error) => self.notify(format!("Cannot open virtual keyboard: {error}")),
                    }
                }
            }
            Some(Action::Status) => self.state.show_status = !self.state.show_status,
            Some(Action::Debug) => self.state.show_debug = !self.state.show_debug,
            None => {}
        }
        if is_key_pressed(KeyCode::F11) {
            self.request_fullscreen(!self.state.fullscreen);
        }
        if !menu_consumed && is_key_pressed(KeyCode::Escape) {
            if self.state.show_settings {
                self.state.show_settings = false;
            } else if self.state.fullscreen {
                self.request_fullscreen(false);
            }
        }
        if is_key_pressed(KeyCode::F1) {
            self.state.show_settings = !self.state.show_settings;
        }
        if is_key_pressed(KeyCode::F2) {
            self.state.scanlines_enabled = !self.state.scanlines_enabled;
        }
        if is_key_pressed(KeyCode::F3) {
            self.state.crt_reflections_enabled = !self.state.crt_reflections_enabled;
        }
        if is_key_pressed(KeyCode::F4) {
            self.state.crt_curvature_enabled = !self.state.crt_curvature_enabled;
        }
        if is_key_pressed(KeyCode::F6) {
            self.state.flicker_enabled = !self.state.flicker_enabled;
        }
        if is_key_pressed(KeyCode::F7) {
            self.state.show_status = !self.state.show_status;
        }
        if is_key_pressed(KeyCode::F12) {
            self.state.show_debug = !self.state.show_debug;
        }
        if self.emulator.is_loaded() && is_key_pressed(KeyCode::F8) {
            self.paused = !self.paused;
            self.cycle_credit = 0.0;
        }
        if self.emulator.is_loaded() && is_key_pressed(KeyCode::F10) {
            self.reset();
        }
        if (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl))
            && is_key_pressed(KeyCode::R)
        {
            self.power_cycle();
        }
        if self.state.show_settings {
            match ui::settings_input(&mut self.state, &mut self.experience) {
                SettingsAction::None => {}
                SettingsAction::Reset => self.reset(),
                SettingsAction::PowerCycle => self.power_cycle(),
            }
        }

        for file in get_dropped_files() {
            match file.path {
                Some(path) => self.open_program(&path),
                None => self.notify("Cannot load a dropped file without a filename."),
            }
        }

        self.advance_emulation(delta, menu_consumed, step_requested);
    }

    fn advance_emulation(&mut self, delta: f32, menu_consumed: bool, step_requested: bool) {
        self.experience.update(delta);
        self.state.program_loading = self.autoload.is_some();
        if let Some((_, remaining)) = &mut self.notification {
            *remaining -= delta;
            if *remaining <= 0.0 {
                self.notification = None;
            }
        }
        if !matches!(
            self.experience.screen,
            ScreenState::PowerOff | ScreenState::Shutdown
        ) {
            if self.emulator.is_loaded() {
                self.update_machine(delta, menu_consumed, step_requested);
            } else {
                self.emulator.run_frame();
            }
        }
    }

    fn update_machine(&mut self, delta: f32, menu_consumed: bool, step_requested: bool) {
        let accepts_keys = self.autoload.is_none()
            && !self.menu.is_open()
            && !menu_consumed
            && !self.state.show_settings
            && !is_key_down(KeyCode::LeftControl)
            && !is_key_down(KeyCode::RightControl);
        if self.autoload.is_none() {
            for (row, keys) in KEYS.iter().enumerate() {
                for (column, key) in keys.iter().enumerate() {
                    let pressed = accepts_keys && host_key_down(*key, is_key_down)
                        || self.state.virtual_keys[row][column];
                    self.emulator
                        .set_key(row, column, pressed)
                        .expect("static ZX81 keyboard mapping must be in bounds");
                }
            }
        }
        let result = if self.paused {
            if step_requested || (!self.about.open && is_key_pressed(KeyCode::F9)) {
                self.emulator.step_instruction()
            } else {
                Ok(0)
            }
        } else {
            let budget = accrue_cycles(&mut self.cycle_credit, delta);
            if budget > 0 {
                let accelerated = if self.autoload.as_ref().is_some_and(AutoLoad::is_playing) {
                    budget.max(CPU_HZ / 2)
                } else {
                    budget
                };
                let extra = accelerated - budget;
                let result = if self.autoload.as_ref().is_some_and(AutoLoad::is_playing) {
                    self.emulator.run_cycles_until_tape_stops(accelerated)
                } else {
                    self.emulator.run_cycles(accelerated)
                };
                result.inspect(|&ran| {
                    self.cycle_credit -= f64::from(ran.saturating_sub(extra));
                })
            } else {
                Ok(0)
            }
        };
        let cycles = match result {
            Ok(cycles) => cycles,
            Err(error) => {
                self.paused = true;
                self.notify(format!("Emulation stopped: {error}"));
                return;
            }
        };
        if let Some(loader) = &mut self.autoload {
            match loader.advance(&mut self.emulator, cycles) {
                Ok(true) => {
                    self.autoload = None;
                    self.notify("Program loaded.");
                }
                Ok(false) => {}
                Err(error) => {
                    self.reset();
                    self.notify(format!("Program load failed: {error}"));
                    return;
                }
            }
        }
        if let Err(error) = self.emulator.refresh_display() {
            let cycles = self.emulator.cpu_snapshot().map_or(0, |cpu| cpu.cycles);
            if cycles >= 3_250_000 && !self.display_error_reported && self.autoload.is_none() {
                self.display_error_reported = true;
                self.notify(format!("Cannot decode ZX81 display: {error}"));
            }
        } else {
            self.display_error_reported = false;
        }
    }

    fn draw(&mut self) {
        let capture = if std::mem::take(&mut self.screenshot_requested) {
            match begin_screenshot_capture() {
                Ok(target) => Some(target),
                Err(error) => {
                    self.notify(format!("Cannot capture CRT screenshot: {error}"));
                    None
                }
            }
        } else {
            None
        };
        clear_background(Color::from_rgba(16, 18, 19, 255));
        let (background, scene, monitor_area) =
            scene_layout(Rect::new(0.0, 0.0, screen_width(), screen_height()));
        if let Some(texture) = &self.background {
            draw_texture_ex(
                texture,
                background.x,
                background.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(background.w, background.h)),
                    ..Default::default()
                },
            );
        }
        draw_scene_title(scene, self.title.as_ref());
        let scaling = Scaling::fit(monitor_area, bezel::SCENE_BOUNDS);
        let aperture = bezel::aperture_rect(&scaling);
        let screen = bezel::viewport_rect(&scaling);
        let mut settings = self.experience.settings;
        if !self.state.flicker_enabled {
            settings.flicker_strength = 0.0;
        }
        let effects = crt::visuals(
            self.experience.screen,
            self.experience.elapsed,
            &settings,
            self.experience.startup_variation,
        );
        let glow = if self.state.glow_enabled {
            settings.bloom_strength
        } else {
            0.0
        };
        let bloom = if self.state.bloom_enabled {
            settings.bloom_strength
        } else {
            0.0
        };
        for layer in RENDER_ORDER {
            match layer {
                RenderLayer::TvShadow => {
                    bezel::draw_bezel_shadow(&self.bezel, &scaling);
                    bezel::draw_screen_backing(aperture);
                }
                RenderLayer::TvCabinet => {
                    bezel::draw_tv_bezel(&self.bezel, &scaling, aperture);
                    bezel::draw_power_indicator(
                        &scaling,
                        self.experience.power_indicator_brightness(),
                    );
                }
                RenderLayer::Viewport => {
                    let jitter = if self.experience.screen == ScreenState::Booting {
                        let t = self.experience.startup_fraction();
                        if (1.5 / 3.5..2.5 / 3.5).contains(&t) {
                            (self.experience.elapsed * 24.0 + self.experience.startup_variation)
                                .sin()
                                * 0.003
                                * (1.0 - t)
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    };
                    self.emulator.render_screen(
                        screen,
                        &viewport::ScreenFrame {
                            state: self.experience.screen,
                            progress: self.experience.loading_progress,
                            filename: self.experience.filename.as_deref(),
                            brightness: effects.brightness,
                            bloom,
                            glow,
                            vertical_jitter: jitter,
                        },
                    );
                    if self.autoload.as_ref().is_some_and(AutoLoad::is_playing) {
                        if let Some(tape) = self.emulator.tape() {
                            self.crt
                                .draw_loading_lines(screen, tape, effects.brightness);
                        }
                    }
                    match self.experience.screen {
                        ScreenState::Booting => self.crt.render_power_on(
                            screen,
                            self.experience.startup_fraction(),
                            bloom,
                            (self.experience.elapsed * 16.0 + self.experience.startup_variation)
                                .sin(),
                        ),
                        ScreenState::Shutdown => self.crt.render_shutdown(
                            screen,
                            self.experience.shutdown_fraction(),
                            bloom,
                        ),
                        _ => {}
                    }
                    if self.state.scanlines_enabled && effects.scanlines > 0.0 {
                        self.crt.draw_scanlines(screen, effects.scanlines);
                    }
                    if self.state.crt_curvature_enabled && effects.brightness > 0.0 {
                        self.crt.draw_curvature(screen);
                    }
                    if self.state.flicker_enabled && effects.brightness > 0.0 {
                        let intensity = (get_time() as f32 * 37.0).sin().abs()
                            * settings.flicker_strength
                            * 0.25;
                        draw_rectangle(
                            screen.x,
                            screen.y,
                            screen.w,
                            screen.h,
                            Color::new(0.0, 0.0, 0.0, intensity),
                        );
                    }
                    self.crt.draw_soft_edges(screen);
                }
                RenderLayer::CrtShadowMask => self.crt.draw_vignette(&self.bezel, &scaling, screen),
                RenderLayer::CrtGlass => self.crt.draw_glass(
                    &self.bezel,
                    &scaling,
                    screen,
                    if effects.brightness > 0.0 { 1.0 } else { 0.08 },
                ),
                RenderLayer::CrtReflections
                    if self.state.crt_reflections_enabled && effects.reflections > 0.0 =>
                {
                    self.crt
                        .draw_reflections(&self.bezel, &scaling, screen, effects.reflections);
                }
                RenderLayer::CrtReflections => {}
            }
        }
        if let Some(target) = capture {
            match finish_screenshot_capture(&target, screen) {
                Ok(screenshot) => self.pending_screenshot = Some(screenshot),
                Err(error) => self.notify(format!("Cannot capture CRT screenshot: {error}")),
            }
        }
        if self.state.show_status {
            ui::draw_status(&self.state, &self.experience);
        }
        if let Some((message, _)) = &self.notification {
            ui::draw_notification(message);
        }
        if self.state.show_debug {
            viewport::draw_viewport_outline(screen);
            debug::draw_debug_overlay(screen, &self.state, &scaling, &self.emulator, self.paused);
        }
        if self.state.show_settings {
            ui::draw_settings(&scaling, &self.state, &self.experience);
        }
        self.menu
            .draw(screen_width(), &self.state, &self.experience, self.paused);
        if let Some(target_fullscreen) = self.pending_fullscreen {
            ui::draw_fullscreen_confirmation(target_fullscreen, self.confirmation_selection);
        }
        self.about.draw(self.background.as_ref());
    }
}

fn games_library_dir() -> std::io::Result<std::path::PathBuf> {
    let mut candidates = vec![
        std::path::PathBuf::from("Games Library"),
        std::path::PathBuf::from("ZX81_Emulator/Games Library"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            candidates.push(parent.join("Games Library"));
        }
    }
    candidates.push(std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Games Library"));
    resolve_games_library(candidates)
}

fn resolve_games_library(
    candidates: impl IntoIterator<Item = std::path::PathBuf>,
) -> std::io::Result<std::path::PathBuf> {
    for candidate in candidates {
        match std::fs::metadata(&candidate) {
            Ok(metadata) if metadata.is_dir() => return candidate.canonicalize(),
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) => {}
            Err(error) => {
                return Err(std::io::Error::new(
                    error.kind(),
                    format!("{}: {error}", candidate.display()),
                ));
            }
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Games Library folder not found",
    ))
}

#[cfg(target_os = "linux")]
fn linux_library_filename(directory: &std::path::Path) -> std::io::Result<String> {
    let directory = directory.to_str().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "games library path is not valid UTF-8",
        )
    })?;
    // rfd's Zenity fallback uses --filename instead of its starting directory.
    Ok(format!("{}/", directory.trim_end_matches('/')))
}

fn scene_layout(bounds: Rect) -> (Rect, Rect, Rect) {
    let background = viewport::calculate_viewport(bounds);
    let design = Rect::new(
        0.0,
        0.0,
        INITIAL_WINDOW_WIDTH as f32,
        INITIAL_WINDOW_HEIGHT as f32,
    );
    let scaling = Scaling::fit(background, design);
    let mut content = scaling.rect(menu::content_rect(design.w, design.h));
    let bottom = content.y + content.h;
    content.y = content.y.max(menu::BAR_HEIGHT).min(bottom);
    content.h = (bottom - content.y).max(0.0);
    let scene = viewport::calculate_viewport(content);
    let header = scene.h * 0.13;
    (
        background,
        scene,
        Rect::new(scene.x, scene.y + header, scene.w, scene.h - header),
    )
}

const TITLE_SCALE: f32 = 0.765;

fn scene_title_rect(background: Rect, texture_size: Vec2) -> Rect {
    let width = TITLE_SCALE
        * (background.w * 0.48).min(background.h * 0.108 * texture_size.x / texture_size.y);
    let height = width * texture_size.y / texture_size.x;
    Rect::new(
        background.x + background.w * 0.97 - width,
        background.y + background.h * 0.01,
        width,
        height,
    )
}

fn draw_scene_title(background: Rect, texture: Option<&Texture2D>) {
    if background.w <= 0.0 || background.h <= 0.0 {
        return;
    }
    if let Some(texture) = texture {
        let rect = scene_title_rect(background, vec2(texture.width(), texture.height()));
        draw_texture_ex(
            texture,
            rect.x,
            rect.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                ..Default::default()
            },
        );
        return;
    }
    let right = background.x + background.w * 0.97;
    for (text, size, baseline, color) in [
        (
            "EMULATOR-81",
            0.052,
            0.061,
            Color::from_rgba(250, 73, 36, 255),
        ),
        (
            "ZX81 EMULATOR",
            0.031,
            0.104,
            Color::from_rgba(65, 120, 210, 255),
        ),
    ] {
        let font_scale = background.h * size * TITLE_SCALE / 64.0;
        let width = measure_text(text, None, 64, font_scale).width;
        draw_text_ex(
            text,
            right - width,
            background.y + background.h * baseline,
            TextParams {
                font_size: 64,
                font_scale,
                color,
                ..Default::default()
            },
        );
    }
}

fn write_file_atomically(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(bytes)?;
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn begin_screenshot_capture() -> std::io::Result<RenderTarget> {
    let (width, height) = macroquad::miniquad::window::screen_size();
    if !width.is_finite()
        || !height.is_finite()
        || width < 1.0
        || height < 1.0
        || width > u16::MAX as f32
        || height > u16::MAX as f32
        || screen_width() <= 0.0
        || screen_height() <= 0.0
    {
        return Err(std::io::Error::other("invalid screenshot dimensions"));
    }
    let target = render_target(width as u32, height as u32);
    set_camera(&Camera2D {
        render_target: Some(target.clone()),
        ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, screen_width(), screen_height()))
    });
    Ok(target)
}

fn finish_screenshot_capture(
    target: &RenderTarget,
    screen: Rect,
) -> std::io::Result<image::RgbaImage> {
    set_default_camera();
    let image = target.texture.get_texture_data();
    draw_texture_ex(
        &target.texture,
        0.0,
        0.0,
        WHITE,
        DrawTextureParams {
            dest_size: Some(vec2(screen_width(), screen_height())),
            flip_y: true,
            ..Default::default()
        },
    );
    crop_crt_screenshot(&image, screen, screen_width(), screen_height())
}

fn crop_crt_screenshot(
    framebuffer: &Image,
    screen: Rect,
    window_width: f32,
    window_height: f32,
) -> std::io::Result<image::RgbaImage> {
    if window_width <= 0.0
        || window_height <= 0.0
        || !window_width.is_finite()
        || !window_height.is_finite()
        || !screen.x.is_finite()
        || !screen.y.is_finite()
        || !screen.w.is_finite()
        || !screen.h.is_finite()
        || screen.w <= 0.0
        || screen.h <= 0.0
    {
        return Err(std::io::Error::other("CRT has no visible area"));
    }
    let width = u32::from(framebuffer.width);
    let height = u32::from(framebuffer.height);
    let scale_x = width as f32 / window_width;
    let scale_y = height as f32 / window_height;
    let left = (screen.x * scale_x).floor().clamp(0.0, width as f32) as u32;
    let right = ((screen.x + screen.w) * scale_x)
        .ceil()
        .clamp(0.0, width as f32) as u32;
    let top = (screen.y * scale_y).floor().clamp(0.0, height as f32) as u32;
    let bottom = ((screen.y + screen.h) * scale_y)
        .ceil()
        .clamp(0.0, height as f32) as u32;
    if right <= left
        || bottom <= top
        || framebuffer.bytes.len() != width as usize * height as usize * 4
    {
        return Err(std::io::Error::other(
            "CRT is outside the visible framebuffer",
        ));
    }
    let mut pixels = Vec::with_capacity((right - left) as usize * (bottom - top) as usize * 4);
    for y in top..bottom {
        // Render-target readback is bottom-up.
        let start = ((height - 1 - y) * width + left) as usize * 4;
        pixels.extend_from_slice(&framebuffer.bytes[start..start + (right - left) as usize * 4]);
    }
    image::RgbaImage::from_raw(right - left, bottom - top, pixels)
        .ok_or_else(|| std::io::Error::other("invalid CRT screenshot dimensions"))
}

fn write_screenshot_file(
    path: &std::path::Path,
    screenshot: &image::RgbaImage,
) -> std::io::Result<()> {
    let mut bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut bytes)
        .write_image(
            screenshot.as_raw(),
            screenshot.width(),
            screenshot.height(),
            image::ColorType::Rgba8,
        )
        .map_err(std::io::Error::other)?;
    write_file_atomically(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_keys_follow_zx81_layout_and_either_shift() {
        let layout = [
            [
                KeyCode::Key1,
                KeyCode::Key2,
                KeyCode::Key3,
                KeyCode::Key4,
                KeyCode::Key5,
                KeyCode::Key6,
                KeyCode::Key7,
                KeyCode::Key8,
                KeyCode::Key9,
                KeyCode::Key0,
            ],
            [
                KeyCode::Q,
                KeyCode::W,
                KeyCode::E,
                KeyCode::R,
                KeyCode::T,
                KeyCode::Y,
                KeyCode::U,
                KeyCode::I,
                KeyCode::O,
                KeyCode::P,
            ],
            [
                KeyCode::A,
                KeyCode::S,
                KeyCode::D,
                KeyCode::F,
                KeyCode::G,
                KeyCode::H,
                KeyCode::J,
                KeyCode::K,
                KeyCode::L,
                KeyCode::Enter,
            ],
            [
                KeyCode::LeftShift,
                KeyCode::Z,
                KeyCode::X,
                KeyCode::C,
                KeyCode::V,
                KeyCode::B,
                KeyCode::N,
                KeyCode::M,
                KeyCode::Period,
                KeyCode::Space,
            ],
        ];
        for (display_row, keys) in layout.iter().enumerate() {
            for (display_column, key) in keys.iter().enumerate() {
                let row = if display_column < 5 {
                    3 - display_row
                } else {
                    4 + display_row
                };
                let column = if display_column < 5 {
                    display_column
                } else {
                    9 - display_column
                };
                assert_eq!(KEYS[row][column], *key);
            }
        }
        for shift in [KeyCode::LeftShift, KeyCode::RightShift] {
            assert!(host_key_down(KeyCode::LeftShift, |key| key == shift));
            assert!(!host_key_down(KeyCode::Period, |key| key == shift));
        }
        assert!(host_key_down(KeyCode::Period, |key| key == KeyCode::Period));
        for key in KEYS.iter().flatten() {
            assert_eq!(
                host_key_down(*key, |pressed| pressed == KeyCode::Comma),
                matches!(key, KeyCode::LeftShift | KeyCode::Period),
                "host comma mapping for {key:?}"
            );
        }
    }

    #[test]
    fn background_and_monitor_fit_at_different_window_sizes() {
        let title_image = Image::from_file_with_format(
            &std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/title.png")).unwrap(),
            None,
        )
        .unwrap();
        let title_size = vec2(title_image.width() as f32, title_image.height() as f32);
        for (width, height) in [
            (800.0, 600.0),
            (1600.0, 1200.0),
            (400.0, 300.0),
            (1920.0, 1080.0),
            (800.0, 1200.0),
        ] {
            let (background, scene, monitor_area) =
                scene_layout(Rect::new(0.0, 0.0, width, height));
            let monitor = Scaling::fit(monitor_area, bezel::SCENE_BOUNDS).rect(bezel::SCENE_BOUNDS);
            let title = scene_title_rect(scene, title_size);
            assert!((background.w / background.h - 4.0 / 3.0).abs() < 0.00001);
            assert!(background.x >= 0.0 && background.y >= 0.0);
            assert!(background.x + background.w <= width + 0.01);
            assert!(background.y + background.h <= height + 0.01);
            assert!(scene.y >= menu::BAR_HEIGHT);
            assert!(monitor.x >= scene.x - 0.01);
            assert!(monitor.y >= scene.y + scene.h * 0.13 - 0.01);
            assert!(monitor.x + monitor.w <= scene.x + scene.w + 0.01);
            assert!(monitor.y + monitor.h <= scene.y + scene.h + 0.01);
            assert!(title.x >= scene.x && title.y >= scene.y);
            assert!(title.x + title.w <= scene.x + scene.w);
            assert!(title.y + title.h <= monitor_area.y);
            assert!((title.w / title.h - title_size.x / title_size.y).abs() < 0.00001);
            let original_width =
                (scene.w * 0.48).min(scene.h * 0.108 * title_size.x / title_size.y);
            assert!((title.w - original_width * 0.9 * 0.85).abs() < 0.001);
            assert!((title.x + title.w - (scene.x + scene.w * 0.97)).abs() < 0.001);
            assert!((title.y - (scene.y + scene.h * 0.01)).abs() < 0.001);
        }
    }

    #[test]
    fn four_by_three_window_scales_the_whole_scene_proportionally() {
        let (background, scene, monitor_area) = scene_layout(Rect::new(0.0, 0.0, 800.0, 600.0));
        let monitor = Scaling::fit(monitor_area, bezel::SCENE_BOUNDS).rect(bezel::SCENE_BOUNDS);
        let title = scene_title_rect(scene, vec2(990.0, 150.0));
        let (large_background, large_scene, large_monitor_area) =
            scene_layout(Rect::new(0.0, 0.0, 1600.0, 1200.0));
        let large_monitor =
            Scaling::fit(large_monitor_area, bezel::SCENE_BOUNDS).rect(bezel::SCENE_BOUNDS);
        let large_title = scene_title_rect(large_scene, vec2(990.0, 150.0));
        for (small, large) in [
            (background, large_background),
            (scene, large_scene),
            (monitor, large_monitor),
            (title, large_title),
        ] {
            assert!((large.x - small.x * 2.0).abs() < 0.001);
            assert!((large.y - small.y * 2.0).abs() < 0.001);
            assert!((large.w - small.w * 2.0).abs() < 0.001);
            assert!((large.h - small.h * 2.0).abs() < 0.001);
        }
        assert_eq!(background, Rect::new(0.0, 0.0, 800.0, 600.0));
    }

    #[test]
    fn games_library_resolution_finds_a_directory_and_reports_missing() {
        let temporary = tempfile::tempdir().unwrap();
        let missing = temporary.path().join("missing");
        let file = temporary.path().join("file");
        std::fs::write(&file, b"not a library").unwrap();
        let library = temporary.path().join("Games Library");
        std::fs::create_dir(&library).unwrap();
        assert_eq!(
            resolve_games_library([missing.clone(), file, library.clone()]).unwrap(),
            library.canonicalize().unwrap()
        );
        assert_eq!(
            resolve_games_library([missing]).unwrap_err().kind(),
            std::io::ErrorKind::NotFound
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn library_picker_supplies_zenity_a_directory_path() {
        assert_eq!(
            linux_library_filename(std::path::Path::new("/tmp/Games Library")).unwrap(),
            "/tmp/Games Library/"
        );
    }

    #[test]
    fn cycle_budget_preserves_elapsed_time_and_instruction_overshoot() {
        let mut credit = 0.0;
        let first = accrue_cycles(&mut credit, 0.25);
        assert_eq!(first, CPU_HZ / 10);
        credit -= f64::from(first + 3);
        let second = accrue_cycles(&mut credit, 0.0);
        assert_eq!(second, CPU_HZ / 10);
        credit -= f64::from(second);
        assert_eq!(accrue_cycles(&mut credit, 0.0), CPU_HZ / 20 - 3);
        credit = -3.0;
        assert_eq!(accrue_cycles(&mut credit, 0.0), 0);
        assert_eq!(credit, -3.0);
    }

    #[test]
    fn save_writes_and_replaces_the_selected_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("program.p");
        write_file_atomically(&path, b"first").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"first");
        write_file_atomically(&path, b"second").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"second");
        assert!(
            write_file_atomically(&directory.path().join("missing/program.p"), b"bad").is_err()
        );
    }

    #[test]
    fn screenshot_crops_scales_flips_and_saves_png() {
        let framebuffer = Image {
            width: 4,
            height: 4,
            bytes: (0..16).flat_map(|i| [i, 0, 0, 255]).collect(),
        };
        let screenshot =
            crop_crt_screenshot(&framebuffer, Rect::new(0.5, 0.5, 1.0, 1.0), 2.0, 2.0).unwrap();
        assert_eq!(screenshot.dimensions(), (2, 2));
        assert_eq!(screenshot.get_pixel(0, 0).0, [9, 0, 0, 255]);
        assert_eq!(screenshot.get_pixel(1, 1).0, [6, 0, 0, 255]);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("capture.png");
        write_screenshot_file(&path, &screenshot).unwrap();
        let decoded = image::open(&path).unwrap().to_rgba8();
        assert_eq!(decoded, screenshot);
        assert!(
            write_screenshot_file(&directory.path().join("missing/capture.png"), &screenshot)
                .is_err()
        );
    }

    #[test]
    #[ignore = "requires an X11 graphical display on Linux"]
    fn screenshot_captures_rendered_crt_pixels() {
        let config = Conf {
            window_title: "CRT screenshot test".to_owned(),
            window_width: 160,
            window_height: 120,
            high_dpi: true,
            platform: macroquad::miniquad::conf::Platform {
                linux_backend: macroquad::miniquad::conf::LinuxBackend::X11Only,
                ..Default::default()
            },
            ..Default::default()
        };
        macroquad::Window::from_config(config, async {
            let target = begin_screenshot_capture().unwrap();
            clear_background(BLACK);
            let screen = Rect::new(10.0, 10.0, 40.0, 40.0);
            draw_rectangle(10.0, 10.0, 40.0, 20.0, Color::from_rgba(200, 30, 40, 255));
            draw_rectangle(10.0, 30.0, 40.0, 20.0, Color::from_rgba(20, 210, 50, 255));
            let screenshot = finish_screenshot_capture(&target, screen).unwrap();
            draw_rectangle(10.0, 10.0, 40.0, 40.0, BLUE);

            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("capture.png");
            write_screenshot_file(&path, &screenshot).unwrap();
            let decoded = image::open(&path).unwrap().to_rgba8();
            let (width, height) = decoded.dimensions();
            assert_eq!(
                decoded.get_pixel(width / 2, height / 4).0,
                [200, 30, 40, 255]
            );
            assert_eq!(
                decoded.get_pixel(width / 2, height * 3 / 4).0,
                [20, 210, 50, 255]
            );

            let mut emulator = EmulatorCore::new();
            let mut pixels =
                vec![0; crate::emulator::SCREEN_WIDTH * crate::emulator::SCREEN_HEIGHT];
            pixels[..crate::emulator::SCREEN_WIDTH * crate::emulator::SCREEN_HEIGHT / 2].fill(255);
            emulator.write_display_buffer(&pixels).unwrap();
            let mut state = AppState::default();
            state.scanlines_enabled = false;
            state.crt_reflections_enabled = false;
            state.flicker_enabled = false;
            state.show_settings = true;
            state.show_debug = true;
            let mut experience = Experience::default();
            experience.screen = ScreenState::Ready;
            let mut app = App {
                bezel: TvAssets {
                    tv_cabinet: None,
                    crt_glass: None,
                    crt_reflections: None,
                    crt_shadow_mask: None,
                    tv_shadow: None,
                },
                background: None,
                title: None,
                crt: Crt::new(),
                state,
                experience,
                emulator,
                menu: Menu::default(),
                autoload: None,
                notification: None,
                paused: false,
                cycle_credit: 0.0,
                display_error_reported: false,
                discard_frame_time: false,
                pending_fullscreen: None,
                confirmation_selection: 0,
                keyboard_window: None,
                screenshot_requested: true,
                pending_screenshot: None,
                about: About::new(),
            };
            app.draw();
            let screenshot = app.pending_screenshot.take().unwrap();
            write_screenshot_file(&path, &screenshot).unwrap();
            let decoded = image::open(&path).unwrap().to_rgba8();
            let (width, height) = decoded.dimensions();
            assert_eq!(decoded.get_pixel(width / 2, height / 4).0, [255; 4]);
            assert_eq!(
                decoded.get_pixel(width / 2, height * 3 / 4).0,
                [0, 0, 0, 255]
            );
        });
    }

    #[test]
    fn screenshot_rejects_empty_or_invisible_crt() {
        let framebuffer = Image {
            width: 2,
            height: 2,
            bytes: vec![0; 16],
        };
        assert!(
            crop_crt_screenshot(&framebuffer, Rect::new(3.0, 0.0, 1.0, 1.0), 2.0, 2.0).is_err()
        );
        assert!(
            crop_crt_screenshot(&framebuffer, Rect::new(0.0, 0.0, 1.0, 1.0), 0.0, 2.0).is_err()
        );
        assert!(
            crop_crt_screenshot(&framebuffer, Rect::new(0.0, 0.0, 0.0, 1.0), 2.0, 2.0).is_err()
        );
    }
}
