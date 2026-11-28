use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{
    app::AppState,
    experience::{CrtWarmupSettings, Experience},
};

pub const INITIAL_WINDOW_WIDTH: i32 = 800;
pub const INITIAL_WINDOW_HEIGHT: i32 = 600;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct Preferences {
    window_width: i32,
    window_height: i32,
    fullscreen: bool,
    fullscreen_confirmation: bool,
    show_grid_background: bool,
    screen_focused: bool,
    warmup_enabled: bool,
    scanlines_enabled: bool,
    scanline_strength: f32,
    crt_reflections_enabled: bool,
    crt_curvature_enabled: bool,
    bloom_enabled: bool,
    glow_enabled: bool,
    flicker_enabled: bool,
    tape_sound: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        let state = AppState::default();
        Self {
            window_width: INITIAL_WINDOW_WIDTH,
            window_height: INITIAL_WINDOW_HEIGHT,
            fullscreen: state.fullscreen,
            fullscreen_confirmation: state.fullscreen_confirmation,
            show_grid_background: state.show_grid_background,
            screen_focused: state.screen_focused,
            warmup_enabled: CrtWarmupSettings::default().enabled,
            scanlines_enabled: state.scanlines_enabled,
            scanline_strength: state.scanline_strength,
            crt_reflections_enabled: state.crt_reflections_enabled,
            crt_curvature_enabled: state.crt_curvature_enabled,
            bloom_enabled: state.bloom_enabled,
            glow_enabled: state.glow_enabled,
            flicker_enabled: state.flicker_enabled,
            tape_sound: state.tape_sound,
        }
    }
}

impl Preferences {
    pub(crate) fn apply(&self, state: &mut AppState, experience: &mut Experience) {
        state.fullscreen = self.fullscreen;
        state.fullscreen_confirmation = self.fullscreen_confirmation;
        state.show_grid_background = self.show_grid_background;
        state.screen_focused = self.screen_focused;
        state.scanlines_enabled = self.scanlines_enabled;
        state.scanline_strength = self.scanline_strength;
        state.crt_reflections_enabled = self.crt_reflections_enabled;
        state.crt_curvature_enabled = self.crt_curvature_enabled;
        state.bloom_enabled = self.bloom_enabled;
        state.glow_enabled = self.glow_enabled;
        state.flicker_enabled = self.flicker_enabled;
        state.tape_sound = self.tape_sound;
        experience.settings.enabled = self.warmup_enabled;
        if !self.warmup_enabled {
            experience.reset();
        }
    }

    pub(crate) fn capture(
        &mut self,
        state: &AppState,
        experience: &Experience,
        size: Option<(i32, i32)>,
    ) {
        if let Some((width, height)) = size.filter(|_| !state.fullscreen) {
            if (160..=16384).contains(&width) && (120..=16384).contains(&height) {
                self.window_width = width;
                self.window_height = height;
            }
        }
        self.fullscreen = state.fullscreen;
        self.fullscreen_confirmation = state.fullscreen_confirmation;
        self.show_grid_background = state.show_grid_background;
        self.screen_focused = state.screen_focused;
        self.warmup_enabled = experience.settings.enabled;
        self.scanlines_enabled = state.scanlines_enabled;
        self.scanline_strength = state.scanline_strength;
        self.crt_reflections_enabled = state.crt_reflections_enabled;
        self.crt_curvature_enabled = state.crt_curvature_enabled;
        self.bloom_enabled = state.bloom_enabled;
        self.glow_enabled = state.glow_enabled;
        self.flicker_enabled = state.flicker_enabled;
        self.tape_sound = state.tape_sound;
    }

    pub(crate) fn tape_sound(&self) -> bool {
        self.tape_sound
    }

    pub(crate) fn window_size(&self) -> (i32, i32) {
        (self.window_width, self.window_height)
    }

    fn validate(&self) -> Result<(), String> {
        if !(160..=16384).contains(&self.window_width)
            || !(120..=16384).contains(&self.window_height)
        {
            return Err("window size is outside the supported range".into());
        }
        if !self.scanline_strength.is_finite() || !(0.0..=1.0).contains(&self.scanline_strength) {
            return Err("scanline strength must be between 0 and 1".into());
        }
        Ok(())
    }
}

