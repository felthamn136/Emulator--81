use macroquad::prelude::*;

use crate::{
    assets::TvAssets,
    config::{ScreenCalibration, SCREEN_CALIBRATION},
    scaling::Scaling,
    ui, viewport,
};

/// The artwork and viewport share the original asset pack's 4096px coordinates.
pub const TV_CANVAS: Rect = Rect::new(75.0, -260.0, 1450.0, 1450.0);
pub const SCENE_BOUNDS: Rect = Rect::new(218.0, -12.0, 1165.0, 980.0);
const FALLBACK_BOUNDS: Rect = Rect::new(225.0, 70.0, 1150.0, 720.0);

pub fn aperture_rect(scaling: &Scaling) -> Rect {
    aperture_rect_with_calibration(scaling, &SCREEN_CALIBRATION)
}

fn aperture_rect_with_calibration(scaling: &Scaling, calibration: &ScreenCalibration) -> Rect {
    let scale = TV_CANVAS.w / 4096.0;
    let rect = Rect::new(
        TV_CANVAS.x + calibration.tv_screen_x * scale,
        TV_CANVAS.y + calibration.tv_screen_y * scale,
        calibration.tv_screen_width * scale,
        calibration.tv_screen_height * scale,
    );
    scaling.rect(rect)
}

pub fn viewport_rect(scaling: &Scaling) -> Rect {
    viewport_rect_with_calibration(scaling, &SCREEN_CALIBRATION)
}

fn viewport_rect_with_calibration(scaling: &Scaling, calibration: &ScreenCalibration) -> Rect {
    let aperture = aperture_rect_with_calibration(scaling, calibration);
    viewport::calculate_viewport(aperture)
}

pub fn draw_texture_layer(texture: &Texture2D, canvas: Rect, scaling: &Scaling) {
    let rect = scaling.rect(canvas);
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
}

/// Crop the full-canvas texture to the viewport without changing its alignment.
pub fn draw_viewport_layer(texture: &Texture2D, scaling: &Scaling, viewport: Rect) {
    draw_viewport_layer_tinted(texture, scaling, viewport, WHITE);
}

pub fn draw_viewport_layer_tinted(
    texture: &Texture2D,
    scaling: &Scaling,
    viewport: Rect,
    tint: Color,
) {
    let canvas = scaling.rect(TV_CANVAS);
    if let Some((destination, source)) =
        clipped_texture_rect(canvas, viewport, texture.width(), texture.height())
    {
        draw_texture_ex(
            texture,
            destination.x,
            destination.y,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(destination.w, destination.h)),
                source: Some(source),
                ..Default::default()
            },
        );
    }
}

fn clipped_texture_rect(canvas: Rect, clip: Rect, width: f32, height: f32) -> Option<(Rect, Rect)> {
    if canvas.w <= 0.0 || canvas.h <= 0.0 || clip.w <= 0.0 || clip.h <= 0.0 {
        return None;
    }
    let left = canvas.x.max(clip.x);
    let top = canvas.y.max(clip.y);
    let right = (canvas.x + canvas.w).min(clip.x + clip.w);
    let bottom = (canvas.y + canvas.h).min(clip.y + clip.h);
    if right <= left || bottom <= top {
        return None;
    }
    let dest = Rect::new(left, top, right - left, bottom - top);
    let source = Rect::new(
        (left - canvas.x) / canvas.w * width,
        (top - canvas.y) / canvas.h * height,
        dest.w / canvas.w * width,
        dest.h / canvas.h * height,
    );
    Some((dest, source))
}

pub fn draw_bezel_shadow(assets: &TvAssets, scaling: &Scaling) {
    if let Some(texture) = &assets.tv_shadow {
        draw_texture_layer(texture, TV_CANVAS, scaling);
    }
}

pub fn draw_tv_bezel(assets: &TvAssets, scaling: &Scaling, viewport: Rect) {
    if let Some(texture) = &assets.tv_cabinet {
        draw_texture_layer(texture, TV_CANVAS, scaling);
    } else {
        draw_fallback(scaling, viewport);
    }
}

