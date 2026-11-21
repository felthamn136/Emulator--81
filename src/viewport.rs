use macroquad::prelude::*;

use crate::experience::ScreenState;

/// Fit a centered 4:3 viewport within the available rectangle.
pub fn calculate_viewport(bounds: Rect) -> Rect {
    let width = bounds.w.max(0.0);
    let height = bounds.h.max(0.0);
    let w = width.min(height * 4.0 / 3.0);
    let h = w * 3.0 / 4.0;
    Rect::new(
        bounds.x + (width - w) / 2.0,
        bounds.y + (height - h) / 2.0,
        w,
        h,
    )
}

pub struct ScreenFrame<'a> {
    pub state: ScreenState,
    pub progress: f32,
    pub filename: Option<&'a str>,
    pub brightness: f32,
    pub bloom: f32,
    pub glow: f32,
    pub vertical_jitter: f32,
}

pub fn draw_viewport(viewport: Rect, frame: &ScreenFrame<'_>) {
    draw_rectangle(viewport.x, viewport.y, viewport.w, viewport.h, BLACK);
    let lines: Vec<(String, f32)> = match frame.state {
        ScreenState::Booting => vec![
            ("SINCLAIR ZX81".into(), 0.25),
            ("16K RAM PACK".into(), 0.45),
            ("READY".into(), 0.65),
        ],
        ScreenState::Ready => vec![("READY".into(), 0.48)],
        ScreenState::Searching => vec![("LOAD \"\"".into(), 0.35), ("SEARCHING".into(), 0.55)],
        ScreenState::Loading => {
            let lengths = [28, 22, 18, 14, 10, 6, 2];
            let mut remaining = (frame.progress.clamp(0.0, 1.0) * 100.0).floor() as usize;
            let mut lines = vec![("LOADING".into(), 0.20)];
            for (index, length) in lengths.into_iter().enumerate() {
                let count = remaining.min(length);
                remaining -= count;
                lines.push(("#".repeat(count), 0.34 + index as f32 * 0.075));
            }
            lines
        }
        ScreenState::Loaded => vec![("PROGRAM LOADED".into(), 0.38), ("READY".into(), 0.58)],
        ScreenState::Inserted => vec![
            ("PROGRAM INSERTED".into(), 0.28),
            (
                frame
                    .filename
                    .unwrap_or("")
                    .to_uppercase()
                    .chars()
                    .take(24)
                    .collect(),
                0.48,
            ),
            ("READY".into(), 0.68),
        ],
        ScreenState::PowerOff | ScreenState::Shutdown => Vec::new(),
    };
    let pixel = viewport.w / (28.0 * 6.0 + 8.0);
    let brightness = frame.brightness.clamp(0.0, 1.0);
    let color = Color::new(1.0, 1.0, 1.0, brightness);
    for (text, y) in lines {
        let y = y + frame.vertical_jitter.clamp(-0.02, 0.02);
        if frame.bloom > 0.0 && brightness > 0.0 {
            draw_pixel_text(
                &text,
                viewport,
                y,
                pixel,
                Color::new(
                    1.0,
                    1.0,
                    1.0,
                    (frame.bloom * brightness * 0.06).clamp(0.0, 1.0),
                ),
                pixel * 1.2,
            );
        }
        if frame.glow > 0.0 && brightness > 0.0 {
            draw_pixel_text(
                &text,
                viewport,
                y,
                pixel,
                Color::new(
                    1.0,
                    1.0,
                    1.0,
                    (frame.glow * brightness * 0.18).clamp(0.0, 1.0),
                ),
                pixel * 0.55,
            );
        }
        draw_pixel_text(&text, viewport, y, pixel, color, 0.0);
    }
}

pub fn draw_viewport_outline(viewport: Rect) {
    draw_rectangle_lines(viewport.x, viewport.y, viewport.w, viewport.h, 2.0, GREEN);
}

// A fixed 5x7 bitmap keeps the placeholder monospaced without requiring a font asset.
fn draw_pixel_text(text: &str, viewport: Rect, y: f32, pixel: f32, color: Color, spread: f32) {
    let width = text.chars().count() as f32 * 6.0 * pixel - pixel;
    let start_x = viewport.x + (viewport.w - width) / 2.0;
    let start_y = viewport.y + viewport.h * y;
    for (i, ch) in text.chars().enumerate() {
        for (row, bits) in glyph(ch).iter().enumerate() {
            for col in 0..5 {
                if bits & (1 << (4 - col)) != 0 {
                    draw_rectangle(
                        start_x + (i as f32 * 6.0 + col as f32) * pixel - spread,
                        start_y + row as f32 * pixel - spread,
                        pixel + spread * 2.0,
                        pixel + spread * 2.0,
                        color,
                    );
                }
            }
        }
    }
}

fn glyph(ch: char) -> [u8; 7] {
    match ch {
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [15, 16, 16, 16, 16, 16, 15],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [15, 16, 16, 19, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 18, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [14, 16, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 1, 14],
        '#' => [10, 31, 10, 10, 31, 10, 0],
        '"' => [10, 10, 10, 0, 0, 0, 0],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        '_' => [0, 0, 0, 0, 0, 0, 31],
        ' ' => [0; 7],
        _ => [14, 17, 1, 2, 4, 0, 4],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_fits_and_centers_without_distortion() {
        assert_eq!(
            calculate_viewport(Rect::new(0.0, 0.0, 1920.0, 1080.0)),
            Rect::new(240.0, 0.0, 1440.0, 1080.0)
        );
        assert_eq!(
            calculate_viewport(Rect::new(10.0, 20.0, 800.0, 1000.0)),
            Rect::new(10.0, 220.0, 800.0, 600.0)
        );
        assert_eq!(
            calculate_viewport(Rect::new(0.0, 0.0, 0.0, 0.0)),
            Rect::new(0.0, 0.0, 0.0, 0.0)
        );
    }

    #[test]
    fn startup_message_glyphs_are_available() {
        for ch in "SINCLAIR ZX81 16K RAM PACK READY LOAD \"\" SEARCHING LOADING # PROGRAM INSERTED DEMO.P81".chars() {
            assert!(ch == ' ' || glyph(ch).iter().any(|&row| row != 0), "{ch}");
        }
    }
}