pub(crate) fn preferences_path() -> Result<PathBuf, String> {
    dirs::config_dir()
        .map(|dir| dir.join("membrane81").join("preferences.json"))
        .ok_or_else(|| "Cannot determine the user configuration directory".to_owned())
}

fn read_preferences(path: &Path) -> Result<Option<Preferences>, String> {
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Cannot read preferences {}: {error}",
                path.display()
            ))
        }
    };
    let preferences: Preferences = serde_json::from_slice(&data)
        .map_err(|error| format!("Cannot parse preferences {}: {error}", path.display()))?;
    preferences
        .validate()
        .map_err(|error| format!("Invalid preferences {}: {error}", path.display()))?;
    Ok(Some(preferences))
}

fn load_preferences_with_fallback(path: &Path, legacy_path: &Path) -> Result<Preferences, String> {
    if let Some(preferences) = read_preferences(path)? {
        return Ok(preferences);
    }
    Ok(read_preferences(legacy_path)?.unwrap_or_default())
}

pub(crate) fn startup_preferences() -> &'static Result<Preferences, String> {
    static PREFERENCES: OnceLock<Result<Preferences, String>> = OnceLock::new();
    PREFERENCES.get_or_init(|| {
        let directory = dirs::config_dir()
            .ok_or_else(|| "Cannot determine the user configuration directory".to_owned())?;
        load_preferences_with_fallback(
            &directory.join("membrane81").join("preferences.json"),
            &directory.join("emulator-81").join("preferences.json"),
        )
    })
}

pub(crate) fn save_preferences(path: &Path, preferences: &Preferences) -> Result<(), String> {
    preferences.validate()?;
    let directory = path
        .parent()
        .ok_or_else(|| format!("Invalid preferences path {}", path.display()))?;
    std::fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Cannot create preferences directory {}: {error}",
            directory.display()
        )
    })?;
    let mut file = tempfile::NamedTempFile::new_in(directory).map_err(|error| {
        format!(
            "Cannot create preferences file in {}: {error}",
            directory.display()
        )
    })?;
    serde_json::to_writer_pretty(&mut file, preferences)
        .map_err(|error| format!("Cannot serialize preferences: {error}"))?;
    file.write_all(b"\n")
        .and_then(|_| file.flush())
        .map_err(|error| format!("Cannot write preferences {}: {error}", path.display()))?;
    file.persist(path).map_err(|error| {
        format!(
            "Cannot save preferences {}: {}",
            path.display(),
            error.error
        )
    })?;
    Ok(())
}

/// Screen aperture coordinates in the original 4096-square TV artwork.
pub struct ScreenCalibration {
    pub tv_screen_x: f32,
    pub tv_screen_y: f32,
    pub tv_screen_width: f32,
    pub tv_screen_height: f32,
}

