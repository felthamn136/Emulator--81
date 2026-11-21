use macroquad::prelude::*;

use crate::{app::AppState, bezel::TV_CANVAS, emulator::EmulatorCore, scaling::Scaling};

pub fn draw_debug_overlay(
    viewport: Rect,
    state: &AppState,
    scaling: &Scaling,
    emulator: &EmulatorCore,
    paused: bool,
) {
    let dpi = screen_dpi_scale();
    let tv = scaling.rect(TV_CANVAS);
    let mut lines = vec![
        format!("FPS: {}", get_fps()),
        format!(
            "Window: {:.0} x {:.0} logical",
            screen_width(),
            screen_height()
        ),
        format!(
            "Framebuffer: {:.0} x {:.0} px",
            screen_width() * dpi,
            screen_height() * dpi
        ),
        format!(
            "Viewport: {:.0} x {:.0} px",
            viewport.w * dpi,
            viewport.h * dpi
        ),
        format!("Fullscreen requested: {}", state.fullscreen),
        format!("Scanlines: {}", state.scanlines_enabled),
        format!("Reflections: {}", state.crt_reflections_enabled),
        format!("Curvature: {}", state.crt_curvature_enabled),
        format!("TV position: {:.1}, {:.1}", tv.x, tv.y),
        format!("TV scale: {:.4}", tv.w / 4096.0),
        format!(
            "Viewport rect: {:.1}, {:.1}, {:.1}, {:.1}",
            viewport.x, viewport.y, viewport.w, viewport.h
        ),
    ];
    if let Some(cpu) = emulator.cpu_snapshot() {
        lines.extend([
            format!(
                "CPU: {}  Cycles: {}",
                if paused { "PAUSED" } else { "RUNNING" },
                cpu.cycles
            ),
            format!(
                "PC {:04X}  SP {:04X}  Opcode {:02X}",
                cpu.pc, cpu.sp, cpu.opcode
            ),
            format!(
                "AF {:04X}  BC {:04X}  DE {:04X}  HL {:04X}",
                cpu.af, cpu.bc, cpu.de, cpu.hl
            ),
            format!("IX {:04X}  IY {:04X}", cpu.ix, cpu.iy),
            "F8 PAUSE/RUN  F9 STEP  F10 RESET".to_string(),
        ]);
    }
    let size = (screen_height() * 0.024).clamp(14.0, 24.0);
    let line_height = size * 1.25;
    draw_rectangle(
        8.0,
        crate::menu::BAR_HEIGHT + 8.0,
        size * 30.0,
        line_height * lines.len() as f32 + 14.0,
        Color::new(0.0, 0.0, 0.0, 0.8),
    );
    for (i, text) in lines.iter().enumerate() {
        draw_text(
            text,
            16.0,
            crate::menu::BAR_HEIGHT + 12.0 + line_height * (i + 1) as f32,
            size,
            WHITE,
        );
    }
}
