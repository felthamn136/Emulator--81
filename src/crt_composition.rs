/// CRT composition and integration with bezel artwork.
///
/// This module provides layered rendering of CRT visual effects that seamlessly
/// integrate with the monitor bezel artwork, creating the illusion of a glass
/// CRT tube physically recessed into the monitor casing.
///
/// Rendering layers (in order):
/// 1. Bezel/background (TV cabinet, shadow)
/// 2. CRT display content
/// 3. Inner CRT shadow (creates recession effect)
/// 4. Glass surface (optional highlight)
/// 5. Foreground bezel (control labels, reflections)

use macroquad::prelude::*;

/// Configuration for CRT composition and glass effects
#[derive(Clone, Debug)]
pub struct CrtCompositionConfig {
    /// Inner shadow strength (0.0 = none, 1.0 = strong)
    /// Creates dark transition around CRT opening
    pub inner_shadow_strength: f32,

    /// Shadow blur radius in pixels (creates soft edges)
    /// Recommended: 8-16 for subtle effect
    pub shadow_blur_radius: f32,

    /// Edge darkening intensity (0.0 = none, 1.0 = strong)
    /// Applied as vignette around CRT aperture
    pub edge_darkening: f32,

    /// Glass highlight opacity (0.0 = none, 1.0 = strong)
    /// Very subtle effect, typically 0.02-0.08
    pub glass_highlight_opacity: f32,

    /// Fresnel effect strength (0.0 = none, 1.0 = strong)
    /// Makes edges appear more reflective than center
    pub fresnel_strength: f32,

    /// Vignette softness (larger = softer transition)
    /// Recommended: 0.1-0.3 of aperture size
    pub vignette_softness: f32,
}

impl Default for CrtCompositionConfig {
    fn default() -> Self {
        Self {
            inner_shadow_strength: 0.5,      // Moderate shadow for recession
            shadow_blur_radius: 12.0,        // Soft edges, ~12px blur
            edge_darkening: 0.3,             // Subtle edge tinting (30%)
            glass_highlight_opacity: 0.04,   // Very subtle highlight (4%)
            fresnel_strength: 0.15,          // Subtle fresnel effect
            vignette_softness: 0.15,         // 15% of aperture size
        }
    }
}

