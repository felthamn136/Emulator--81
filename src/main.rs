mod about;
mod app;
mod assets;
mod autoload;
mod bezel;
mod config;
mod crt;
mod debug;
pub mod emulator;
pub mod experience;
pub mod memory;
mod menu;
mod scaling;
mod tape;
mod tape_audio;
mod ui;
pub mod video;
pub mod viewport;
mod virtual_keyboard;

use config::window_conf;

#[macroquad::main(window_conf)]
async fn main() {
    config::center_window();
    if virtual_keyboard::is_keyboard_process() {
        virtual_keyboard::run().await;
    } else {
        app::run().await;
    }
}
