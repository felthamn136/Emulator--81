/// CRT calibration and debug mode for parameter tuning and testing.
///
/// This module provides an interactive calibration interface for tuning CRT effects
/// and verifying that no ZX81 pixels are hidden by the CRT mask. When enabled,
/// displays adjustable controls for all CRT parameters and visual overlays showing:
/// - Original framebuffer boundary
/// - Safe-area boundary
/// - CRT aperture boundary
/// - Center point marker
/// - Corner and edge position markers
/// - Cursor position test points

use macroquad::prelude::*;

/// Configuration for CRT calibration debug mode
#[derive(Clone, Debug)]
pub struct CrtCalibrationConfig {
    /// Enable calibration mode
    pub enabled: bool,
    /// Show framebuffer boundary overlay
    pub show_framebuffer_bounds: bool,
    /// Show safe-area boundary overlay
    pub show_safe_area_bounds: bool,
    /// Show CRT aperture boundary overlay
    pub show_aperture_bounds: bool,
    /// Show center point marker
    pub show_center_marker: bool,
    /// Show corner markers
    pub show_corner_markers: bool,
    /// Show edge position markers
    pub show_edge_markers: bool,
    /// Show cursor test positions
    pub show_cursor_positions: bool,
    /// Overlay opacity (0.0 to 1.0)
    pub overlay_opacity: f32,
}

impl Default for CrtCalibrationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            show_framebuffer_bounds: true,
            show_safe_area_bounds: true,
            show_aperture_bounds: true,
            show_center_marker: true,
            show_corner_markers: true,
            show_edge_markers: true,
            show_cursor_positions: true,
            overlay_opacity: 0.6,
        }
    }
}

impl CrtCalibrationConfig {
    /// Create a new config with all overlays disabled
    pub fn none() -> Self {
        Self {
            enabled: false,
            show_framebuffer_bounds: false,
            show_safe_area_bounds: false,
            show_aperture_bounds: false,
            show_center_marker: false,
            show_corner_markers: false,
            show_edge_markers: false,
            show_cursor_positions: false,
            overlay_opacity: 0.6,
        }
    }

    /// Create a new config with all overlays enabled
    pub fn full() -> Self {
        Self {
            enabled: true,
            show_framebuffer_bounds: true,
            show_safe_area_bounds: true,
            show_aperture_bounds: true,
            show_center_marker: true,
            show_corner_markers: true,
            show_edge_markers: true,
            show_cursor_positions: true,
            overlay_opacity: 0.6,
        }
    }

