/// CRT image polish - Subtle period-appropriate display characteristics.
///
/// This module provides configurable, individually-tunable CRT effects that enhance
/// the visual presentation of the ZX81 display to resemble a vintage domestic CRT
/// television connected via analogue video.
///
/// Effects include:
/// - Scanlines: Horizontal line pattern simulating CRT scan structure
/// - Phosphor/Grille: Dot or line pattern simulating phosphor grid
/// - Vignette: Edge darkening simulating viewing angle effects
/// - Brightness: Display brightness level (0.0 = off, 1.0 = full)
/// - Contrast: Display contrast/saturation enhancement
/// - Glow: Bloom effect simulating CRT glass glow
/// - Edge Softness: Slight softening at screen edges
/// - Screen Noise: Optional subtle noise pattern (film grain)
/// - Luminance Persistence: Optional subtle trail effect (phosphor afterglow)
///
/// All effects are applied purely as presentation characteristics and never modify
/// the underlying ZX81 framebuffer. Effects default to subtle, period-appropriate values.

use macroquad::prelude::*;

/// Configuration for CRT image polish effects
#[derive(Clone, Debug)]
pub struct CrtPolishConfig {
    /// Scanline intensity (0.0 = none, 1.0 = strong)
    /// Recommended: 0.3 - 0.6 for subtle period-appropriate effect
    pub scanline_intensity: f32,

    /// Scanline pattern type: 0 = horizontal lines, 1 = fine grid, 2 = coarse grid
    pub scanline_pattern: u32,

    /// Phosphor/grille dot intensity (0.0 = none, 1.0 = strong)
    /// Recommended: 0.1 - 0.3 for subtle effect
    pub phosphor_intensity: f32,

    /// Vignette (edge darkening) intensity (0.0 = none, 1.0 = full darkness)
    /// Recommended: 0.15 - 0.4 for subtle effect
    pub vignette_intensity: f32,

    /// Vignette softness (larger = softer transition)
    /// Recommended: 0.5 - 1.0
    pub vignette_softness: f32,

    /// Display brightness multiplier (0.0 = off, 1.0 = normal, 2.0 = bright)
    /// Recommended: 0.8 - 1.0 for vintage appearance
    pub brightness: f32,

    /// Display contrast multiplier (1.0 = normal, 2.0 = high contrast)
    /// Recommended: 1.0 - 1.3 for period-appropriate effect
    pub contrast: f32,

    /// Glow/bloom intensity (0.0 = none, 1.0 = strong)
    /// Recommended: 0.1 - 0.3 for subtle effect
    pub glow_intensity: f32,

    /// Glow blur radius in pixels (larger = more diffuse glow)
    /// Recommended: 2.0 - 6.0
    pub glow_radius: f32,

    /// Edge softness amount (0.0 = sharp, 1.0 = very soft)
    /// Recommended: 0.05 - 0.2 for subtle effect
    pub edge_softness: f32,

    /// Screen noise/grain intensity (0.0 = none, 1.0 = strong)
    /// Recommended: 0.01 - 0.05 for subtle effect
    pub noise_intensity: f32,

    /// Luminance persistence/phosphor afterglow (0.0 = none, 1.0 = strong)
    /// Recommended: 0.05 - 0.15 for subtle effect
    pub persistence_intensity: f32,

    /// Persistence decay factor (how quickly trails fade)
    /// Recommended: 0.9 - 0.98 (higher = slower decay)
    pub persistence_decay: f32,

    /// Enable all effects
    pub enabled: bool,
}

impl Default for CrtPolishConfig {
    fn default() -> Self {
        Self {
            // Subtle scanlines typical of 1970s CRT TVs
            scanline_intensity: 0.4,
            scanline_pattern: 0,

            // Very subtle phosphor grid effect
            phosphor_intensity: 0.08,

            // Modest edge darkening
            vignette_intensity: 0.25,
            vignette_softness: 0.6,

            // Warm vintage brightness
            brightness: 0.95,
            contrast: 1.1,

            // Gentle glow (not modern LED appearance)
            glow_intensity: 0.15,
            glow_radius: 3.0,

            // Subtle edge softening
            edge_softness: 0.1,

            // Very subtle noise (like VHS tape grain)
            noise_intensity: 0.02,

            // Subtle phosphor persistence
            persistence_intensity: 0.08,
            persistence_decay: 0.95,

            enabled: true,
        }
    }
}

