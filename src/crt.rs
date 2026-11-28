use macroquad::prelude::*;

use crate::{
    assets::TvAssets,
    bezel,
    emulator::LINE_T_STATES,
    experience::{CrtWarmupSettings, ScreenState},
    scaling::Scaling,
    tape::Tape,
};

const LOADING_LINES: u32 = 192;
const LOADING_SAMPLES: u32 = 64;
const SCREEN_IMAGE_SCALE: f32 = 0.96;

pub fn image_rect(screen: Rect) -> Rect {
    let width = screen.w * SCREEN_IMAGE_SCALE;
    let height = screen.h * SCREEN_IMAGE_SCALE;
    Rect::new(
        screen.x + (screen.w - width) / 2.0,
        screen.y + (screen.h - height) / 2.0,
        width,
        height,
    )
}

fn rounded_rect(rect: Rect, radius: f32, color: Color) {
    draw_rectangle(
        rect.x + radius,
        rect.y,
        rect.w - radius * 2.0,
        rect.h,
        color,
    );
    draw_rectangle(
        rect.x,
        rect.y + radius,
        rect.w,
        rect.h - radius * 2.0,
        color,
    );
    for (x, y) in [
        (rect.x + radius, rect.y + radius),
        (rect.x + rect.w - radius, rect.y + radius),
        (rect.x + radius, rect.y + rect.h - radius),
        (rect.x + rect.w - radius, rect.y + rect.h - radius),
    ] {
        draw_circle(x, y, radius, color);
    }
}

fn rounded_scanline(rect: Rect, radius: f32, y: f32, height: f32, color: Color) {
    let top = y.max(rect.y);
    let bottom = (y + height).min(rect.y + rect.h);
    if bottom <= top {
        return;
    }
    let edge_distance = (top - rect.y).min(rect.y + rect.h - bottom);
    let corner_distance = (radius - edge_distance).max(0.0);
    let inset = radius
        - (radius * radius - corner_distance * corner_distance)
            .max(0.0)
            .sqrt();
    draw_rectangle(
        rect.x + inset,
        top,
        rect.w - 2.0 * inset,
        bottom - top,
        color,
    );
}

pub struct CrtVisuals {
    pub brightness: f32,
    pub scanlines: f32,
    pub reflections: f32,
}

pub fn visuals(
    state: ScreenState,
    elapsed: f32,
    settings: &CrtWarmupSettings,
    variation: f32,
) -> CrtVisuals {
    match state {
        ScreenState::Booting => {
            let t = (elapsed / settings.startup_duration.max(0.01)).clamp(0.0, 1.0);
            let text = ((t - 1.5 / 3.5) / (2.0 / 3.5)).clamp(0.0, 1.0);
            let scanlines = ((elapsed - settings.startup_duration * 1.5 / 3.5)
                / settings.scanline_fade_time.max(0.01))
            .clamp(0.0, 1.0);
            let pulse = 1.0
                + (elapsed * (19.0 + variation) + variation).sin()
                    * settings.flicker_strength
                    * (1.0 - text);
            CrtVisuals {
                brightness: (text * pulse).clamp(0.0, 1.0),
                scanlines,
                reflections: text,
            }
        }
        ScreenState::PowerOff | ScreenState::Shutdown => CrtVisuals {
            brightness: 0.0,
            scanlines: 0.0,
            reflections: 0.0,
        },
        _ => CrtVisuals {
            brightness: 1.0,
            scanlines: 1.0,
            reflections: 1.0,
        },
    }
}

pub struct Crt;

impl Crt {
    pub fn new() -> Self {
        Self
    }

    pub fn draw_screen_frame(&self, screen: Rect, luminance: f32) {
        if screen.w <= 0.0 || screen.h <= 0.0 {
            return;
        }
        let radius = screen.w.min(screen.h) * 0.06;
        let border = screen.w.min(screen.h) * 0.004;
        let luminance = luminance.clamp(0.0, 1.0);
        rounded_rect(screen, radius, Color::from_rgba(54, 57, 57, 255));
        rounded_rect(
            Rect::new(
                screen.x + border,
                screen.y + border,
                screen.w - border * 2.0,
                screen.h - border * 2.0,
            ),
            radius - border,
            Color::new(luminance, luminance, luminance, 1.0),
        );
    }