    /// Set overlay opacity
    pub fn with_opacity(mut self, opacity: f32) -> Self {
        self.overlay_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Toggle framebuffer bounds display
    pub fn toggle_framebuffer_bounds(mut self) -> Self {
        self.show_framebuffer_bounds = !self.show_framebuffer_bounds;
        self
    }

    /// Toggle safe-area bounds display
    pub fn toggle_safe_area_bounds(mut self) -> Self {
        self.show_safe_area_bounds = !self.show_safe_area_bounds;
        self
    }

    /// Toggle aperture bounds display
    pub fn toggle_aperture_bounds(mut self) -> Self {
        self.show_aperture_bounds = !self.show_aperture_bounds;
        self
    }

    /// Toggle center marker display
    pub fn toggle_center_marker(mut self) -> Self {
        self.show_center_marker = !self.show_center_marker;
        self
    }

    /// Toggle corner markers display
    pub fn toggle_corner_markers(mut self) -> Self {
        self.show_corner_markers = !self.show_corner_markers;
        self
    }

    /// Toggle edge markers display
    pub fn toggle_edge_markers(mut self) -> Self {
        self.show_edge_markers = !self.show_edge_markers;
        self
    }

    /// Toggle cursor position display
    pub fn toggle_cursor_positions(mut self) -> Self {
        self.show_cursor_positions = !self.show_cursor_positions;
        self
    }
}

/// CRT calibration debug renderer
pub struct CrtCalibration {
    config: CrtCalibrationConfig,
    /// Current selected parameter index (for UI navigation)
    selected_parameter: usize,
    /// Parameter adjustment value (-1.0 to 1.0)
    parameter_adjustment: f32,
}

impl CrtCalibration {
    /// Create a new calibration instance with default config
    pub fn new() -> Self {
        Self {
            config: CrtCalibrationConfig::default(),
            selected_parameter: 0,
            parameter_adjustment: 0.0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CrtCalibrationConfig) -> Self {
        Self {
            config,
            selected_parameter: 0,
            parameter_adjustment: 0.0,
        }
    }

    /// Get current configuration
    pub fn config(&self) -> &CrtCalibrationConfig {
        &self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: CrtCalibrationConfig) {
        self.config = config;
    }

    /// Enable calibration mode
    pub fn enable(&mut self) {
        self.config.enabled = true;
    }

    /// Disable calibration mode
    pub fn disable(&mut self) {
        self.config.enabled = false;
    }

    /// Toggle calibration mode on/off
    pub fn toggle(&mut self) {
        self.config.enabled = !self.config.enabled;
    }

    /// Get current enabled state
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Draw framebuffer boundary (original ZX81 framebuffer area)
    pub fn draw_framebuffer_bounds(&self, aperture: Rect) {
        if !self.config.show_framebuffer_bounds || !self.config.enabled {
            return;
        }

        let color = Color::new(1.0, 0.0, 0.0, self.config.overlay_opacity);
        // Draw rectangle outline
        draw_rectangle_lines(aperture.x, aperture.y, aperture.w, aperture.h, 2.0, color);
        // Label
        draw_text_ex(
            "FRAMEBUFFER",
            aperture.x + 4.0,
            aperture.y + 16.0,
            TextParams {
                font_size: 14,
                font_scale: 1.0,
                color,
                ..Default::default()
            },
        );
    }

    /// Draw safe-area boundary (area inside which content is guaranteed visible)
    pub fn draw_safe_area_bounds(&self, aperture: Rect, safe_area_x: f32, safe_area_y: f32) {
        if !self.config.show_safe_area_bounds || !self.config.enabled {
            return;
        }

        let inset_x = aperture.w * safe_area_x;
        let inset_y = aperture.h * safe_area_y;
        let safe_rect = Rect::new(
            aperture.x + inset_x,
            aperture.y + inset_y,
            aperture.w - 2.0 * inset_x,
            aperture.h - 2.0 * inset_y,
        );

        let color = Color::new(0.0, 1.0, 1.0, self.config.overlay_opacity);
        draw_rectangle_lines(safe_rect.x, safe_rect.y, safe_rect.w, safe_rect.h, 2.0, color);
        // Label
        draw_text_ex(
            "SAFE AREA",
            safe_rect.x + 4.0,
            safe_rect.y + 16.0,
            TextParams {
                font_size: 14,
                font_scale: 1.0,
                color,
                ..Default::default()
            },
        );
    }

    /// Draw CRT aperture boundary (area after barrel distortion and rounding)
    pub fn draw_aperture_bounds(&self, aperture: Rect) {
        if !self.config.show_aperture_bounds || !self.config.enabled {
            return;
        }

        let color = Color::new(0.0, 1.0, 0.0, self.config.overlay_opacity);
        // Draw aperture outline with dashed pattern
        self.draw_dashed_rect(aperture, 2.0, color, 10.0, 5.0);
        // Label
        draw_text_ex(
            "APERTURE",
            aperture.x + 4.0,
            aperture.y + 32.0,
            TextParams {
                font_size: 14,
                font_scale: 1.0,
                color,
                ..Default::default()
            },
        );
    }

    /// Draw center point marker
    pub fn draw_center_marker(&self, aperture: Rect) {
        if !self.config.show_center_marker || !self.config.enabled {
            return;
        }

        let center_x = aperture.x + aperture.w / 2.0;
        let center_y = aperture.y + aperture.h / 2.0;
        let color = Color::new(1.0, 1.0, 0.0, self.config.overlay_opacity);

        // Draw crosshair
        let size = 20.0;
        draw_line(center_x - size, center_y, center_x + size, center_y, 2.0, color);
        draw_line(center_x, center_y - size, center_x, center_y + size, 2.0, color);
        draw_circle_lines(center_x, center_y, 5.0, 2.0, color);
        // Label
        draw_text_ex(
            "CENTER",
            center_x + 10.0,
            center_y - 8.0,
            TextParams {
                font_size: 12,
                font_scale: 1.0,
                color,
                ..Default::default()
            },
        );
    }

    /// Draw corner position markers to verify content visibility
    pub fn draw_corner_markers(&self, aperture: Rect) {
        if !self.config.show_corner_markers || !self.config.enabled {
            return;
        }

        let color = Color::new(1.0, 0.5, 0.0, self.config.overlay_opacity);
        let marker_size = 15.0;
        let corners = [
            (aperture.x + marker_size, aperture.y + marker_size, "TL"),
            (
                aperture.x + aperture.w - marker_size,
                aperture.y + marker_size,
                "TR",
            ),
            (
                aperture.x + marker_size,
                aperture.y + aperture.h - marker_size,
                "BL",
            ),
            (
                aperture.x + aperture.w - marker_size,
                aperture.y + aperture.h - marker_size,
                "BR",
            ),
        ];

        for (x, y, label) in &corners {
            draw_circle_lines(*x, *y, 8.0, 2.0, color);
            draw_line(*x - 10.0, *y, *x + 10.0, *y, 2.0, color);
            draw_line(*x, *y - 10.0, *x, *y + 10.0, 2.0, color);
            draw_text_ex(
                label,
                x + 12.0,
                y - 4.0,
                TextParams {
                    font_size: 12,
                    font_scale: 1.0,
                    color,
                    ..Default::default()
                },
            );
        }
    }

    /// Draw edge position markers along all four edges
    pub fn draw_edge_markers(&self, aperture: Rect) {
        if !self.config.show_edge_markers || !self.config.enabled {
            return;
        }

        let color = Color::new(0.5, 0.5, 1.0, self.config.overlay_opacity);
        let marker_size = 10.0;
        let spacing = aperture.w / 5.0; // 5 markers per edge

        // Top edge
        for i in 1..5 {
            let x = aperture.x + spacing * (i as f32);
            draw_line(x, aperture.y, x, aperture.y + marker_size, 1.0, color);
            draw_circle_lines(x, aperture.y + marker_size / 2.0, 3.0, 1.0, color);
        }

        // Bottom edge
        for i in 1..5 {
            let x = aperture.x + spacing * (i as f32);
            draw_line(
                x,
                aperture.y + aperture.h,
                x,
                aperture.y + aperture.h - marker_size,
                1.0,
                color,
            );
            draw_circle_lines(
                x,
                aperture.y + aperture.h - marker_size / 2.0,
                3.0,
                1.0,
                color,
            );
        }

        // Left edge
        let spacing_v = aperture.h / 5.0;
        for i in 1..5 {
            let y = aperture.y + spacing_v * (i as f32);
            draw_line(aperture.x, y, aperture.x + marker_size, y, 1.0, color);
            draw_circle_lines(aperture.x + marker_size / 2.0, y, 3.0, 1.0, color);
        }

        // Right edge
        for i in 1..5 {
            let y = aperture.y + spacing_v * (i as f32);
            draw_line(
                aperture.x + aperture.w,
                y,
                aperture.x + aperture.w - marker_size,
                y,
                1.0,
                color,
            );
            draw_circle_lines(
                aperture.x + aperture.w - marker_size / 2.0,
                y,
                3.0,
                1.0,
                color,
            );
        }
    }

    /// Draw cursor test positions (four corners and center)
    pub fn draw_cursor_positions(&self, aperture: Rect) {
        if !self.config.show_cursor_positions || !self.config.enabled {
            return;
        }

        let color = Color::new(1.0, 1.0, 0.5, self.config.overlay_opacity);
        let cursor_size = 20.0;

        // Test positions: corners and center
        let positions = [
            (aperture.x + 20.0, aperture.y + 20.0, "CURSOR_TL"),
            (
                aperture.x + aperture.w - 20.0,
                aperture.y + 20.0,
                "CURSOR_TR",
            ),
            (
                aperture.x + 20.0,
                aperture.y + aperture.h - 20.0,
                "CURSOR_BL",
            ),
            (
                aperture.x + aperture.w - 20.0,
                aperture.y + aperture.h - 20.0,
                "CURSOR_BR",
            ),
            (
                aperture.x + aperture.w / 2.0,
                aperture.y + aperture.h / 2.0,
                "CURSOR_CENTER",
            ),
        ];

        for (x, y, label) in &positions {
            // Draw cursor-like box
            draw_rectangle_lines(*x - 5.0, *y - 5.0, 10.0, 10.0, 1.0, color);
            draw_line(*x - cursor_size, *y, *x + cursor_size, *y, 1.0, color);
            draw_line(*x, *y - cursor_size, *x, *y + cursor_size, 1.0, color);
            
            // Small label
            draw_text_ex(
                label,
                x + 8.0,
                y + 12.0,
                TextParams {
                    font_size: 10,
                    font_scale: 0.8,
                    color,
                    ..Default::default()
                },
            );
        }
    }

    /// Draw all calibration overlays
    pub fn draw_all_overlays(
        &self,
        aperture: Rect,
        safe_area_x: f32,
        safe_area_y: f32,
    ) {
        if !self.config.enabled {
            return;
        }

        self.draw_framebuffer_bounds(aperture);
        self.draw_safe_area_bounds(aperture, safe_area_x, safe_area_y);
        self.draw_aperture_bounds(aperture);
        self.draw_center_marker(aperture);
        self.draw_corner_markers(aperture);
        self.draw_edge_markers(aperture);
        self.draw_cursor_positions(aperture);
    }

    /// Draw calibration control panel showing tunable parameters
    pub fn draw_control_panel(
        &self,
        safe_area_x: f32,
        safe_area_y: f32,
        curvature_x: f32,
        curvature_y: f32,
        corner_radius: f32,
        scanline_strength: f32,
        vignette_strength: f32,
        glow_strength: f32,
        brightness: f32,
        contrast: f32,
    ) {
        if !self.config.enabled {
            return;
        }

        let panel_width = 300.0;
        let panel_height = 400.0;
        let panel_x = screen_width() - panel_width - 20.0;
        let panel_y = 60.0;

        // Draw panel background
        draw_rectangle(
            panel_x,
            panel_y,
            panel_width,
            panel_height,
            Color::new(0.1, 0.1, 0.1, 0.9),
        );
        draw_rectangle_lines(panel_x, panel_y, panel_width, panel_height, 2.0, WHITE);

        // Panel title
        draw_text_ex(
            "CRT CALIBRATION",
            panel_x + 10.0,
            panel_y + 20.0,
            TextParams {
                font_size: 16,
                font_scale: 1.0,
                color: YELLOW,
                ..Default::default()
            },
        );

        // Parameters list
        let params = vec![
            format!("Safe Area X: {:.3}", safe_area_x),
            format!("Safe Area Y: {:.3}", safe_area_y),
            format!("Curvature X: {:.3}", curvature_x),
            format!("Curvature Y: {:.3}", curvature_y),
            format!("Corner Radius: {:.3}", corner_radius),
            format!("Scanline Strength: {:.3}", scanline_strength),
            format!("Vignette Strength: {:.3}", vignette_strength),
            format!("Glow Strength: {:.3}", glow_strength),
            format!("Brightness: {:.3}", brightness),
            format!("Contrast: {:.3}", contrast),
        ];

        let line_height = 30.0;
        for (i, param) in params.iter().enumerate() {
            let y = panel_y + 40.0 + (i as f32) * line_height;
            let is_selected = i == self.selected_parameter;
            
            if is_selected {
                draw_rectangle(
                    panel_x + 2.0,
                    y - 12.0,
                    panel_width - 4.0,
                    line_height - 2.0,
                    Color::new(0.3, 0.3, 0.5, 0.5),
                );
            }

            draw_text_ex(
                param,
                panel_x + 10.0,
                y,
                TextParams {
                    font_size: 12,
                    font_scale: 1.0,
                    color: if is_selected { YELLOW } else { WHITE },
                    ..Default::default()
                },
            );
        }

        // Instructions
        draw_text_ex(
            "UP/DOWN: Select",
            panel_x + 10.0,
            panel_y + panel_height - 50.0,
            TextParams {
                font_size: 10,
                font_scale: 0.8,
                color: GRAY,
                ..Default::default()
            },
        );
        draw_text_ex(
            "LEFT/RIGHT: Adjust",
            panel_x + 10.0,
            panel_y + panel_height - 35.0,
            TextParams {
                font_size: 10,
                font_scale: 0.8,
                color: GRAY,
                ..Default::default()
            },
        );
        draw_text_ex(
            "R: Reset Defaults",
            panel_x + 10.0,
            panel_y + panel_height - 20.0,
            TextParams {
                font_size: 10,
                font_scale: 0.8,
                color: GRAY,
                ..Default::default()
            },
        );
    }

    /// Draw a dashed rectangle
    fn draw_dashed_rect(&self, rect: Rect, thickness: f32, color: Color, dash_len: f32, gap_len: f32) {
        let total = dash_len + gap_len;
        
        // Top edge
        let mut x = rect.x;
        while x < rect.x + rect.w {
            let next_x = (x + dash_len).min(rect.x + rect.w);
            draw_line(x, rect.y, next_x, rect.y, thickness, color);
            x += total;
        }
        
        // Bottom edge
        let mut x = rect.x;
        while x < rect.x + rect.w {
            let next_x = (x + dash_len).min(rect.x + rect.w);
            draw_line(x, rect.y + rect.h, next_x, rect.y + rect.h, thickness, color);
            x += total;
        }
        
        // Left edge
        let mut y = rect.y;
        while y < rect.y + rect.h {
            let next_y = (y + dash_len).min(rect.y + rect.h);
            draw_line(rect.x, y, rect.x, next_y, thickness, color);
            y += total;
        }
        
        // Right edge
        let mut y = rect.y;
        while y < rect.y + rect.h {
            let next_y = (y + dash_len).min(rect.y + rect.h);
            draw_line(rect.x + rect.w, y, rect.x + rect.w, next_y, thickness, color);
            y += total;
        }
    }

    /// Select next parameter
    pub fn next_parameter(&mut self, num_parameters: usize) {
        self.selected_parameter = (self.selected_parameter + 1) % num_parameters;
    }

    /// Select previous parameter
    pub fn prev_parameter(&mut self, num_parameters: usize) {
        if self.selected_parameter == 0 {
            self.selected_parameter = num_parameters - 1;
        } else {
            self.selected_parameter -= 1;
        }
    }

    /// Get selected parameter index
    pub fn selected_parameter(&self) -> usize {
        self.selected_parameter
    }

    /// Adjust current parameter
    pub fn adjust_parameter(&mut self, delta: f32) {
        self.parameter_adjustment += delta;
        self.parameter_adjustment = self.parameter_adjustment.clamp(-1.0, 1.0);
    }

    /// Get parameter adjustment value
    pub fn parameter_adjustment(&self) -> f32 {
        self.parameter_adjustment
    }

    /// Reset parameter adjustment
    pub fn reset_adjustment(&mut self) {
        self.parameter_adjustment = 0.0;
    }
}

impl Default for CrtCalibration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_overlays_disabled() {
        let config = CrtCalibrationConfig::default();
        assert!(!config.enabled);
        assert!(config.show_framebuffer_bounds);
        assert!(config.show_safe_area_bounds);
        assert!(config.show_aperture_bounds);
    }

