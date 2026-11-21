use macroquad::prelude::*;

const WIDTH: f32 = 780.0;
const PADDING: f32 = 28.0;
const FONT_SIZE: u16 = 48;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Regular,
    Bold,
    Italic,
}

use Style::{Bold, Italic, Regular};

const PARAGRAPHS: &[&[(&str, Style)]] = &[
    &[
        ("Emulator-81", Bold),
        (" is a modern Sinclair ZX81 emulator built with ", Regular),
        ("Rust", Bold),
        (" and ", Regular),
        ("Macroquad", Bold),
        (".", Regular),
    ],
    &[("Designed primarily for Linux while remaining portable across platforms, Emulator-81 features a scalable CRT-style display, integrated software library, and support for loading and saving ZX81 .P program files.", Regular)],
    &[("Inspired by one of the most innovative home computers ever created, this project aims to preserve and explore the unique architecture of the Sinclair ZX81 while providing a practical way to run original software on modern systems.", Regular)],
    &[
        ("Emulator-81 was conceived, designed, and developed by ", Regular),
        ("Neil Feltham", Bold),
        (". The software has been created using ", Regular),
        ("Microsoft Copilot AI", Bold),
        (", guided through comprehensive prompts, architectural planning, testing, refinement, and iterative development. The project explores how modern AI-assisted development can be applied to the creation of complex emulation software while maintaining a clear technical vision and design direction.", Regular),
    ],
    &[("The project remains an active work in progress, with ongoing improvements focused on emulation accuracy, performance, authenticity, and the overall ZX81 experience.", Regular)],
];

struct Fragment {
    text: String,
    style: Style,
    width: f32,
}

struct Line {
    fragments: Vec<Fragment>,
    width: f32,
    baseline: f32,
    size: f32,
    centered: bool,
}

struct Layout {
    lines: Vec<Line>,
    divider_y: f32,
    height: f32,
}

struct Fonts {
    regular: Font,
    bold: Font,
    italic: Font,
}

impl Fonts {
    fn get(&self, style: Style) -> &Font {
        match style {
            Regular => &self.regular,
            Bold => &self.bold,
            Italic => &self.italic,
        }
    }

    fn measure(&self, text: &str, style: Style, size: f32) -> f32 {
        measure_text(
            text,
            Some(self.get(style)),
            FONT_SIZE,
            size / FONT_SIZE as f32,
        )
        .width
    }
}

pub struct About {
    pub open: bool,
    fonts: Fonts,
    layout: Layout,
    offset: Vec2,
    drag: Option<Vec2>,
}

impl About {
    pub fn new() -> Self {
        fn font(bytes: &[u8]) -> Font {
            let mut font =
                load_ttf_font_from_bytes(bytes).expect("bundled About font must be valid");
            font.set_filter(FilterMode::Linear);
            font
        }
        let fonts = Fonts {
            regular: font(include_bytes!("../assets/fonts/Roboto-Regular.ttf")),
            bold: font(include_bytes!("../assets/fonts/Roboto-Bold.ttf")),
            italic: font(include_bytes!("../assets/fonts/Roboto-Italic.ttf")),
        };
        let layout = build_layout(|text, style, size| fonts.measure(text, style, size));
        Self {
            open: false,
            fonts,
            layout,
            offset: Vec2::ZERO,
            drag: None,
        }
    }

    fn panel(&self) -> Rect {
        Self::panel_rect(
            screen_width(),
            screen_height(),
            self.layout.height,
            self.offset,
        )
    }

    fn panel_rect(width: f32, height: f32, content_height: f32, offset: Vec2) -> Rect {
        let scale = ((width - 24.0).max(0.0) / WIDTH)
            .min((height - 24.0).max(0.0) / content_height)
            .min(1.0);
        let size = vec2(WIDTH, content_height) * scale;
        let position = vec2(width, height) / 2.0 - size / 2.0 + offset;
        Rect::new(
            position.x.clamp(0.0, (width - size.x).max(0.0)),
            position.y.clamp(0.0, (height - size.y).max(0.0)),
            size.x,
            size.y,
        )
    }

