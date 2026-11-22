#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use zx81_emulator::{app, config, config::window_conf, virtual_keyboard};

#[macroquad::main(window_conf)]
async fn main() {
    config::center_window();
    if virtual_keyboard::is_keyboard_process() {
        virtual_keyboard::run().await;
    } else {
        app::run().await;
    }
}
