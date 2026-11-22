use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
};

use macroquad::prelude::*;

const ARGUMENT: &str = "--virtual-keyboard";
const IMAGE_WIDTH: f32 = 1469.0;
const IMAGE_HEIGHT: f32 = 768.0;
const IMAGE: &[u8] = include_bytes!("../assets/zx81-keyboard.png");

// Image rows are in display order; matrix positions follow the ZX81 port wiring.
const MATRIX: [[(usize, usize); 10]; 4] = [
    [
        (3, 0),
        (3, 1),
        (3, 2),
        (3, 3),
        (3, 4),
        (4, 4),
        (4, 3),
        (4, 2),
        (4, 1),
        (4, 0),
    ],
    [
        (2, 0),
        (2, 1),
        (2, 2),
        (2, 3),
        (2, 4),
        (5, 4),
        (5, 3),
        (5, 2),
        (5, 1),
        (5, 0),
    ],
    [
        (1, 0),
        (1, 1),
        (1, 2),
        (1, 3),
        (1, 4),
        (6, 4),
        (6, 3),
        (6, 2),
        (6, 1),
        (6, 0),
    ],
    [
        (0, 0),
        (0, 1),
        (0, 2),
        (0, 3),
        (0, 4),
        (7, 4),
        (7, 3),
        (7, 2),
        (7, 1),
        (7, 0),
    ],
];

pub fn is_keyboard_process() -> bool {
    std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == ARGUMENT)
}