    pub fn update(&mut self) {
        if is_key_pressed(KeyCode::Escape) {
            self.open = false;
            self.drag = None;
            return;
        }
        let panel = self.panel();
        let scale = panel.w / WIDTH;
        let mouse: Vec2 = mouse_position().into();
        if is_mouse_button_pressed(MouseButton::Left) {
            let close = Rect::new(
                panel.x + panel.w - 50.0 * scale,
                panel.y,
                50.0 * scale,
                56.0 * scale,
            );
            if close.contains(mouse) {
                self.open = false;
                self.drag = None;
                return;
            }
            if Rect::new(panel.x, panel.y, panel.w, 56.0 * scale).contains(mouse) {
                self.drag = Some(mouse - panel.point());
            }
        }
        if let Some(drag) = self.drag {
            if is_mouse_button_down(MouseButton::Left) {
                let center = vec2(screen_width(), screen_height()) / 2.0;
                self.offset = mouse - drag - (center - vec2(panel.w, panel.h) / 2.0);
            } else {
                self.drag = None;
            }
        }
    }

    pub fn draw(&self, background: Option<&Texture2D>) {
        if !self.open {
            return;
        }
        let panel = self.panel();
        let scale = panel.w / WIDTH;
        if scale <= 0.0 {
            return;
        }
        draw_rectangle(
            0.0,
            0.0,
            screen_width(),
            screen_height(),
            Color::from_rgba(0, 0, 0, 175),
        );
        draw_rectangle(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            Color::from_rgba(14, 14, 16, 255),
        );
        if let Some(background) = background {
            draw_texture_ex(
                background,
                panel.x,
                panel.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(panel.w, panel.h)),
                    ..Default::default()
                },
            );
            draw_rectangle(
                panel.x,
                panel.y,
                panel.w,
                panel.h,
                Color::from_rgba(0, 0, 0, 100),
            );
        }
        draw_rectangle_lines(
            panel.x,
            panel.y,
            panel.w,
            panel.h,
            2.0 * scale,
            Color::from_rgba(154, 39, 39, 255),
        );
        draw_line(
            panel.x + PADDING * scale,
            panel.y + 56.0 * scale,
            panel.x + (WIDTH - PADDING) * scale,
            panel.y + 56.0 * scale,
            scale,
            Color::from_rgba(133, 34, 34, 255),
        );
        draw_line(
            panel.x + PADDING * scale,
            panel.y + self.layout.divider_y * scale,
            panel.x + (WIDTH - PADDING) * scale,
            panel.y + self.layout.divider_y * scale,
            scale,
            Color::from_rgba(133, 34, 34, 255),
        );
        for line in &self.layout.lines {
            let left = if line.centered {
                (WIDTH - line.width) / 2.0
            } else {
                PADDING
            };
            let mut x = panel.x + left * scale;
            for fragment in &line.fragments {
                draw_text_ex(
                    &fragment.text,
                    x,
                    panel.y + line.baseline * scale,
                    TextParams {
                        font: Some(self.fonts.get(fragment.style)),
                        font_size: FONT_SIZE,
                        font_scale: line.size * scale / FONT_SIZE as f32,
                        color: if fragment.style == Italic {
                            LIGHTGRAY
                        } else {
                            WHITE
                        },
                        ..Default::default()
                    },
                );
                x += fragment.width * scale;
            }
        }
        let cx = panel.x + (WIDTH - 27.0) * scale;
        let cy = panel.y + 28.0 * scale;
        draw_line(
            cx - 6.0 * scale,
            cy - 6.0 * scale,
            cx + 6.0 * scale,
            cy + 6.0 * scale,
            2.0 * scale,
            LIGHTGRAY,
        );
        draw_line(
            cx + 6.0 * scale,
            cy - 6.0 * scale,
            cx - 6.0 * scale,
            cy + 6.0 * scale,
            2.0 * scale,
            LIGHTGRAY,
        );
    }
}

fn build_layout(mut measure: impl FnMut(&str, Style, f32) -> f32) -> Layout {
    let mut lines = Vec::new();
    let mut y = 40.0;
    add_text(
        &mut lines,
        &[("About Emulator-81", Bold)],
        24.0,
        &mut y,
        false,
        &mut measure,
    );
    y = 82.0;
    for paragraph in PARAGRAPHS {
        add_text(&mut lines, paragraph, 16.0, &mut y, false, &mut measure);
        y += 9.0;
    }
    let divider_y = y - 3.0;
    y += 20.0;
    for footer in [
        &[("Emulator-81", Bold)][..],
        &[("Version 0.3.0 Alpha", Bold)][..],
        &[
            ("Designed and Developed by ", Regular),
            ("Neil Feltham", Bold),
        ][..],
        &[
            ("Built with ", Regular),
            ("Rust • Macroquad • Microsoft Copilot AI", Bold),
        ][..],
    ] {
        add_text(&mut lines, footer, 15.0, &mut y, true, &mut measure);
    }
    y += 5.0;
    add_text(&mut lines, &[("Preserving the Sinclair ZX81 for modern systems while exploring the future of AI-assisted software development.", Italic)], 14.0, &mut y, true, &mut measure);
    Layout {
        lines,
        divider_y,
        height: y + 14.0,
    }
}