pub fn draw_control_labels(scaling: &Scaling, font: &Font) {
    let asset_scale = TV_CANVAS.w * scaling.scale / 2048.0;
    let size = 27.0 * asset_scale;
    if size < 6.0 {
        return;
    }
    for (label, x) in [("CONTROLS", 690.0), ("POWER", 1541.0)] {
        let point = scaling.point(
            TV_CANVAS.x + x * TV_CANVAS.w / 2048.0,
            TV_CANVAS.y + 1550.0 * TV_CANVAS.h / 2048.0,
        );
        ui::text(
            label,
            point.x - ui::text_width(label, font, size) / 2.0,
            point.y,
            size,
            Color::from_rgba(187, 190, 185, 210),
            font,
        );
    }
}

fn power_indicator_color(brightness: f32) -> Color {
    let brightness = brightness.clamp(0.0, 1.0);
    Color::new(
        0.06 + 0.12 * brightness,
        0.10 + 0.43 * brightness,
        0.07 + 0.15 * brightness,
        1.0,
    )
}

pub fn draw_power_indicator(scaling: &Scaling, brightness: f32) {
    let center = scaling.point(
        TV_CANVAS.x + 1541.0 * TV_CANVAS.w / 2048.0,
        TV_CANVAS.y + 1601.0 * TV_CANVAS.h / 2048.0,
    );
    let radius = (14.0 * scaling.scale).clamp(4.0, 18.0);
    let brightness = brightness.clamp(0.0, 1.0);
    if brightness > 0.2 {
        draw_circle(
            center.x,
            center.y,
            radius * 1.6,
            Color::new(0.12, 0.55, 0.15, 0.06 * brightness),
        );
    }
    draw_circle(
        center.x,
        center.y,
        radius,
        power_indicator_color(brightness),
    );
    draw_circle(
        center.x - radius * 0.28,
        center.y - radius * 0.32,
        radius * 0.22,
        Color::new(1.0, 1.0, 1.0, 0.12 * brightness),
    );
}

pub fn draw_screen_backing(aperture: Rect) {
    let backing = screen_backing_rect(aperture);
    draw_rectangle(backing.x, backing.y, backing.w, backing.h, BLACK);
}

fn screen_backing_rect(aperture: Rect) -> Rect {
    let margin = aperture.h * 0.1;
    Rect::new(
        aperture.x - margin,
        aperture.y - margin,
        aperture.w + margin * 2.0,
        aperture.h + margin * 2.0,
    )
}