    #[test]
    fn none_config_has_all_disabled() {
        let config = CrtCalibrationConfig::none();
        assert!(!config.enabled);
        assert!(!config.show_framebuffer_bounds);
        assert!(!config.show_safe_area_bounds);
        assert!(!config.show_aperture_bounds);
        assert!(!config.show_center_marker);
        assert!(!config.show_corner_markers);
        assert!(!config.show_edge_markers);
        assert!(!config.show_cursor_positions);
    }

    #[test]
    fn full_config_has_all_enabled() {
        let config = CrtCalibrationConfig::full();
        assert!(config.enabled);
        assert!(config.show_framebuffer_bounds);
        assert!(config.show_safe_area_bounds);
        assert!(config.show_aperture_bounds);
        assert!(config.show_center_marker);
        assert!(config.show_corner_markers);
        assert!(config.show_edge_markers);
        assert!(config.show_cursor_positions);
    }

    #[test]
    fn opacity_clamps_correctly() {
        let config = CrtCalibrationConfig::default()
            .with_opacity(2.0);
        assert_eq!(config.overlay_opacity, 1.0);
        
        let config = CrtCalibrationConfig::default()
            .with_opacity(-0.5);
        assert_eq!(config.overlay_opacity, 0.0);
    }

    #[test]
    fn toggle_methods_work() {
        let config = CrtCalibrationConfig::none()
            .toggle_framebuffer_bounds()
            .toggle_safe_area_bounds()
            .toggle_corner_markers();
        
        assert!(config.show_framebuffer_bounds);
        assert!(config.show_safe_area_bounds);
        assert!(config.show_corner_markers);
        assert!(!config.show_aperture_bounds);
    }