fn add_text(
    lines: &mut Vec<Line>,
    segments: &[(&str, Style)],
    size: f32,
    y: &mut f32,
    centered: bool,
    measure: &mut impl FnMut(&str, Style, f32) -> f32,
) {
    let mut fragments = Vec::new();
    let mut width = 0.0;
    let mut pending_space = false;
    for &(text, style) in segments {
        for part in text.split_inclusive(char::is_whitespace) {
            let word = part.trim_end_matches(char::is_whitespace);
            if !word.is_empty() {
                let space = if pending_space && !fragments.is_empty() {
                    " "
                } else {
                    ""
                };
                let mut value = format!("{space}{word}");
                let mut word_width = measure(&value, style, size);
                if width + word_width > WIDTH - 2.0 * PADDING && !fragments.is_empty() {
                    lines.push(Line {
                        fragments: std::mem::take(&mut fragments),
                        width,
                        baseline: *y,
                        size,
                        centered,
                    });
                    *y += size * 1.28;
                    width = 0.0;
                    value = word.to_owned();
                    word_width = measure(&value, style, size);
                }
                width += word_width;
                fragments.push(Fragment {
                    text: value,
                    style,
                    width: word_width,
                });
            }
            pending_space = part.ends_with(char::is_whitespace);
        }
    }
    if !fragments.is_empty() {
        lines.push(Line {
            fragments,
            width,
            baseline: *y,
            size,
            centered,
        });
        *y += size * 1.28;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_wraps_text_and_preserves_inline_spacing_and_styles() {
        let layout = build_layout(|text, _, size| text.chars().count() as f32 * size * 0.5);
        assert!(layout
            .lines
            .iter()
            .all(|line| line.width <= WIDTH - 2.0 * PADDING));
        assert!(layout.lines.last().unwrap().baseline < layout.height);
        let actual: Vec<String> = layout
            .lines
            .iter()
            .flat_map(|line| {
                let text: String = line
                    .fragments
                    .iter()
                    .map(|fragment| fragment.text.as_str())
                    .collect();
                text.split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .collect();
        let mut expected = String::from("About Emulator-81 ");
        for paragraph in PARAGRAPHS {
            for (text, _) in *paragraph {
                expected.push_str(text);
            }
            expected.push(' ');
        }
        expected.push_str("Emulator-81 Version 0.3.0 Alpha Designed and Developed by Neil Feltham Built with Rust • Macroquad • Microsoft Copilot AI Preserving the Sinclair ZX81 for modern systems while exploring the future of AI-assisted software development.");
        assert_eq!(actual, expected.split_whitespace().collect::<Vec<_>>());
        assert!(layout.lines.iter().any(|line| line
            .fragments
            .iter()
            .any(|fragment| fragment.style == Italic)));
    }

    #[test]
    fn panel_fits_small_and_resized_windows_without_scrolling() {
        let layout = build_layout(|text, _, size| text.chars().count() as f32 * size * 0.5);
        for (width, height) in [
            (800.0, 600.0),
            (320.0, 240.0),
            (400.0, 900.0),
            (1920.0, 1080.0),
        ] {
            for offset in [Vec2::ZERO, vec2(-5000.0, 5000.0)] {
                let panel = About::panel_rect(width, height, layout.height, offset);
                assert!(panel.x >= 0.0 && panel.y >= 0.0);
                assert!(panel.x + panel.w <= width && panel.y + panel.h <= height);
                let scale = panel.w / WIDTH;
                assert!((panel.h - layout.height * scale).abs() < 0.001);
                assert!(
                    panel.y + layout.lines.last().unwrap().baseline * scale < panel.y + panel.h
                );
            }
        }
    }
}