fn draw_fallback(scaling: &Scaling, viewport: Rect) {
    let outer = scaling.rect(FALLBACK_BOUNDS);
    let border = 8.0 * scaling.scale;
    // The opening remains clear if the layer order is customized.
    let dark = Color::from_rgba(27, 29, 30, 255);
    draw_rectangle(outer.x, outer.y, outer.w, viewport.y - outer.y, dark);
    draw_rectangle(
        outer.x,
        viewport.y + viewport.h,
        outer.w,
        outer.y + outer.h - viewport.y - viewport.h,
        dark,
    );
    draw_rectangle(outer.x, viewport.y, viewport.x - outer.x, viewport.h, dark);
    draw_rectangle(
        viewport.x + viewport.w,
        viewport.y,
        outer.x + outer.w - viewport.x - viewport.w,
        viewport.h,
        dark,
    );
    draw_rectangle_lines(outer.x, outer.y, outer.w, outer.h, border, GRAY);
    draw_rectangle_lines(
        viewport.x - border,
        viewport.y - border,
        viewport.w + border * 2.0,
        viewport.h + border * 2.0,
        border,
        GRAY,
    );
    let panel_x = outer.x + outer.w * 0.83;
    for i in 0..5 {
        draw_circle(
            panel_x,
            outer.y + outer.h * (0.18 + i as f32 * 0.11),
            17.0 * scaling.scale,
            GRAY,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_indicator_is_opaque_and_softens_the_lit_state() {
        let off = power_indicator_color(0.0);
        let dim = power_indicator_color(0.15);
        let on = power_indicator_color(1.0);
        assert_eq!((off.a, dim.a, on.a), (1.0, 1.0, 1.0));
        assert!(off.g < dim.g && dim.g < on.g);
        assert!(on.g < 0.6 && on.g > on.r && on.g > on.b);
        assert_eq!(power_indicator_color(-1.0), off);
        assert_eq!(power_indicator_color(2.0), on);
    }

    #[test]
    fn screen_backing_covers_translucent_bezel_edge() {
        let bounds = crate::menu::content_rect(1280.0, 720.0);
        let scaling = Scaling::fit(bounds, SCENE_BOUNDS);
        let aperture = aperture_rect(&scaling);
        let backing = screen_backing_rect(aperture);
        let image = Image::from_file_with_format(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/bezel/tv_cabinet.png"
            ))
            .unwrap(),
            None,
        )
        .unwrap();
        let edge = scaling.point(
            TV_CANVAS.x + 440.0 * TV_CANVAS.w / 2048.0,
            TV_CANVAS.y + 960.0 * TV_CANVAS.h / 2048.0,
        );
        assert!(image.get_pixel(440, 960).a < 0.99);
        assert!(!aperture.contains(edge));
        assert!(backing.contains(edge));
    }

    #[test]
    fn monitor_artwork_fits_scene_bounds() {
        let image = Image::from_file_with_format(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/bezel/tv_cabinet.png"
            ))
            .unwrap(),
            None,
        )
        .unwrap();
        let scale = TV_CANVAS.w / image.width() as f32;
        for y in 0..image.height() {
            for x in 0..image.width() {
                if image.get_pixel(x as u32, y as u32).a > 1.0 / 255.0 {
                    let left = TV_CANVAS.x + x as f32 * scale;
                    let top = TV_CANVAS.y + y as f32 * scale;
                    assert!(left >= SCENE_BOUNDS.x && top >= SCENE_BOUNDS.y);
                    assert!(left + scale <= SCENE_BOUNDS.x + SCENE_BOUNDS.w);
                    assert!(top + scale <= SCENE_BOUNDS.y + SCENE_BOUNDS.h);
                }
            }
        }
    }

    #[test]
    fn calibrated_opening_is_centered_and_remains_four_by_three() {
        let calibration = ScreenCalibration {
            tv_screen_x: 100.0,
            tv_screen_y: 200.0,
            tv_screen_width: 800.0,
            tv_screen_height: 800.0,
        };
        let design = Rect::new(0.0, 0.0, 1600.0, 1200.0);
        let rect = viewport_rect_with_calibration(&Scaling::fit(design, design), &calibration);
        assert!((rect.w / rect.h - 4.0 / 3.0).abs() < 0.00001);
        let aperture = aperture_rect_with_calibration(&Scaling::fit(design, design), &calibration);
        assert!((rect.y + rect.h / 2.0 - aperture.y - aperture.h / 2.0).abs() < 0.001);
    }

    #[test]
    fn default_screen_fills_the_aperture_without_distortion() {
        for (width, height) in [(1280.0, 720.0), (1600.0, 1200.0), (3840.0, 2160.0)] {
            let bounds = crate::menu::content_rect(width, height);
            let scaling = Scaling::fit(bounds, SCENE_BOUNDS);
            let aperture = aperture_rect(&scaling);
            let screen = viewport_rect(&scaling);
            assert!((screen.x - aperture.x).abs() < 0.001);
            assert!((screen.y - aperture.y).abs() < 0.001);
            assert!((screen.w - aperture.w).abs() < 0.001);
            assert!((screen.h - aperture.h).abs() < 0.001);
            assert!((screen.w / screen.h - 4.0 / 3.0).abs() < 0.00001);
        }
    }

    #[test]
    fn viewport_texture_crop_does_not_change_position_or_scale() {
        let (dest, source) = clipped_texture_rect(
            Rect::new(100.0, 200.0, 400.0, 400.0),
            Rect::new(150.0, 250.0, 200.0, 100.0),
            2048.0,
            2048.0,
        )
        .unwrap();
        assert_eq!(dest, Rect::new(150.0, 250.0, 200.0, 100.0));
        assert_eq!(source, Rect::new(256.0, 256.0, 1024.0, 512.0));
        assert!(clipped_texture_rect(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(200.0, 200.0, 10.0, 10.0),
            2048.0,
            2048.0,
        )
        .is_none());
    }
}
