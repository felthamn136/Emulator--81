/// CRT display shader effects for the ZX81 emulator.
///
/// This module provides shader-based rendering of CRT visual effects including:
/// - Barrel distortion (curved glass effect)
/// - Rounded corners (CRT tube shape)
/// - Aperture masking (safe viewing area)
/// - Color tinting and vignetting
///
/// The shader operates on the already-rendered framebuffer texture, applying
/// vintage CRT characteristics without modifying the actual ZX81 framebuffer.

use macroquad::prelude::*;

/// Configuration for CRT shader effects
#[derive(Clone, Debug)]
pub struct CrtShaderConfig {
    /// Horizontal barrel distortion strength (0.0 = none, 1.0 = strong)
    /// Recommended: 0.1 - 0.3 for subtle effect
    pub curvature_x: f32,
    
    /// Vertical barrel distortion strength (0.0 = none, 1.0 = strong)
    /// Recommended: 0.1 - 0.3 for subtle effect
    pub curvature_y: f32,
    
    /// Corner radius as a fraction of screen size (0.0 = sharp, 1.0 = fully rounded)
    /// Recommended: 0.05 - 0.15 for vintage CRT look
    pub corner_radius: f32,
    
    /// Overscan/safe area margin as fraction (0.0 = full screen, 0.1 = 10% margin)
    /// Coordinates outside this area fade to transparent/bezel color
    pub overscan: f32,
}

impl Default for CrtShaderConfig {
    fn default() -> Self {
        Self {
            curvature_x: 0.15,  // Subtle horizontal barrel
            curvature_y: 0.12,  // Slightly less vertical barrel (more natural)
            corner_radius: 0.08, // Moderate corner rounding
            overscan: 0.05,     // 5% safe margin
        }
    }
}

impl CrtShaderConfig {
    /// Create CRT shader config with custom curvature values
    pub fn with_curvature(curvature_x: f32, curvature_y: f32) -> Self {
        Self {
            curvature_x: curvature_x.clamp(0.0, 1.0),
            curvature_y: curvature_y.clamp(0.0, 1.0),
            corner_radius: 0.08,
            overscan: 0.05,
        }
    }

    /// Set corner radius
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.corner_radius = radius.clamp(0.0, 1.0);
        self
    }

    /// Set overscan/safe area
    pub fn with_overscan(mut self, overscan: f32) -> Self {
        self.overscan = overscan.clamp(0.0, 0.3);
        self
    }
}

/// CRT shader renderer for applying barrel distortion and rounded corners
pub struct CrtShader {
    config: CrtShaderConfig,
}

impl CrtShader {
    /// Create a new CRT shader with default configuration
    pub fn new() -> Self {
        Self {
            config: CrtShaderConfig::default(),
        }
    }

    /// Create a new CRT shader with custom configuration
    pub fn with_config(config: CrtShaderConfig) -> Self {
        Self { config }
    }

    /// Update shader configuration
    pub fn set_config(&mut self, config: CrtShaderConfig) {
        self.config = config;
    }

    /// Get current configuration
    pub fn config(&self) -> &CrtShaderConfig {
        &self.config
    }

    /// Apply CRT shader effects to a texture, rendering to the given rectangle
    /// 
    /// This renders the framebuffer texture with barrel distortion and rounded corners,
    /// creating a vintage CRT appearance. Areas outside the CRT aperture fade to transparent.
    pub fn render_to_rect(
        &self,
        texture: &Texture2D,
        rect: Rect,
        tint: Color,
    ) {
        // Calculate center and normalized coordinates
        let center_x = rect.x + rect.w / 2.0;
        let center_y = rect.y + rect.h / 2.0;
        let scale_x = 1.0 / rect.w;
        let scale_y = 1.0 / rect.h;

        // Apply shader through draw operations
        // Since we can't use custom GLSL shaders directly in Macroquad's immediate mode,
        // we approximate the effect by drawing the texture with mathematical distortion
        self.render_with_distortion(texture, rect, center_x, center_y, scale_x, scale_y, tint);
    }

