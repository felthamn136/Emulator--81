use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenState {
    PowerOff,
    Booting,
    Ready,
    Searching,
    Loading,
    Loaded,
    Inserted,
    Shutdown,
}

#[derive(Clone, Copy, Debug)]
pub struct CrtWarmupSettings {
    pub enabled: bool,
    pub startup_duration: f32,
    pub bloom_strength: f32,
    pub flicker_strength: f32,
    pub scanline_fade_time: f32,
    pub randomise_startup: bool,
}

impl Default for CrtWarmupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            startup_duration: 3.5,
            bloom_strength: 0.3,
            flicker_strength: 0.035,
            scanline_fade_time: 1.6,
            randomise_startup: true,
        }
    }
}

const SEARCH_TIME: f32 = 1.2;
const LOAD_TIME: f32 = 4.8;
const LOADED_TIME: f32 = 1.0;
const INSERTED_TIME: f32 = 3.0;
const SHUTDOWN_TIME: f32 = 1.5;
const POWER_OFF_TIME: f32 = 0.15;

pub struct Experience {
    pub screen: ScreenState,
    pub settings: CrtWarmupSettings,
    pub elapsed: f32,
    pub loading_progress: f32,
    pub filename: Option<String>,
    pub startup_variation: f32,
    cycle_after_shutdown: bool,
}

impl Default for Experience {
    fn default() -> Self {
        let mut experience = Self {
            screen: ScreenState::PowerOff,
            settings: CrtWarmupSettings::default(),
            elapsed: 0.0,
            loading_progress: 0.0,
            filename: None,
            startup_variation: 0.0,
            cycle_after_shutdown: false,
        };
        experience.start_power_on();
        experience
    }
}

impl Experience {
    pub fn start_power_on(&mut self) {
        self.screen = if self.settings.enabled {
            ScreenState::Booting
        } else {
            ScreenState::Ready
        };
        self.elapsed = 0.0;
        self.startup_variation = if self.settings.randomise_startup {
            macroquad::rand::gen_range(-1.0, 1.0)
        } else {
            0.0
        };
        self.cycle_after_shutdown = false;
    }

    pub fn start_shutdown(&mut self, cycle: bool) {
        if self.screen != ScreenState::Shutdown {
            self.screen = ScreenState::Shutdown;
            self.elapsed = 0.0;
        }
        self.cycle_after_shutdown = cycle;
    }

    pub fn reset(&mut self) {
        self.screen = ScreenState::Ready;
        self.elapsed = 0.0;
        self.loading_progress = 0.0;
        self.cycle_after_shutdown = false;
    }

    pub fn toggle_loading(&mut self) -> bool {
        match self.screen {
            ScreenState::Searching | ScreenState::Loading => {
                self.reset();
                false
            }
            ScreenState::Ready | ScreenState::Loaded | ScreenState::Inserted => {
                self.screen = ScreenState::Searching;
                self.elapsed = 0.0;
                self.loading_progress = 0.0;
                true
            }
            _ => false,
        }
    }