    pub fn draw_loading_lines(&self, viewport: Rect, tape: &Tape, brightness: f32) {
        let line_height = viewport.h / LOADING_LINES as f32;
        let sample_width = viewport.w / LOADING_SAMPLES as f32;
        let white = Color::new(1.0, 1.0, 1.0, brightness.clamp(0.0, 1.0));
        draw_rectangle(viewport.x, viewport.y, viewport.w, viewport.h, BLACK);
        for line in 0..LOADING_LINES {
            let mut start = None;
            for column in 0..=LOADING_SAMPLES {
                let high =
                    column < LOADING_SAMPLES && tape.level_ago(loading_sample_ago(line, column));
                if high && start.is_none() {
                    start = Some(column);
                } else if !high {
                    if let Some(first) = start.take() {
                        draw_rectangle(
                            viewport.x + first as f32 * sample_width,
                            viewport.y + line as f32 * line_height,
                            (column - first) as f32 * sample_width,
                            line_height,
                            white,
                        );
                    }
                }
            }
        }
    }

    pub fn draw_scanlines(&self, viewport: Rect, opacity: f32) {
        if viewport.w <= 0.0 || viewport.h <= 0.0 {
            return;
        }
        const LINES: usize = 240;
        let spacing = viewport.h / LINES as f32;
        let border = viewport.w.min(viewport.h) * 0.004;
        let radius = viewport.w.min(viewport.h) * 0.06 - border;
        let interior = Rect::new(
            viewport.x + border,
            viewport.y + border,
            viewport.w - 2.0 * border,
            viewport.h - 2.0 * border,
        );
        let pixel = 1.0 / screen_dpi_scale();
        let physical_spacing = spacing / pixel;
        let strength = scanline_strength(physical_spacing, opacity);
        let center_height = (spacing * 0.2).max(pixel).min(spacing);
        let side_height = ((spacing - center_height) / 2.0).min(pixel);
        let side_strength = (physical_spacing - 2.0).clamp(0.0, 1.0);
        for line in 0..LINES {
            let y = viewport.y + (line as f32 + 0.5) * spacing - center_height / 2.0;
            rounded_scanline(
                interior,
                radius,
                y,
                center_height,
                Color::new(0.0, 0.0, 0.0, 0.09 * strength),
            );
            rounded_scanline(
                interior,
                radius,
                y - side_height,
                side_height,
                Color::new(0.0, 0.0, 0.0, 0.035 * strength * side_strength),
            );
            rounded_scanline(
                interior,
                radius,
                y + center_height,
                side_height,
                Color::new(0.0, 0.0, 0.0, 0.035 * strength * side_strength),
            );
        }
    }

    pub fn draw_soft_edges(&self, viewport: Rect) {
        const BANDS: usize = 8;
        let edge = (viewport.w.min(viewport.h) * 0.025).max(0.0);
        for band in 0..BANDS {
            let inset = edge * band as f32 / BANDS as f32;
            let thickness = edge / BANDS as f32;
            let alpha = 0.15 * (1.0 - (band as f32 + 0.5) / BANDS as f32).powi(2);
            let color = Color::new(0.0, 0.0, 0.0, alpha);
            draw_rectangle(
                viewport.x + inset,
                viewport.y + inset,
                viewport.w - 2.0 * inset,
                thickness,
                color,
            );
            draw_rectangle(
                viewport.x + inset,
                viewport.y + viewport.h - inset - thickness,
                viewport.w - 2.0 * inset,
                thickness,
                color,
            );
            draw_rectangle(
                viewport.x + inset,
                viewport.y + inset + thickness,
                thickness,
                viewport.h - 2.0 * (inset + thickness),
                color,
            );
            draw_rectangle(
                viewport.x + viewport.w - inset - thickness,
                viewport.y + inset + thickness,
                thickness,
                viewport.h - 2.0 * (inset + thickness),
                color,
            );
        }
    }