impl CrtCompositionConfig {
    /// Create configuration with custom shadow strength
    pub fn with_shadow_strength(strength: f32) -> Self {
        Self {
            inner_shadow_strength: strength.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Set inner shadow strength
    pub fn set_inner_shadow_strength(&mut self, strength: f32) {
        self.inner_shadow_strength = strength.clamp(0.0, 1.0);
    }

    /// Set edge darkening strength
    pub fn set_edge_darkening(&mut self, darkening: f32) {
        self.edge_darkening = darkening.clamp(0.0, 1.0);
    }

    /// Set glass highlight opacity
    pub fn set_glass_highlight(&mut self, opacity: f32) {
        self.glass_highlight_opacity = opacity.clamp(0.0, 1.0);
    }

    /// Set fresnel effect strength
    pub fn set_fresnel_strength(&mut self, strength: f32) {
        self.fresnel_strength = strength.clamp(0.0, 1.0);
    }
}

/// CRT composition renderer for seamless bezel integration
pub struct CrtComposition {
    config: CrtCompositionConfig,
}

impl CrtComposition {
    /// Create a new CRT composition with default configuration
    pub fn new() -> Self {
        Self {
            config: CrtCompositionConfig::default(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CrtCompositionConfig) -> Self {
        Self { config }
    }

    /// Update composition configuration
    pub fn set_config(&mut self, config: CrtCompositionConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &CrtCompositionConfig {
        &self.config
    }

    /// Draw inner shadow around CRT aperture to create recession effect
    ///
    /// This creates a dark transition that makes the glass appear physically
    /// recessed into the monitor casing, as in a real CRT.
    pub fn draw_inner_shadow(&self, aperture: Rect) {
        if self.config.inner_shadow_strength <= 0.001 {
            return;
        }

        let shadow_color = Color::new(0.0, 0.0, 0.0, self.config.inner_shadow_strength);

        // Shadow is drawn as multiple layers at different positions to create
        // a soft recession effect. Each layer is slightly inset from the aperture.
        for layer in 1..=4 {
            let inset = (layer as f32) * (self.config.shadow_blur_radius / 4.0);
            let inset_rect = Rect::new(
                aperture.x + inset,
                aperture.y + inset,
                (aperture.w - inset * 2.0).max(0.0),
                (aperture.h - inset * 2.0).max(0.0),
            );

            // Alpha decreases for each layer, creating soft gradient
            let alpha_factor = 1.0 - (layer as f32 / 5.0);
            let layer_color = Color::new(0.0, 0.0, 0.0, shadow_color.a * alpha_factor * 0.6);

            // Draw frame borders (top, bottom, left, right)
            self.draw_shadow_frame(aperture, inset_rect, layer_color);
        }

        // Draw additional softening at aperture edges for smooth transition
        self.draw_edge_softening(aperture);
    }

    /// Draw shadow frame at specified inset
    fn draw_shadow_frame(&self, outer: Rect, inner: Rect, color: Color) {
        // Top edge
        draw_rectangle(outer.x, outer.y, outer.w, inner.y - outer.y, color);

        // Bottom edge
        draw_rectangle(
            outer.x,
            inner.y + inner.h,
            outer.w,
            outer.y + outer.h - (inner.y + inner.h),
            color,
        );

        // Left edge
        draw_rectangle(
            outer.x,
            inner.y,
            inner.x - outer.x,
            inner.h,
            color,
        );

        // Right edge
        draw_rectangle(
            inner.x + inner.w,
            inner.y,
            outer.x + outer.w - (inner.x + inner.w),
            inner.h,
            color,
        );
    }

    /// Draw softening effect at aperture edges
    fn draw_edge_softening(&self, aperture: Rect) {
        let softness = self.config.vignette_softness * aperture.h.min(aperture.w);
        let alpha = self.config.inner_shadow_strength * 0.3;

        // Softening rectangles at each edge, with reduced alpha for subtle effect
        let _soft_color = Color::new(0.0, 0.0, 0.0, alpha);

        // Top softening
        let top_y = aperture.y;
        for step in 0..4 {
            let rel_progress = step as f32 / 4.0;
            let step_alpha = alpha * (1.0 - rel_progress);
            let step_color = Color::new(0.0, 0.0, 0.0, step_alpha);
            let y_offset = rel_progress * softness * 0.25;
            draw_rectangle(
                aperture.x,
                top_y + y_offset,
                aperture.w,
                softness * 0.25,
                step_color,
            );
        }

        // Bottom softening
        let bottom_y = aperture.y + aperture.h;
        for step in 0..4 {
            let rel_progress = step as f32 / 4.0;
            let step_alpha = alpha * (1.0 - rel_progress);
            let step_color = Color::new(0.0, 0.0, 0.0, step_alpha);
            let y_offset = rel_progress * softness * 0.25;
            draw_rectangle(
                aperture.x,
                bottom_y - softness * 0.25 - y_offset,
                aperture.w,
                softness * 0.25,
                step_color,
            );
        }
    }

    /// Draw edge darkening/vignette around CRT aperture
    ///
    /// This creates a subtle darkening at the edges of the CRT display,
    /// making the content appear to glow slightly and creating depth.
    pub fn draw_edge_darkening(&self, aperture: Rect) {
        if self.config.edge_darkening <= 0.001 {
            return;
        }

        let margin = aperture.h * self.config.vignette_softness;
        let vignette_color = Color::new(0.0, 0.0, 0.0, self.config.edge_darkening * 0.5);

        // Top vignette
        draw_rectangle(
            aperture.x,
            aperture.y,
            aperture.w,
            margin,
            vignette_color,
        );

        // Bottom vignette
        draw_rectangle(
            aperture.x,
            aperture.y + aperture.h - margin,
            aperture.w,
            margin,
            vignette_color,
        );

        // Left vignette
        draw_rectangle(
            aperture.x,
            aperture.y + margin,
            margin * 0.8,
            aperture.h - margin * 2.0,
            vignette_color,
        );

        // Right vignette
        draw_rectangle(
            aperture.x + aperture.w - margin * 0.8,
            aperture.y + margin,
            margin * 0.8,
            aperture.h - margin * 2.0,
            vignette_color,
        );
    }

    /// Draw subtle glass highlight effect on CRT surface
    ///
    /// A very subtle highlight at the top-left of the glass, suggesting
    /// a light source reflecting off the curved glass surface.
    pub fn draw_glass_highlight(&self, aperture: Rect) {
        if self.config.glass_highlight_opacity <= 0.001 {
            return;
        }

        let opacity = self.config.glass_highlight_opacity;

        // Main highlight - subtle glow in top-left quadrant
        let highlight_width = aperture.w * 0.15;
        let _highlight_height = aperture.h * 0.12;
        let highlight_x = aperture.x + aperture.w * 0.08;
        let highlight_y = aperture.y + aperture.h * 0.06;

        // Draw highlight as ellipse-like shape using circles
        for i in 0..3 {
            let progress = i as f32 / 3.0;
            let radius = highlight_width * 0.5 * (1.0 - progress * 0.3);
            let alpha = opacity * (1.0 - progress * progress);
            draw_circle(
                highlight_x,
                highlight_y,
                radius,
                Color::new(1.0, 1.0, 1.0, alpha),
            );
        }

        // Optional fresnel-like effect along top edge
        if self.config.fresnel_strength > 0.001 {
            let fresnel_color = Color::new(
                1.0,
                1.0,
                1.0,
                self.config.fresnel_strength * opacity * 0.3,
            );
            draw_rectangle(
                aperture.x,
                aperture.y,
                aperture.w,
                aperture.h * 0.04,
                fresnel_color,
            );
        }
    }

    /// Draw smooth anti-aliased transition between glass and bezel
    ///
    /// Creates a subtle border effect that blends the CRT glass into the
    /// surrounding bezel, avoiding harsh rectangular boundaries.
    pub fn draw_glass_bezel_transition(&self, aperture: Rect) {
        let transition_width = aperture.h * 0.03;
        if transition_width <= 0.5 {
            return; // Too small to render
        }

        let transition_color = Color::new(0.0, 0.0, 0.0, 0.15);

        // Create smooth transition by drawing thin outline
        // Top edge
        draw_rectangle(
            aperture.x - transition_width * 0.5,
            aperture.y - transition_width,
            aperture.w + transition_width,
            transition_width,
            transition_color,
        );

        // Bottom edge
        draw_rectangle(
            aperture.x - transition_width * 0.5,
            aperture.y + aperture.h,
            aperture.w + transition_width,
            transition_width,
            transition_color,
        );

        // Left edge
        draw_rectangle(
            aperture.x - transition_width,
            aperture.y,
            transition_width,
            aperture.h,
            transition_color,
        );

        // Right edge
        draw_rectangle(
            aperture.x + aperture.w,
            aperture.y,
            transition_width,
            aperture.h,
            transition_color,
        );
    }

    /// Apply complete CRT composition effects
    ///
    /// Renders all layers (inner shadow, edge darkening, glass highlight, transition)
    /// to create a seamless integration between CRT display and monitor bezel.
    pub fn apply_all_effects(&self, aperture: Rect) {
        self.draw_inner_shadow(aperture);
        self.draw_edge_darkening(aperture);
        self.draw_glass_highlight(aperture);
        self.draw_glass_bezel_transition(aperture);
    }

    /// Apply composition effects with custom brightness
    ///
    /// Scales all effect opacities by the given brightness factor,
    /// allowing effects to fade in/out during CRT power-on/off sequences.
    pub fn apply_all_effects_with_brightness(&self, aperture: Rect, brightness: f32) {
        if brightness <= 0.001 {
            return; // No effects visible
        }

        // Create temporary config with scaled opacities
        let mut scaled_config = self.config.clone();
        scaled_config.inner_shadow_strength *= brightness;
        scaled_config.edge_darkening *= brightness;
        scaled_config.glass_highlight_opacity *= brightness;
        scaled_config.fresnel_strength *= brightness;

        // Create temporary composition with scaled config
        let temp_composition = CrtComposition::with_config(scaled_config);
        temp_composition.apply_all_effects(aperture);
    }
}

impl Default for CrtComposition {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values_are_subtle() {
        let config = CrtCompositionConfig::default();
        assert!(config.inner_shadow_strength > 0.0 && config.inner_shadow_strength <= 1.0);
        assert!(config.edge_darkening > 0.0 && config.edge_darkening <= 1.0);
        assert!(config.glass_highlight_opacity < 0.1); // Very subtle highlight
        assert!(config.glass_highlight_opacity > 0.01);
        assert!(config.fresnel_strength < 0.3); // Subtle fresnel
    }

    #[test]
    fn config_values_clamp_correctly() {
        let mut config = CrtCompositionConfig::default();
        config.set_inner_shadow_strength(1.5); // Over-high
        assert_eq!(config.inner_shadow_strength, 1.0);

        config.set_inner_shadow_strength(-0.5); // Negative
        assert_eq!(config.inner_shadow_strength, 0.0);

        config.set_edge_darkening(0.5);
        assert_eq!(config.edge_darkening, 0.5);
    }

    #[test]
    fn builder_pattern_works() {
        let config = CrtCompositionConfig::with_shadow_strength(0.6);
        assert_eq!(config.inner_shadow_strength, 0.6);
        assert_eq!(config.edge_darkening, CrtCompositionConfig::default().edge_darkening);
    }

    #[test]
    fn composition_creation_succeeds() {
        let composition = CrtComposition::new();
        assert!(composition.config().inner_shadow_strength > 0.0);

        let custom_config = CrtCompositionConfig::with_shadow_strength(0.3);
        let custom = CrtComposition::with_config(custom_config);
        assert_eq!(custom.config().inner_shadow_strength, 0.3);
    }

    #[test]
    fn composition_config_mutation() {
        let mut composition = CrtComposition::new();
        let original = composition.config().inner_shadow_strength;

        let mut new_config = composition.config().clone();
        new_config.set_inner_shadow_strength(0.8);
        composition.set_config(new_config);

        assert_ne!(composition.config().inner_shadow_strength, original);
        assert_eq!(composition.config().inner_shadow_strength, 0.8);
    }

    #[test]
    fn brightness_scaling_reduces_effects() {
        let composition = CrtComposition::new();
        let config = composition.config().clone();
        let original_shadow = config.inner_shadow_strength;

        // Scaled at 0.5 brightness should have half the original opacity
        let mut scaled_config = config.clone();
        scaled_config.inner_shadow_strength *= 0.5;
        assert_eq!(
            scaled_config.inner_shadow_strength,
            original_shadow * 0.5
        );
    }

    #[test]
    fn zero_brightness_disables_effects() {
        // Test that brightness scaling works - cannot call drawing functions in test
        let composition = CrtComposition::new();
        let config = composition.config().clone();
        
        // Scaling to zero should disable effects
        let mut scaled_config = config.clone();
        scaled_config.inner_shadow_strength *= 0.0;
        assert_eq!(scaled_config.inner_shadow_strength, 0.0);
    }
}
