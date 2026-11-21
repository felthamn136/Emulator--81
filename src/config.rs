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
        window_title: "ZX81 Emulator Project Prototype".to_owned(),
        window_width: INITIAL_WINDOW_WIDTH,
        window_height: INITIAL_WINDOW_HEIGHT,
        window_resizable: true,
        window_aspect_ratio: Some((4, 3)),
        high_dpi: true,
        platform: macroquad::miniquad::conf::Platform {
            swap_interval: Some(1),
            linux_backend: macroquad::miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            ..Default::default()
        },
        ..Default::default()
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
        assert!(config.window_resizable);
        assert_eq!(config.window_aspect_ratio, Some((4, 3)));
        assert_eq!((config.window_width, config.window_height), (800, 600));
    }
}