    #[test]
    fn calibration_creation_succeeds() {
        let calib = CrtCalibration::new();
        assert!(!calib.config.enabled);
        assert_eq!(calib.selected_parameter, 0);
        assert_eq!(calib.parameter_adjustment, 0.0);
    }

    #[test]
    fn calibration_enable_disable_works() {
        let mut calib = CrtCalibration::new();
        assert!(!calib.is_enabled());
        
        calib.enable();
        assert!(calib.is_enabled());
        
        calib.disable();
        assert!(!calib.is_enabled());
        
        calib.toggle();
        assert!(calib.is_enabled());
    }

    #[test]
    fn parameter_navigation_wraps() {
        let mut calib = CrtCalibration::new();
        assert_eq!(calib.selected_parameter(), 0);
        
        calib.next_parameter(10);
        assert_eq!(calib.selected_parameter(), 1);
        
        calib.next_parameter(10);
        calib.next_parameter(10);
        assert_eq!(calib.selected_parameter(), 3);
        
        // Wrap around
        for _ in 0..7 {
            calib.next_parameter(10);
        }
        assert_eq!(calib.selected_parameter(), 0); // Wrapped
    }

    #[test]
    fn parameter_adjustment_clamps() {
        let mut calib = CrtCalibration::new();
        
        calib.adjust_parameter(2.0);
        assert_eq!(calib.parameter_adjustment(), 1.0); // Clamped to max
        
        calib.reset_adjustment();
        assert_eq!(calib.parameter_adjustment(), 0.0);
        
        calib.adjust_parameter(-2.0);
        assert_eq!(calib.parameter_adjustment(), -1.0); // Clamped to min
    }

    #[test]
    fn prev_parameter_wraps_at_start() {
        let mut calib = CrtCalibration::new();
        assert_eq!(calib.selected_parameter(), 0);
        
        calib.prev_parameter(10);
        assert_eq!(calib.selected_parameter(), 9); // Wrapped to end
    }

    #[test]
    fn config_with_custom_opacity() {
        let config = CrtCalibrationConfig::default()
            .with_opacity(0.3);
        assert_eq!(config.overlay_opacity, 0.3);
    }
}