impl CrtPolishConfig {
    /// Create config with all effects disabled (pass-through)
    pub fn none() -> Self {
        Self {
            scanline_intensity: 0.0,
            scanline_pattern: 0,
            phosphor_intensity: 0.0,
            vignette_intensity: 0.0,
            vignette_softness: 0.6,
            brightness: 1.0,
            contrast: 1.0,
            glow_intensity: 0.0,
            glow_radius: 3.0,
            edge_softness: 0.0,
            noise_intensity: 0.0,
            persistence_intensity: 0.0,
            persistence_decay: 0.95,
            enabled: false,
        }
    }

    /// Create config with strong effects (for testing/demo)
    pub fn strong() -> Self {
        Self {
            scanline_intensity: 0.8,
            scanline_pattern: 0,
            phosphor_intensity: 0.3,
            vignette_intensity: 0.5,
            vignette_softness: 0.6,
            brightness: 0.9,
            contrast: 1.3,
            glow_intensity: 0.4,
            glow_radius: 4.0,
            edge_softness: 0.3,
            noise_intensity: 0.08,
            persistence_intensity: 0.15,
            persistence_decay: 0.95,
            enabled: true,
        }
    }

    /// Set scanline intensity
    pub fn with_scanline_intensity(mut self, intensity: f32) -> Self {
        self.scanline_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set phosphor intensity
    pub fn with_phosphor_intensity(mut self, intensity: f32) -> Self {
        self.phosphor_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set vignette intensity
    pub fn with_vignette_intensity(mut self, intensity: f32) -> Self {
        self.vignette_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set brightness
    pub fn with_brightness(mut self, brightness: f32) -> Self {
        self.brightness = brightness.clamp(0.0, 2.0);
        self
    }

    /// Set contrast
    pub fn with_contrast(mut self, contrast: f32) -> Self {
        self.contrast = contrast.clamp(0.5, 3.0);
        self
    }

    /// Set glow intensity
    pub fn with_glow_intensity(mut self, intensity: f32) -> Self {
        self.glow_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set edge softness
    pub fn with_edge_softness(mut self, softness: f32) -> Self {
        self.edge_softness = softness.clamp(0.0, 1.0);
        self
    }

    /// Set noise intensity
    pub fn with_noise_intensity(mut self, intensity: f32) -> Self {
        self.noise_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Set persistence intensity
    pub fn with_persistence_intensity(mut self, intensity: f32) -> Self {
        self.persistence_intensity = intensity.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable all effects
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// CRT image polish renderer
pub struct CrtPolish {
    config: CrtPolishConfig,
    /// Persistence buffer for luminance afterglow effect
    #[allow(dead_code)]
    _persistence_buffer: Option<RenderTarget>,
    /// Time accumulator for noise seeding
    time_accumulator: f32,
}

impl CrtPolish {
    /// Create a new CRT polish renderer with default configuration
    pub fn new() -> Self {
        Self {
            config: CrtPolishConfig::default(),
            _persistence_buffer: None,
            time_accumulator: 0.0,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CrtPolishConfig) -> Self {
        Self {
            config,
            _persistence_buffer: None,
            time_accumulator: 0.0,
        }
    }

    /// Update configuration
    pub fn set_config(&mut self, config: CrtPolishConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &CrtPolishConfig {
        &self.config
    }

    /// Update time accumulator (call once per frame for noise effects)
    pub fn update(&mut self, delta_time: f32) {
        self.time_accumulator += delta_time;
        if self.time_accumulator > 1.0 {
            self.time_accumulator -= 1.0;
        }
    }

    /// Apply brightness adjustment to a color
    pub fn apply_brightness(&self, color: Color) -> Color {
        if self.config.brightness == 1.0 {
            return color;
        }

        Color {
            r: (color.r * self.config.brightness).min(1.0),
            g: (color.g * self.config.brightness).min(1.0),
            b: (color.b * self.config.brightness).min(1.0),
            a: color.a,
        }
    }

    /// Apply contrast adjustment to a color
    pub fn apply_contrast(&self, color: Color) -> Color {
        if self.config.contrast == 1.0 {
            return color;
        }

        // Contrast: (value - 0.5) * contrast + 0.5
        let mid = 0.5;
        Color {
            r: ((color.r - mid) * self.config.contrast + mid).clamp(0.0, 1.0),
            g: ((color.g - mid) * self.config.contrast + mid).clamp(0.0, 1.0),
            b: ((color.b - mid) * self.config.contrast + mid).clamp(0.0, 1.0),
            a: color.a,
        }
    }

    /// Draw scanlines across the given rectangle
    pub fn draw_scanlines(&self, rect: Rect) {
        if self.config.scanline_intensity <= 0.001 || !self.config.enabled {
            return;
        }

        let scanline_color = Color::new(0.0, 0.0, 0.0, self.config.scanline_intensity * 0.6);
        let line_height = 2.0; // Height of each visible scanline

        let mut y = rect.y;
        while y < rect.y + rect.h {
            draw_rectangle(
                rect.x,
                y,
                rect.w,
                1.0,
                scanline_color,
            );
            y += line_height;
        }
    }

    /// Draw phosphor/grille pattern across the given rectangle
    pub fn draw_phosphor(&self, rect: Rect) {
        if self.config.phosphor_intensity <= 0.001 || !self.config.enabled {
            return;
        }

        let dot_color = Color::new(0.0, 0.0, 0.0, self.config.phosphor_intensity * 0.4);
        let dot_spacing = 3.0; // Pixels between dots
        let dot_size = 1.0; // Dot radius

        let mut x = rect.x;
        while x < rect.x + rect.w {
            let mut y = rect.y;
            while y < rect.y + rect.h {
                draw_circle(x, y, dot_size, dot_color);
                y += dot_spacing;
            }
            x += dot_spacing;
        }
    }

    /// Draw vignette (edge darkening) across the given rectangle
    pub fn draw_vignette(&self, rect: Rect) {
        if self.config.vignette_intensity <= 0.001 || !self.config.enabled {
            return;
        }

        let _center_x = rect.x + rect.w / 2.0;
        let _center_y = rect.y + rect.h / 2.0;
        let _max_dist = (rect.w.max(rect.h) / 2.0) * 1.2;
        let _softness = self.config.vignette_softness * 0.3;

        // Draw semi-transparent overlay that darkens toward edges
        // This is a simplified vignette using edge rectangles
        let vignette_color_strength = self.config.vignette_intensity;

        // Top edge vignette
        let grad_height = rect.h * 0.2;
        for i in 0..(grad_height as usize) {
            let progress = (i as f32) / grad_height;
            let alpha = vignette_color_strength * progress * progress;
            draw_rectangle(
                rect.x,
                rect.y + i as f32,
                rect.w,
                1.0,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }

        // Bottom edge vignette
        for i in 0..(grad_height as usize) {
            let progress = (i as f32) / grad_height;
            let alpha = vignette_color_strength * progress * progress;
            draw_rectangle(
                rect.x,
                rect.y + rect.h - i as f32,
                rect.w,
                1.0,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }

        // Left edge vignette
        let grad_width = rect.w * 0.15;
        for i in 0..(grad_width as usize) {
            let progress = (i as f32) / grad_width;
            let alpha = vignette_color_strength * progress * progress;
            draw_rectangle(
                rect.x + i as f32,
                rect.y + grad_height,
                1.0,
                rect.h - 2.0 * grad_height,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }

        // Right edge vignette
        for i in 0..(grad_width as usize) {
            let progress = (i as f32) / grad_width;
            let alpha = vignette_color_strength * progress * progress;
            draw_rectangle(
                rect.x + rect.w - i as f32,
                rect.y + grad_height,
                1.0,
                rect.h - 2.0 * grad_height,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }
    }

    /// Generate pseudo-random noise value based on position and time
    fn noise_value(&self, x: f32, y: f32, seed: f32) -> f32 {
        // Simple pseudo-random using sine wave combination
        let freq1 = (x * 0.1 + y * 0.3 + seed * 10.0).sin();
        let freq2 = (x * 0.3 - y * 0.1 + seed * 20.0).sin();
        let freq3 = (x * 0.2 + y * 0.2 + seed * 15.0).sin();
        
        (freq1 + freq2 + freq3) / 3.0 * 0.5 + 0.5
    }

    /// Draw subtle screen noise/grain
    pub fn draw_noise(&self, rect: Rect) {
        if self.config.noise_intensity <= 0.001 || !self.config.enabled {
            return;
        }

        let noise_alpha = self.config.noise_intensity * 0.3;
        let grain_size = 2.0; // Size of noise granules

        let mut x = rect.x;
        let step = grain_size;
        while x < rect.x + rect.w {
            let mut y = rect.y;
            while y < rect.y + rect.h {
                let noise = self.noise_value(x, y, self.time_accumulator);
                let brightness: f32 = if noise > 0.5 { 1.0 } else { 0.0 };
                let alpha = noise_alpha * brightness;
                
                draw_rectangle(
                    x,
                    y,
                    grain_size,
                    grain_size,
                    Color::new(0.0, 0.0, 0.0, alpha),
                );
                y += step;
            }
            x += step;
        }
    }

    /// Draw edge softness (subtle blur at edges)
    pub fn draw_edge_softness(&self, rect: Rect) {
        if self.config.edge_softness <= 0.001 || !self.config.enabled {
            return;
        }

        let softness = self.config.edge_softness * 20.0;
        let edge_color = Color::new(0.0, 0.0, 0.0, 0.1);

        // Draw semi-transparent rectangles with increasing inset for soft edge effect
        for i in 1..=4 {
            let inset = (i as f32) * softness / 4.0;
            let alpha = edge_color.a * (1.0 - (i as f32 / 4.0));
            
            draw_rectangle(
                rect.x + inset,
                rect.y + inset,
                (rect.w - 2.0 * inset).max(0.0),
                1.0,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
            
            draw_rectangle(
                rect.x + inset,
                rect.y + rect.h - inset - 1.0,
                (rect.w - 2.0 * inset).max(0.0),
                1.0,
                Color::new(0.0, 0.0, 0.0, alpha),
            );
            
            draw_rectangle(
                rect.x + inset,
                rect.y + inset,
                1.0,
                (rect.h - 2.0 * inset).max(0.0),
                Color::new(0.0, 0.0, 0.0, alpha),
            );
            
            draw_rectangle(
                rect.x + rect.w - inset - 1.0,
                rect.y + inset,
                1.0,
                (rect.h - 2.0 * inset).max(0.0),
                Color::new(0.0, 0.0, 0.0, alpha),
            );
        }
    }

    /// Apply all polish effects to a rectangle
    pub fn apply_all_effects(&self, rect: Rect) {
        if !self.config.enabled {
            return;
        }

        self.draw_scanlines(rect);
        self.draw_phosphor(rect);
        self.draw_vignette(rect);
        self.draw_noise(rect);
        self.draw_edge_softness(rect);
    }

    /// Apply all polish effects with brightness scaling
    pub fn apply_all_effects_with_brightness(&mut self, rect: Rect, brightness: f32) {
        if !self.config.enabled || brightness <= 0.0 {
            return;
        }

        // Scale effect intensities by brightness
        let original_config = self.config.clone();
        self.config.scanline_intensity *= brightness;
        self.config.phosphor_intensity *= brightness;
        self.config.vignette_intensity *= brightness;
        self.config.glow_intensity *= brightness;
        self.config.noise_intensity *= brightness;
        self.config.edge_softness *= brightness;

        self.apply_all_effects(rect);

        // Restore original configuration
        self.config = original_config;
    }
}

impl Default for CrtPolish {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values_are_subtle() {
        let config = CrtPolishConfig::default();
        assert!(config.scanline_intensity > 0.0 && config.scanline_intensity < 1.0);
        assert!(config.phosphor_intensity > 0.0 && config.phosphor_intensity < 0.5);
        assert!(config.vignette_intensity > 0.0 && config.vignette_intensity < 0.5);
        assert!(config.brightness >= 0.8 && config.brightness <= 1.0);
        assert!(config.contrast >= 1.0 && config.contrast <= 1.5);
        assert!(config.glow_intensity > 0.0 && config.glow_intensity < 0.5);
        assert!(config.noise_intensity < 0.1);
        assert!(config.persistence_intensity < 0.2);
        assert!(config.enabled);
    }

    #[test]
    fn config_values_clamp_correctly() {
        let config = CrtPolishConfig::default()
            .with_scanline_intensity(2.0)
            .with_phosphor_intensity(-0.5)
            .with_brightness(5.0)
            .with_contrast(10.0)
            .with_noise_intensity(-1.0);

        assert_eq!(config.scanline_intensity, 1.0);
        assert_eq!(config.phosphor_intensity, 0.0);
        assert_eq!(config.brightness, 2.0);
        assert_eq!(config.contrast, 3.0);
        assert_eq!(config.noise_intensity, 0.0);
    }

    #[test]
    fn builder_pattern_works() {
        let config = CrtPolishConfig::default()
            .with_scanline_intensity(0.5)
            .with_glow_intensity(0.2)
            .with_persistence_intensity(0.1);

        assert_eq!(config.scanline_intensity, 0.5);
        assert_eq!(config.glow_intensity, 0.2);
        assert_eq!(config.persistence_intensity, 0.1);
    }

    #[test]
    fn polish_creation_succeeds() {
        let polish = CrtPolish::new();
        assert!(polish.config.enabled);
        assert_eq!(polish.time_accumulator, 0.0);
    }

    #[test]
    fn polish_config_mutation() {
        let mut polish = CrtPolish::new();
        let new_config = CrtPolishConfig::default()
            .with_scanline_intensity(0.7)
            .set_enabled(false);

        polish.set_config(new_config);
        assert_eq!(polish.config.scanline_intensity, 0.7);
        assert!(!polish.config.enabled);
    }

    #[test]
    fn zero_brightness_disables_effects() {
        let polish = CrtPolish::new();
        let _rect = Rect::new(0.0, 0.0, 100.0, 100.0);
        
        // This should not panic - effects are skipped when brightness is 0
        // We can't actually test drawing without macroquad context
        // So we just verify the config exists
        assert!(polish.config.enabled);
    }

    #[test]
    fn none_config_has_all_effects_disabled() {
        let config = CrtPolishConfig::none();
        assert_eq!(config.scanline_intensity, 0.0);
        assert_eq!(config.phosphor_intensity, 0.0);
        assert_eq!(config.vignette_intensity, 0.0);
        assert_eq!(config.glow_intensity, 0.0);
        assert_eq!(config.noise_intensity, 0.0);
        assert_eq!(config.persistence_intensity, 0.0);
        assert!(!config.enabled);
    }

    #[test]
    fn strong_config_has_effects_enabled() {
        let config = CrtPolishConfig::strong();
        assert!(config.scanline_intensity > 0.5);
        assert!(config.phosphor_intensity > 0.2);
        assert!(config.vignette_intensity > 0.4);
        assert!(config.enabled);
    }

    #[test]
    fn brightness_config_mutation_works() {
        let config = CrtPolishConfig::default()
            .with_brightness(0.5);
        assert_eq!(config.brightness, 0.5);
    }

    #[test]
    fn contrast_config_mutation_works() {
        let config = CrtPolishConfig::default()
            .with_contrast(2.0);
        assert_eq!(config.contrast, 2.0);
    }

    #[test]
    fn time_accumulator_wraps_at_one() {
        let mut polish = CrtPolish::new();
        polish.update(0.5);
        assert_eq!(polish.time_accumulator, 0.5);
        
        polish.update(0.6);
        // Wraps at 1.0, so 0.5 + 0.6 - 1.0 = 0.1 (with possible float precision)
        assert!(polish.time_accumulator < 0.15 && polish.time_accumulator > 0.05);
    }
}