    pub fn insert_program(&mut self, path: &Path) -> Result<(), &'static str> {
        let valid = path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| {
                ["p", "p81", "tap"]
                    .iter()
                    .any(|supported| ext.eq_ignore_ascii_case(supported))
            });
        if !valid {
            return Err("unsupported file type (expected .p, .p81 or .tap)");
        }
        if matches!(
            self.screen,
            ScreenState::Booting | ScreenState::Shutdown | ScreenState::PowerOff
        ) {
            return Err("wait until the television is ready before inserting a program");
        }
        let filename = path.file_name().ok_or("file has no name")?;
        self.filename = Some(filename.to_string_lossy().into_owned());
        self.screen = ScreenState::Inserted;
        self.elapsed = 0.0;
        self.loading_progress = 0.0;
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.screen, ScreenState::Searching | ScreenState::Loading)
    }

    pub fn startup_fraction(&self) -> f32 {
        (self.elapsed / self.settings.startup_duration.max(0.01)).clamp(0.0, 1.0)
    }

    pub fn shutdown_fraction(&self) -> f32 {
        (self.elapsed / SHUTDOWN_TIME).clamp(0.0, 1.0)
    }

    pub fn power_indicator_brightness(&self) -> f32 {
        if matches!(self.screen, ScreenState::Shutdown | ScreenState::PowerOff)
            && self.cycle_after_shutdown
        {
            if (self.elapsed * 8.0).sin() >= 0.0 {
                1.0
            } else {
                0.15
            }
        } else if matches!(self.screen, ScreenState::PowerOff | ScreenState::Shutdown) {
            0.0
        } else {
            1.0
        }
    }

    /// Advances through every crossed boundary and returns actual tape-playing time.
    pub fn update(&mut self, delta: f32) -> f32 {
        if !delta.is_finite() || delta <= 0.0 {
            return 0.0;
        }
        let mut remaining = delta;
        let mut tape_time = 0.0;
        while remaining > 0.0 {
            let duration = match self.screen {
                ScreenState::Booting => self.settings.startup_duration.max(0.01),
                ScreenState::Searching => SEARCH_TIME,
                ScreenState::Loading => LOAD_TIME,
                ScreenState::Loaded => LOADED_TIME,
                ScreenState::Inserted => INSERTED_TIME,
                ScreenState::Shutdown => SHUTDOWN_TIME,
                ScreenState::PowerOff if self.cycle_after_shutdown => POWER_OFF_TIME,
                ScreenState::PowerOff | ScreenState::Ready => break,
            };
            let step = remaining.min((duration - self.elapsed).max(0.0));
            if self.is_playing() {
                tape_time += step;
            }
            self.elapsed += step;
            remaining -= step;
            if self.screen == ScreenState::Loading {
                self.loading_progress = (self.elapsed / LOAD_TIME).clamp(0.0, 1.0);
            }
            if self.elapsed < duration {
                break;
            }
            self.elapsed = 0.0;
            self.screen = match self.screen {
                ScreenState::Booting => ScreenState::Ready,
                ScreenState::Searching => ScreenState::Loading,
                ScreenState::Loading => ScreenState::Loaded,
                ScreenState::Loaded | ScreenState::Inserted => ScreenState::Ready,
                ScreenState::Shutdown => ScreenState::PowerOff,
                ScreenState::PowerOff => {
                    self.start_power_on();
                    self.screen
                }
                ScreenState::Ready => ScreenState::Ready,
            };
        }
        tape_time
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_load_and_stop_follow_timed_states() {
        let mut experience = Experience::default();
        experience.update(3.49);
        assert_eq!(experience.screen, ScreenState::Booting);
        experience.update(0.02);
        assert_eq!(experience.screen, ScreenState::Ready);
        assert!(experience.toggle_loading());
        assert!((experience.update(3.6) - 3.6).abs() < 0.001);
        assert_eq!(experience.screen, ScreenState::Loading);
        assert!((experience.loading_progress - 0.5).abs() < 0.001);
        assert!((experience.update(5.0) - 2.4).abs() < 0.001);
        assert_eq!(experience.screen, ScreenState::Ready);
        assert_eq!(experience.loading_progress, 1.0);
        assert!(experience.toggle_loading());
        assert!(!experience.toggle_loading());
        assert_eq!(experience.screen, ScreenState::Ready);
    }

    #[test]
    fn power_cycle_and_disabled_warmup() {
        let mut experience = Experience::default();
        experience.start_shutdown(true);
        experience.update(1.6);
        assert_eq!(experience.screen, ScreenState::PowerOff);
        experience.update(0.1);
        assert_eq!(experience.screen, ScreenState::Booting);
        experience.update(3.5);
        assert_eq!(experience.screen, ScreenState::Ready);
        experience.settings.enabled = false;
        experience.start_shutdown(true);
        experience.update(1.7);
        assert_eq!(experience.screen, ScreenState::Ready);
    }

    #[test]
    fn dropped_program_validates_extension_and_does_not_load_code() {
        let mut experience = Experience::default();
        experience.reset();
        assert!(experience.insert_program(Path::new("demo.P81")).is_ok());
        assert_eq!(experience.filename.as_deref(), Some("demo.P81"));
        assert_eq!(experience.screen, ScreenState::Inserted);
        assert!(experience.insert_program(Path::new("demo.exe")).is_err());
        experience.update(3.1);
        assert_eq!(experience.screen, ScreenState::Ready);
    }
}