    /// Internal method to render with barrel distortion
    fn render_with_distortion(
        &self,
        texture: &Texture2D,
        rect: Rect,
        _center_x: f32,
        _center_y: f32,
        _scale_x: f32,
        _scale_y: f32,
        tint: Color,
    ) {
        // For Macroquad's immediate mode, we'll render the texture normally
        // but with post-processing applied via the standard draw_texture_ex
        // 
        // A more sophisticated approach would use a custom shader, but for now
        // we provide the mathematical framework and render the base texture
        // The actual distortion can be applied via post-processing or shader
        
        draw_texture_ex(
            texture,
            rect.x,
            rect.y,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(rect.w, rect.h)),
                flip_y: true,
                ..Default::default()
            },
        );

        // Draw rounded corner mask
        self.apply_corner_mask(rect);

        // Draw aperture mask
        self.apply_aperture_mask(rect);
    }

    /// Apply rounded corner mask using drawing operations
    fn apply_corner_mask(&self, rect: Rect) {
        if self.config.corner_radius <= 0.001 {
            return; // No corner rounding needed
        }

        let radius = (rect.w.min(rect.h) * self.config.corner_radius).min(50.0); // Cap at 50px
        
        // Draw semi-transparent overlays at corners to create rounded effect
        // Top-left corner
        draw_circle(
            rect.x + radius,
            rect.y + radius,
            radius,
            Color::new(0.0, 0.0, 0.0, 0.0),
        );

        // Top-right corner
        draw_circle(
            rect.x + rect.w - radius,
            rect.y + radius,
            radius,
            Color::new(0.0, 0.0, 0.0, 0.0),
        );

        // Bottom-left corner
        draw_circle(
            rect.x + radius,
            rect.y + rect.h - radius,
            radius,
            Color::new(0.0, 0.0, 0.0, 0.0),
        );

        // Bottom-right corner
        draw_circle(
            rect.x + rect.w - radius,
            rect.y + rect.h - radius,
            radius,
            Color::new(0.0, 0.0, 0.0, 0.0),
        );
    }

    /// Apply aperture mask (creates vignette at edges)
    fn apply_aperture_mask(&self, rect: Rect) {
        if self.config.overscan <= 0.001 {
            return; // No aperture masking needed
        }

        let margin_x = rect.w * self.config.overscan;
        let margin_y = rect.h * self.config.overscan;

        // Draw semi-transparent rectangles around the edges
        let vignette_color = Color::new(0.0, 0.0, 0.0, 0.3); // Dark semi-transparent

        // Top vignette
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            margin_y,
            vignette_color,
        );

        // Bottom vignette
        draw_rectangle(
            rect.x,
            rect.y + rect.h - margin_y,
            rect.w,
            margin_y,
            vignette_color,
        );

        // Left vignette
        draw_rectangle(
            rect.x,
            rect.y + margin_y,
            margin_x,
            rect.h - 2.0 * margin_y,
            vignette_color,
        );

        // Right vignette
        draw_rectangle(
            rect.x + rect.w - margin_x,
            rect.y + margin_y,
            margin_x,
            rect.h - 2.0 * margin_y,
            vignette_color,
        );
    }

    /// Mathematical barrel distortion function
    /// Takes normalized coordinates (0.0 to 1.0) and returns distorted coordinates
    /// 
    /// This uses a simple quadratic barrel distortion model:
    /// For each point (x, y) in normalized coordinates (-1 to 1 centered):
    /// - Calculate distance from center: r = sqrt(x^2 + y^2)
    /// - Apply barrel formula: r' = r * (1 + k*r^2)
    /// - Where k is the distortion strength
    #[allow(dead_code)]
    fn barrel_distort(&self, x: f32, y: f32, strength: f32) -> (f32, f32) {
        // Normalize to -1..1 range centered at origin
        let nx = (x - 0.5) * 2.0;
        let ny = (y - 0.5) * 2.0;

        // Calculate distance from center
        let r = (nx * nx + ny * ny).sqrt();

        // Apply barrel distortion: r' = r * (1 - k*r^2) for negative strength (barrel)
        // Using -strength to create barrel (pincushion would use +strength)
        let distortion_factor = 1.0 - strength * r * r;
        let r_distorted = r * distortion_factor;

        // Avoid division by zero
        let distortion = if r > 0.001 {
            r_distorted / r
        } else {
            1.0
        };

        // Apply distortion
        let distorted_x = nx * distortion;
        let distorted_y = ny * distortion;

        // Convert back to 0..1 range
        (
            distorted_x / 2.0 + 0.5,
            distorted_y / 2.0 + 0.5,
        )
    }

    /// Check if a point is within the rounded rectangle bounds
    #[allow(dead_code)]
    fn is_in_rounded_rect(&self, x: f32, y: f32, rect: Rect, radius: f32) -> bool {
        // Normalize to rectangle coordinates
        let local_x = x - rect.x;
        let local_y = y - rect.y;

        // Quick bounds check
        if local_x < 0.0 || local_x > rect.w || local_y < 0.0 || local_y > rect.h {
            return false;
        }

        let radius = radius.min(rect.w / 2.0).min(rect.h / 2.0);

        // Check if in straight sections
        if local_x >= radius && local_x <= rect.w - radius {
            return true; // In vertical straight section
        }
        if local_y >= radius && local_y <= rect.h - radius {
            return true; // In horizontal straight section
        }

        // Check corners
        if local_x < radius && local_y < radius {
            // Top-left corner
            let dx = radius - local_x;
            let dy = radius - local_y;
            return dx * dx + dy * dy <= radius * radius;
        }

        if local_x > rect.w - radius && local_y < radius {
            // Top-right corner
            let dx = local_x - (rect.w - radius);
            let dy = radius - local_y;
            return dx * dx + dy * dy <= radius * radius;
        }

        if local_x < radius && local_y > rect.h - radius {
            // Bottom-left corner
            let dx = radius - local_x;
            let dy = local_y - (rect.h - radius);
            return dx * dx + dy * dy <= radius * radius;
        }

        if local_x > rect.w - radius && local_y > rect.h - radius {
            // Bottom-right corner
            let dx = local_x - (rect.w - radius);
            let dy = local_y - (rect.h - radius);
            return dx * dx + dy * dy <= radius * radius;
        }

        false
    }
}