pub fn window_conf() -> Conf {
    Conf {
        window_title: "ZX81 Virtual Keyboard".to_owned(),
        window_width: 768,
        window_height: 402,
        window_resizable: true,
        high_dpi: true,
        icon: Some(crate::config::application_icon()),
        platform: macroquad::miniquad::conf::Platform {
            swap_interval: Some(1),
            linux_backend: macroquad::miniquad::conf::LinuxBackend::WaylandWithX11Fallback,
            linux_wm_class: crate::config::application_window_class(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn image_rect(width: f32, height: f32) -> Rect {
    let scale = (width / IMAGE_WIDTH).min(height / IMAGE_HEIGHT).max(0.0);
    let w = IMAGE_WIDTH * scale;
    let h = IMAGE_HEIGHT * scale;
    Rect::new((width - w) / 2.0, (height - h) / 2.0, w, h)
}

fn key_rect(row: usize, column: usize) -> Rect {
    let x = [
        28.0, 171.0, 313.0, 459.0, 603.0, 744.0, 888.0, 1033.0, 1177.0, 1321.0,
    ][column];
    let y = [30.0, 212.0, 412.0, 603.0][row];
    let width = [
        125.0, 123.0, 124.0, 124.0, 121.0, 123.0, 124.0, 124.0, 123.0, 123.0,
    ][column];
    let height = [113.0, 113.0, 111.0, 113.0][row];
    Rect::new(x, y, width, height)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Key {
    Matrix(usize, usize),
    Function,
}

fn key_at(position: Vec2, bounds: Rect) -> Option<Key> {
    if bounds.w <= 0.0 || bounds.h <= 0.0 || !bounds.contains(position) {
        return None;
    }
    let point = vec2(
        (position.x - bounds.x) * IMAGE_WIDTH / bounds.w,
        (position.y - bounds.y) * IMAGE_HEIGHT / bounds.h,
    );
    for (row, keys) in MATRIX.iter().enumerate() {
        for (column, &key) in keys.iter().enumerate() {
            if key_rect(row, column).contains(point) {
                if key == (6, 0) && point.y < 451.0 {
                    return Some(Key::Function);
                }
                return Some(Key::Matrix(key.0, key.1));
            }
        }
    }
    None
}

fn key_bit((row, column): (usize, usize)) -> u64 {
    1 << (row * 5 + column)
}

#[derive(Default)]
struct KeyboardInput {
    shift: bool,
    held: Option<(usize, usize)>,
    function: bool,
}

impl KeyboardInput {
    fn press(&mut self, key: Option<Key>) {
        match key {
            Some(Key::Matrix(0, 0)) => self.shift = !self.shift,
            Some(Key::Matrix(row, column)) => self.held = Some((row, column)),
            Some(Key::Function) => {
                self.held = Some((6, 0));
                self.function = true;
            }
            None => {}
        }
    }

    fn release(&mut self) {
        if self.held.take().is_some() {
            self.shift = false;
            self.function = false;
        }
    }

    fn mask(&self) -> u64 {
        u64::from(self.shift || self.function) | self.held.map_or(0, key_bit)
    }
}

pub async fn run() {
    // Closing the parent's stdin pipe also closes an orphaned keyboard window.
    std::thread::spawn(|| {
        let mut byte = [0];
        if let Err(error) = std::io::stdin().read(&mut byte) {
            eprintln!("Virtual keyboard parent connection failed: {error}");
        }
        std::process::exit(0);
    });

    let image = match Image::from_file_with_format(IMAGE, None) {
        Ok(image) => image,
        Err(error) => {
            eprintln!("Cannot decode virtual keyboard image: {error}");
            std::process::exit(1);
        }
    };
    let texture = Texture2D::from_image(&image);
    texture.set_filter(FilterMode::Linear);
    let mut input = KeyboardInput::default();
    let mut last_mask = 0;
    loop {
        if is_key_pressed(KeyCode::Escape) {
            macroquad::miniquad::window::request_quit();
        }
        let bounds = image_rect(screen_width(), screen_height());
        if is_mouse_button_pressed(MouseButton::Left) {
            input.press(key_at(mouse_position().into(), bounds));
        }
        if !is_mouse_button_down(MouseButton::Left) {
            input.release();
        }
        let mask = input.mask();
        if mask != last_mask {
            let result =
                writeln!(std::io::stdout(), "{mask:010x}").and_then(|()| std::io::stdout().flush());
            if let Err(error) = result {
                eprintln!("Cannot send virtual keyboard input: {error}");
                std::process::exit(1);
            }
            last_mask = mask;
        }

        clear_background(BLACK);
        draw_texture_ex(
            &texture,
            bounds.x,
            bounds.y,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(bounds.w, bounds.h)),
                ..Default::default()
            },
        );
        for (row, keys) in MATRIX.iter().enumerate() {
            for (column, &key) in keys.iter().enumerate() {
                if mask & key_bit(key) != 0 {
                    let rect = key_rect(row, column);
                    draw_rectangle_lines(
                        bounds.x + rect.x * bounds.w / IMAGE_WIDTH,
                        bounds.y + rect.y * bounds.h / IMAGE_HEIGHT,
                        rect.w * bounds.w / IMAGE_WIDTH,
                        rect.h * bounds.h / IMAGE_HEIGHT,
                        2.0,
                        YELLOW,
                    );
                }
            }
        }
        next_frame().await;
    }
}

pub enum Event {
    State(u64),
    Closed,
    None,
}

pub struct KeyboardWindow {
    child: Child,
    events: Receiver<Result<u64, String>>,
}

impl KeyboardWindow {
    pub fn open() -> std::io::Result<Self> {
        let mut child = Command::new(std::env::current_exe()?)
            .arg(ARGUMENT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()?;
        let output = child.stdout.take().expect("piped keyboard stdout");
        let (sender, events) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                let result = line
                    .map_err(|error| error.to_string())
                    .and_then(|line| {
                        u64::from_str_radix(&line, 16).map_err(|error| error.to_string())
                    })
                    .and_then(|mask| {
                        if mask >> 40 == 0 {
                            Ok(mask)
                        } else {
                            Err("invalid keyboard matrix".to_owned())
                        }
                    });
                if sender.send(result).is_err() {
                    break;
                }
            }
        });
        Ok(Self { child, events })
    }

    pub fn poll(&mut self) -> Result<Event, String> {
        match self.events.try_recv() {
            Ok(Ok(mask)) => Ok(Event::State(mask)),
            Ok(Err(error)) => Err(format!("Virtual keyboard input failed: {error}")),
            Err(TryRecvError::Empty) => Ok(Event::None),
            Err(TryRecvError::Disconnected) => match self.child.try_wait() {
                Ok(Some(status)) if status.success() => Ok(Event::Closed),
                Ok(Some(status)) => Err(format!("Virtual keyboard exited unexpectedly: {status}")),
                Ok(None) => Ok(Event::None),
                Err(error) => Err(format!("Cannot check virtual keyboard: {error}")),
            },
        }
    }
}

impl Drop for KeyboardWindow {
    fn drop(&mut self) {
        match self.child.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) => {
                if let Err(error) = self.child.kill() {
                    eprintln!("Cannot close virtual keyboard: {error}");
                }
                if let Err(error) = self.child.wait() {
                    eprintln!("Cannot reap virtual keyboard: {error}");
                }
            }
            Err(error) => eprintln!("Cannot check virtual keyboard: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_keys_map_to_matrix_at_different_window_sizes() {
        let x = [
            89.0, 232.0, 374.0, 520.0, 664.0, 805.0, 949.0, 1094.0, 1238.0, 1382.0,
        ];
        let y = [86.0, 268.0, 467.0, 659.0];
        let expected = [
            [
                (3, 0),
                (3, 1),
                (3, 2),
                (3, 3),
                (3, 4),
                (4, 4),
                (4, 3),
                (4, 2),
                (4, 1),
                (4, 0),
            ],
            [
                (2, 0),
                (2, 1),
                (2, 2),
                (2, 3),
                (2, 4),
                (5, 4),
                (5, 3),
                (5, 2),
                (5, 1),
                (5, 0),
            ],
            [
                (1, 0),
                (1, 1),
                (1, 2),
                (1, 3),
                (1, 4),
                (6, 4),
                (6, 3),
                (6, 2),
                (6, 1),
                (6, 0),
            ],
            [
                (0, 0),
                (0, 1),
                (0, 2),
                (0, 3),
                (0, 4),
                (7, 4),
                (7, 3),
                (7, 2),
                (7, 1),
                (7, 0),
            ],
        ];
        for (width, height) in [(1469.0, 768.0), (1024.0, 700.0), (400.0, 400.0)] {
            let bounds = image_rect(width, height);
            for (row, keys) in expected.iter().enumerate() {
                for (column, &expected) in keys.iter().enumerate() {
                    let point = vec2(
                        bounds.x + x[column] * bounds.w / IMAGE_WIDTH,
                        bounds.y + y[row] * bounds.h / IMAGE_HEIGHT,
                    );
                    assert_eq!(
                        key_at(point, bounds),
                        Some(Key::Matrix(expected.0, expected.1))
                    );
                }
            }

            assert_eq!(key_at(vec2(bounds.x + 1.0, bounds.y + 1.0), bounds), None);
            let gap = vec2(
                bounds.x + 160.0 * bounds.w / IMAGE_WIDTH,
                bounds.y + 86.0 * bounds.h / IMAGE_HEIGHT,
            );
            assert_eq!(key_at(gap, bounds), None);
        }
    }

    #[test]
    fn shifted_period_sends_shift_and_period() {
        let bounds = image_rect(IMAGE_WIDTH, IMAGE_HEIGHT);
        let mut input = KeyboardInput::default();
        input.press(key_at(vec2(90.0, 660.0), bounds));
        input.release();
        input.press(key_at(vec2(1238.0, 660.0), bounds));
        assert_eq!(input.mask(), key_bit((0, 0)) | key_bit((7, 1)));
        input.release();
        assert_eq!(input.mask(), 0);
    }

    #[test]
    fn shift_latches_for_one_key_and_can_be_cancelled() {
        let mut input = KeyboardInput::default();
        input.press(Some(Key::Matrix(0, 0)));
        input.release();
        assert_eq!(input.mask(), key_bit((0, 0)));
        input.press(Some(Key::Matrix(3, 0)));
        assert_eq!(input.mask(), key_bit((0, 0)) | key_bit((3, 0)));
        input.release();
        assert_eq!(input.mask(), 0);
        input.press(Some(Key::Matrix(0, 0)));
        input.press(Some(Key::Matrix(0, 0)));
        assert_eq!(input.mask(), 0);
    }

    #[test]
    fn function_label_presses_shift_and_newline() {
        let bounds = image_rect(1469.0, 768.0);
        assert_eq!(key_at(vec2(1382.0, 435.0), bounds), Some(Key::Function));
        assert_eq!(key_at(vec2(1382.0, 480.0), bounds), Some(Key::Matrix(6, 0)));
        let mut input = KeyboardInput::default();
        input.press(Some(Key::Function));
        assert_eq!(input.mask(), key_bit((0, 0)) | key_bit((6, 0)));
        input.release();
        assert_eq!(input.mask(), 0);
        input.press(Some(Key::Matrix(6, 0)));
        assert_eq!(input.mask(), key_bit((6, 0)));
    }
}