    pub fn draw_vignette(&self, assets: &TvAssets, scaling: &Scaling, viewport: Rect) {
        if let Some(texture) = &assets.crt_shadow_mask {
            bezel::draw_viewport_layer(texture, scaling, viewport);
        }
    }

    pub fn draw_reflections(
        &self,
        assets: &TvAssets,
        scaling: &Scaling,
        viewport: Rect,
        opacity: f32,
    ) {
        if let Some(texture) = &assets.crt_reflections {
            bezel::draw_viewport_layer_tinted(
                texture,
                scaling,
                viewport,
                Color::new(1.0, 1.0, 1.0, opacity),
            );
        } else {
            draw_line(
                viewport.x + viewport.w * 0.08,
                viewport.y + viewport.h * 0.12,
                viewport.x + viewport.w * 0.72,
                viewport.y + viewport.h * 0.32,
                2.0 * scaling.scale,
                Color::new(1.0, 1.0, 1.0, 0.08 * opacity),
            );
        }
    }

    pub fn draw_glass(&self, assets: &TvAssets, scaling: &Scaling, viewport: Rect, opacity: f32) {
        if let Some(texture) = &assets.crt_glass {
            bezel::draw_viewport_layer_tinted(
                texture,
                scaling,
                viewport,
                Color::new(1.0, 1.0, 1.0, opacity),
            );
        }
    }

    pub fn render_power_on(&self, viewport: Rect, fraction: f32, bloom: f32, wobble: f32) {
        let cold = 0.5 / 3.5;
        let line = 0.75 / 3.5;
        let expansion = 1.5 / 3.5;
        if fraction < cold || fraction >= expansion {
            return;
        }
        let (width, height, opacity) = if fraction < line {
            let p = (fraction - cold) / (line - cold);
            (
                viewport.w * p.max(0.01),
                (viewport.h * 0.006).max(1.0).min(viewport.h),
                0.82,
            )
        } else {
            let p = (fraction - line) / (expansion - line);
            (
                viewport.w * 0.98,
                viewport.h * (0.006 + 0.85 * p * p),
                0.82 * (1.0 - ((p - 0.72) / 0.28).clamp(0.0, 1.0)),
            )
        };
        let x = (viewport.x + (viewport.w - width) / 2.0 + wobble * viewport.w * 0.003)
            .clamp(viewport.x, viewport.x + viewport.w - width);
        let y = viewport.y + (viewport.h - height) / 2.0;
        for i in (1..=4).rev() {
            let margin = viewport.h * 0.008 * i as f32;
            let left = (x - margin).max(viewport.x);
            let right = (x + width + margin).min(viewport.x + viewport.w);
            let top = (y - margin).max(viewport.y);
            let bottom = (y + height + margin).min(viewport.y + viewport.h);
            draw_rectangle(
                left,
                top,
                right - left,
                bottom - top,
                Color::new(1.0, 1.0, 1.0, bloom * opacity * 0.06),
            );
        }
        for band in 0..24 {
            let relative = (band as f32 + 0.5) / 24.0 * 2.0 - 1.0;
            let inset = width * relative * relative * 0.015;
            let band_y = y + height * band as f32 / 24.0;
            draw_rectangle(
                x + inset,
                band_y,
                width - inset * 2.0,
                height / 24.0 + 0.01,
                Color::new(1.0, 1.0, 1.0, opacity * (1.0 - 0.12 * relative.abs())),
            );
        }
    }

