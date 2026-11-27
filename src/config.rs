use macroquad::prelude::*;

pub const INITIAL_WINDOW_WIDTH: i32 = 800;
pub const INITIAL_WINDOW_HEIGHT: i32 = 600;

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
    Conf {
        window_title: "Emulator-81".to_owned(),
        window_width: INITIAL_WINDOW_WIDTH,
        window_height: INITIAL_WINDOW_HEIGHT,
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
    if flatpak.is_some_and(|id| id == "io.github.felthamn136.Emulator81") {
        "io.github.felthamn136.Emulator81"
    } else if snap.is_some_and(|name| name == "emulator-81") {
        "emulator-81_emulator-81"
    } else {
        "Emulator-81"
    }
}

pub(crate) fn application_icon() -> macroquad::miniquad::conf::Icon {
    let image = image::load_from_memory(include_bytes!("../packaging/emulator-81-icon.png"))
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
    fn window_is_resizable_with_four_by_three_content() {
        let config = window_conf();
        assert_eq!(config.window_title, "Emulator-81");
        assert!(config.window_resizable);
        assert_eq!(config.window_aspect_ratio, Some((4, 3)));
        assert_eq!((config.window_width, config.window_height), (800, 600));
    }

    #[test]
    fn windows_use_packaged_icon_and_matching_linux_identity() {
        let expected = image::load_from_memory(include_bytes!("../packaging/emulator-81-icon.png"))
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
        assert_eq!(window_class_for(None, None), "Emulator-81");
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