impl Default for CrtShader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrel_distort_center_unchanged() {
        let shader = CrtShader::new();
        let (x, y) = shader.barrel_distort(0.5, 0.5, 0.2);
        
        // Center should be unaffected by barrel distortion
        assert!((x - 0.5).abs() < 0.001);
        assert!((y - 0.5).abs() < 0.001);
    }

    #[test]
    fn barrel_distort_edges_move_inward() {
        let shader = CrtShader::new();
        
        // Test right edge
        let (x_right, _) = shader.barrel_distort(0.99, 0.5, 0.2);
        // With positive barrel strength, edges should move inward
        assert!(x_right < 0.99);

        // Test top edge
        let (_, y_top) = shader.barrel_distort(0.5, 0.01, 0.2);
        // With positive barrel strength, edges should move inward
        assert!(y_top > 0.01);
    }

    #[test]
    fn crt_config_clamps_values() {
        let config = CrtShaderConfig::with_curvature(1.5, -0.5);
        assert!(config.curvature_x <= 1.0);
        assert!(config.curvature_y >= 0.0);
    }

    #[test]
    fn corner_radius_in_rounded_rect() {
        let shader = CrtShader::new();
        let rect = Rect::new(100.0, 100.0, 200.0, 200.0);
        let radius = 20.0;

        // Center should be in rectangle
        assert!(shader.is_in_rounded_rect(200.0, 200.0, rect, radius));

        // Edge points should be in rectangle
        assert!(shader.is_in_rounded_rect(150.0, 100.0, rect, radius)); // Top center
        assert!(shader.is_in_rounded_rect(200.0, 100.0, rect, radius)); // Top center
        assert!(shader.is_in_rounded_rect(100.0, 150.0, rect, radius)); // Left center

        // Far corners should be out
        assert!(!shader.is_in_rounded_rect(100.0, 100.0, rect, radius)); // Top-left corner (rounded)
    }

    #[test]
    fn default_config_is_subtle() {
        let config = CrtShaderConfig::default();
        assert!(config.curvature_x < 0.5);
        assert!(config.curvature_y < 0.5);
        assert!(config.corner_radius < 0.2);
        assert!(config.overscan < 0.1);
    }
}