    pub fn render_shutdown(&self, viewport: Rect, fraction: f32, bloom: f32) {
        let (width, height, alpha) = if fraction < 0.55 {
            (
                viewport.w * 0.98,
                viewport.h * (1.0 - fraction / 0.55).max(0.006),
                0.7,
            )
        } else if fraction < 0.8 {
            (
                viewport.w * (1.0 - (fraction - 0.55) / 0.25).max(0.01),
                viewport.h * 0.006,
                1.0,
            )
        } else {
            (
                viewport.h * 0.006,
                viewport.h * 0.006,
                (1.0 - (fraction - 0.8) / 0.2).max(0.0),
            )
        };
        let x = viewport.x + (viewport.w - width) / 2.0;
        let y = viewport.y + (viewport.h - height) / 2.0;
        let margin = viewport.h * 0.02;
        let left = (x - margin).max(viewport.x);
        let right = (x + width + margin).min(viewport.x + viewport.w);
        let top = (y - margin).max(viewport.y);
        let bottom = (y + height + margin).min(viewport.y + viewport.h);
        draw_rectangle(
            left,
            top,
            right - left,
            bottom - top,
            Color::new(1.0, 1.0, 1.0, alpha * bloom * 0.2),
        );
        draw_rectangle(x, y, width, height, Color::new(1.0, 1.0, 1.0, alpha));
    }

    /// The curvature placeholder shades the edges without warping the display.
    pub fn draw_curvature(&self, viewport: Rect) {
        let shade = Color::new(0.0, 0.0, 0.0, 0.18);
        let width = viewport.w * 0.016;
        draw_rectangle(viewport.x, viewport.y, width, viewport.h, shade);
        draw_rectangle(
            viewport.x + viewport.w - width,
            viewport.y,
            width,
            viewport.h,
            shade,
        );
    }
}

fn scanline_strength(physical_spacing: f32, opacity: f32) -> f32 {
    opacity.clamp(0.0, 1.0) * (physical_spacing - 1.0).clamp(0.0, 1.0)
}

fn loading_sample_ago(line: u32, column: u32) -> u64 {
    u64::from((LOADING_LINES - line) * LINE_T_STATES - column * LINE_T_STATES / LOADING_SAMPLES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanlines_fade_at_small_pixel_sizes_and_respect_opacity() {
        assert_eq!(scanline_strength(1.0, 1.0), 0.0);
        assert_eq!(scanline_strength(1.5, 1.0), 0.5);
        assert_eq!(scanline_strength(2.0, 0.4), 0.4);
        assert_eq!(scanline_strength(4.0, 0.0), 0.0);
    }

    #[test]
    fn loading_lines_follow_the_tape_signal_across_the_screen() {
        let mut bytes = vec![0; 0x80];
        bytes[11..13].copy_from_slice(&0x4089u16.to_le_bytes());
        let mut tape = Tape::from_p(&bytes).unwrap();
        tape.toggle().unwrap();
        tape.advance(crate::emulator::CPU_HZ * 5 + 50_000);
        let sample = |tape: &Tape| {
            (0..LOADING_LINES)
                .flat_map(|line| {
                    (0..LOADING_SAMPLES)
                        .map(move |column| tape.level_ago(loading_sample_ago(line, column)))
                })
                .collect::<Vec<_>>()
        };
        let frame = sample(&tape);
        assert!(frame.contains(&true) && frame.contains(&false));
        assert!(frame
            .chunks(LOADING_SAMPLES as usize)
            .any(|line| { line.contains(&true) && line.contains(&false) }));
        assert_eq!(frame, sample(&tape));
        tape.advance(1_000);
        assert_ne!(frame, sample(&tape));
    }

    #[test]
    fn warmup_fades_text_scanlines_and_reflections_only_after_cold_phase() {
        let settings = CrtWarmupSettings::default();
        let cold = visuals(ScreenState::Booting, 0.4, &settings, 0.0);
        assert_eq!(
            (cold.brightness, cold.scanlines, cold.reflections),
            (0.0, 0.0, 0.0)
        );
        let warming = visuals(ScreenState::Booting, 2.0, &settings, 0.0);
        assert!((0.0..1.0).contains(&warming.brightness));
        assert!((0.0..1.0).contains(&warming.scanlines));
        assert!((0.0..1.0).contains(&warming.reflections));
        let ready = visuals(ScreenState::Ready, 0.0, &settings, 0.0);
        assert_eq!(
            (ready.brightness, ready.scanlines, ready.reflections),
            (1.0, 1.0, 1.0)
        );
    }
}