pub const SCREEN_CALIBRATION: ScreenCalibration = ScreenCalibration {
    tv_screen_x: 976.0,
    tv_screen_y: 1124.0,
    tv_screen_width: 2120.0,
    tv_screen_height: 1590.0,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderLayer {
    TvShadow,
    TvCabinet,
    Viewport,
    CrtShadowMask,
    CrtGlass,
    CrtReflections,
}

/// Change this list to reorder the independently rendered artwork layers.
pub const RENDER_ORDER: [RenderLayer; 6] = [
    RenderLayer::TvShadow,
    RenderLayer::TvCabinet,
    RenderLayer::Viewport,
    RenderLayer::CrtShadowMask,
    RenderLayer::CrtGlass,
    RenderLayer::CrtReflections,
];

pub fn window_conf() -> Conf {
    if crate::virtual_keyboard::is_keyboard_process() {
        return crate::virtual_keyboard::window_conf();
    }
    let preferences = match startup_preferences() {
        Ok(preferences) => preferences.clone(),
        Err(error) => {
            eprintln!("{error}");
            Preferences::default()
        }
    };
    window_conf_for(&preferences)
}

fn window_conf_for(preferences: &Preferences) -> Conf {
    Conf {
        window_title: "Membrane81".to_owned(),
        window_width: preferences.window_width,
        window_height: preferences.window_height,
        fullscreen: preferences.fullscreen,
        window_resizable: true,
        window_aspect_ratio: Some((4, 3)),
        high_dpi: true,
        icon: Some(application_icon()),
        platform: macroquad::miniquad::conf::Platform {
            swap_interval: Some(1),
            linux_backend: macroquad::miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            linux_wm_class: application_window_class(),
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn application_window_class() -> &'static str {
    window_class_for(
        std::env::var_os("FLATPAK_ID").as_deref(),
        std::env::var_os("SNAP_INSTANCE_NAME").as_deref(),
    )
}

fn window_class_for(
    flatpak: Option<&std::ffi::OsStr>,
    snap: Option<&std::ffi::OsStr>,
) -> &'static str {
    if flatpak.is_some_and(|id| id == "io.github.felthamn136.Membrane81") {
        "io.github.felthamn136.Membrane81"
    } else if flatpak.is_some_and(|id| id == "io.github.felthamn136.Emulator81") {
        "io.github.felthamn136.Emulator81"
    } else if snap.is_some_and(|name| name == "membrane81") {
        "membrane81_membrane81"
    } else if snap.is_some_and(|name| name == "emulator-81") {
        "emulator-81_emulator-81"
    } else {
        "Membrane81"
    }
}

pub(crate) fn application_icon() -> macroquad::miniquad::conf::Icon {
    let image = image::load_from_memory(include_bytes!("../packaging/membrane81-icon.png"))
        .expect("bundled application icon must be a valid PNG")
        .to_rgba8();
    fn pixels<const N: usize>(image: &image::RgbaImage, size: u32) -> [u8; N] {
        image::imageops::resize(image, size, size, image::imageops::FilterType::Lanczos3)
            .into_raw()
            .try_into()
            .expect("resized application icon must have the expected dimensions")
    }
    macroquad::miniquad::conf::Icon {
        small: pixels(&image, 16),
        medium: pixels(&image, 32),
        big: pixels(&image, 64),
    }
}

/// Miniquad centers macOS windows itself. X11 and Windows can be positioned
/// explicitly; Wayland compositors control window placement instead.
pub fn center_window() {
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        #[cfg(target_os = "linux")]
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            eprintln!("Window placement is controlled by the Wayland compositor.");
            return;
        }

        match display_info::DisplayInfo::all() {
            Ok(displays) => {
                if let Some(display) = displays.iter().find(|d| d.is_primary).or(displays.first()) {
                    let (width, height) = macroquad::miniquad::window::screen_size();
                    let x = display.x as i64 + (display.width as i64 - width as i64).max(0) / 2;
                    let y = display.y as i64 + (display.height as i64 - height as i64).max(0) / 2;
                    if let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) {
                        macroquad::miniquad::window::set_window_position(x, y);
                    } else {
                        eprintln!("Cannot center window on a monitor with negative coordinates.");
                    }
                } else {
                    eprintln!("Cannot center window: no display was reported.");
                }
            }

            Err(error) => eprintln!("Cannot center window: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_round_trip_and_restore_runtime_settings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings").join("preferences.json");
        let mut state = AppState::default();
        state.fullscreen = false;
        state.fullscreen_confirmation = false;
        state.show_grid_background = false;
        state.screen_focused = true;
        state.scanlines_enabled = false;
        state.scanline_strength = 0.3;
        state.crt_reflections_enabled = false;
        state.crt_curvature_enabled = true;
        state.bloom_enabled = false;
        state.glow_enabled = false;
        state.flicker_enabled = false;
        state.tape_sound = true;
        let mut experience = Experience::default();
        experience.settings.enabled = false;
        let mut preferences = Preferences::default();
        preferences.capture(&state, &experience, Some((960, 720)));
        state.fullscreen = true;
        preferences.capture(&state, &experience, Some((1920, 1080)));
        save_preferences(&path, &preferences).unwrap();
        let loaded = read_preferences(&path).unwrap().unwrap();
        assert_eq!(loaded, preferences);
        let window = window_conf_for(&loaded);
        assert_eq!((window.window_width, window.window_height), (960, 720));
        assert!(window.fullscreen);
        let mut restored = AppState::default();
        let mut restored_experience = Experience::default();
        loaded.apply(&mut restored, &mut restored_experience);
        assert!(loaded.tape_sound());
        assert!(restored.tape_sound);
        let mut recaptured = Preferences::default();
        restored.fullscreen = false;
        recaptured.capture(&restored, &restored_experience, Some((960, 720)));
        restored.fullscreen = true;
        recaptured.capture(&restored, &restored_experience, None);
        assert_eq!(recaptured, preferences);
        assert_eq!(
            restored_experience.screen,
            crate::experience::ScreenState::Ready
        );
    }

    #[test]
    fn preferences_default_missing_fields_and_reject_invalid_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("preferences.json");
        assert_eq!(read_preferences(&path).unwrap(), None);
        std::fs::write(&path, r#"{"scanlines_enabled":false}"#).unwrap();
        let loaded = read_preferences(&path).unwrap().unwrap();
        assert!(!loaded.scanlines_enabled);
        assert_eq!(loaded.window_width, INITIAL_WINDOW_WIDTH);
        std::fs::write(&path, r#"{"scanline_strength":2}"#).unwrap();
        assert!(read_preferences(&path).is_err());
        std::fs::write(&path, r#"{"window_width":0}"#).unwrap();
        assert!(read_preferences(&path).is_err());
        std::fs::write(&path, b"{").unwrap();
        assert!(read_preferences(&path).is_err());
    }

    #[test]
    fn renamed_preferences_use_legacy_only_when_new_file_is_missing() {
        let directory = tempfile::tempdir().unwrap();
        let legacy = directory
            .path()
            .join("emulator-81")
            .join("preferences.json");
        let current = directory.path().join("membrane81").join("preferences.json");
        let mut old = Preferences::default();
        old.scanlines_enabled = false;
        old.scanline_strength = 0.3;
        save_preferences(&legacy, &old).unwrap();
        assert_eq!(
            load_preferences_with_fallback(&current, &legacy).unwrap(),
            old
        );

        let mut new = old.clone();
        new.scanlines_enabled = true;
        save_preferences(&current, &new).unwrap();
        assert_eq!(
            load_preferences_with_fallback(&current, &legacy).unwrap(),
            new
        );
        std::fs::write(&current, b"{").unwrap();
        assert!(load_preferences_with_fallback(&current, &legacy).is_err());
        assert_eq!(read_preferences(&legacy).unwrap(), Some(old));
    }

    #[test]
    fn window_is_resizable_with_four_by_three_content() {
        let config = window_conf_for(&Preferences::default());
        assert_eq!(config.window_title, "Membrane81");
        assert!(config.window_resizable);
        assert_eq!(config.window_aspect_ratio, Some((4, 3)));
        assert_eq!((config.window_width, config.window_height), (800, 600));
    }

    #[test]
    fn windows_use_packaged_icon_and_matching_linux_identity() {
        let expected = image::load_from_memory(include_bytes!("../packaging/membrane81-icon.png"))
            .unwrap()
            .to_rgba8();
        let expected =
            image::imageops::resize(&expected, 64, 64, image::imageops::FilterType::Lanczos3)
                .into_raw();
        for config in [window_conf(), crate::virtual_keyboard::window_conf()] {
            assert_eq!(config.icon.unwrap().big.as_slice(), expected);
            assert_eq!(config.platform.linux_wm_class, application_window_class());
        }
        use std::ffi::OsStr;
        assert_eq!(window_class_for(None, None), "Membrane81");
        assert_eq!(
            window_class_for(Some(OsStr::new("io.github.felthamn136.Membrane81")), None),
            "io.github.felthamn136.Membrane81"
        );
        assert_eq!(
            window_class_for(None, Some(OsStr::new("membrane81"))),
            "membrane81_membrane81"
        );
        assert_eq!(
            window_class_for(Some(OsStr::new("io.github.felthamn136.Emulator81")), None),
            "io.github.felthamn136.Emulator81"
        );
        assert_eq!(
            window_class_for(None, Some(OsStr::new("emulator-81"))),
            "emulator-81_emulator-81"
        );
    }
}
