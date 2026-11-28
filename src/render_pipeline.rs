/// Display rendering pipeline for the ZX81 emulator.
///
/// This module separates the concerns of ZX81 framebuffer rendering from CRT presentation.
/// The pipeline follows this architecture:
///
/// 1. ZX81 Framebuffer Source (video generation, CPU display buffer)
/// 2. Logical Framebuffer (256×192 monochrome, complete ZX81 output)
/// 3. Off-screen RenderTarget (holds the logical framebuffer)
/// 4. Display Scaling (maintains 4:3 aspect ratio, safe area)
/// 5. CRT Effects (scanlines, brightness, bloom, etc.)
/// 6. CRT Aperture (safe viewing area on physical screen)
/// 7. Bezel Overlay (vintage cabinet artwork)
///
/// Key invariants:
/// - The ZX81 framebuffer is always completely rendered
/// - CRT effects are applied independently of framebuffer generation
/// - Rounding or curvature cannot crop framebuffer pixels
/// - The display maintains a 4:3 presentation ratio
/// - All components are reusable and configurable

use macroquad::prelude::*;

/// Configuration for the framebuffer rendering stage
#[derive(Clone, Debug)]
pub struct FramebufferConfig {
    /// Width of the logical ZX81 framebuffer
    pub width: u32,
    /// Height of the logical ZX81 framebuffer
    pub height: u32,
}

impl Default for FramebufferConfig {
    fn default() -> Self {
        Self {
            width: 256,
            height: 192,
        }
    }
}

/// Configuration for scaling from logical framebuffer to display area
#[derive(Clone, Debug)]
pub struct ScalingConfig {
    /// Preserve the logical 4:3 aspect ratio
    pub maintain_aspect_ratio: bool,
    /// Maximum scaling factor to avoid blurriness
    pub max_scale: f32,
}

impl Default for ScalingConfig {
    fn default() -> Self {
        Self {
            maintain_aspect_ratio: true,
            max_scale: 4.0,
        }
    }
}

/// Configuration for safe area margins to prevent edge obscuring
#[derive(Clone, Debug)]
pub struct SafeAreaConfig {
    /// Horizontal margin as a percentage of framebuffer width (0.0 to 1.0)
    /// 0.0 = no margin, 0.1 = 10% margin on each side
    pub margin_x: f32,
    /// Vertical margin as a percentage of framebuffer height (0.0 to 1.0)
    /// 0.0 = no margin, 0.1 = 10% margin on top and bottom
    pub margin_y: f32,
}

impl Default for SafeAreaConfig {
    fn default() -> Self {
        Self {
            // Default 8% horizontal margin, 6% vertical margin
            // This prevents edge content from being hidden by curvature
            margin_x: 0.08,
            margin_y: 0.06,
        }
    }
}

impl SafeAreaConfig {
    /// Create safe area with specific margins
    pub fn with_margins(margin_x: f32, margin_y: f32) -> Self {
        Self {
            margin_x: margin_x.clamp(0.0, 0.5),
            margin_y: margin_y.clamp(0.0, 0.5),
        }
    }

    /// Calculate the safe area rectangle within the given bounds
    /// Returns a rectangle that is inset from the bounds by the configured margins
    pub fn calculate_safe_rect(&self, bounds: Rect) -> Rect {
        let inset_x = bounds.w * self.margin_x;
        let inset_y = bounds.h * self.margin_y;
        
        Rect::new(
            bounds.x + inset_x,
            bounds.y + inset_y,
            bounds.w - 2.0 * inset_x,
            bounds.h - 2.0 * inset_y,
        )
    }

    /// Calculate margins in pixels for a given display size
    pub fn margins_in_pixels(&self, width: f32, height: f32) -> (f32, f32) {
        (width * self.margin_x, height * self.margin_y)
    }
}

/// Configuration for CRT visual effects
#[derive(Clone, Debug)]
pub struct CrtEffectsConfig {
    /// Enable scanline rendering
    pub scanlines_enabled: bool,
    /// Scanline opacity (0.0 to 1.0)
    pub scanline_strength: f32,
    /// Enable bloom/glow effect
    pub bloom_enabled: bool,
    /// Bloom intensity (0.0 to 1.0)
    pub bloom_strength: f32,
    /// Enable CRT curvature
    pub curvature_enabled: bool,
    /// Horizontal barrel distortion strength (0.0 = none, 1.0 = strong)
    pub curvature_x: f32,
    /// Vertical barrel distortion strength (0.0 = none, 1.0 = strong)
    pub curvature_y: f32,
    /// Corner radius as fraction of screen size (0.0 = sharp, 1.0 = fully rounded)
    pub corner_radius: f32,
    /// Enable screen flicker
    pub flicker_enabled: bool,
    /// Flicker intensity (0.0 to 1.0)
    pub flicker_strength: f32,
}

impl Default for CrtEffectsConfig {
    fn default() -> Self {
        Self {
            scanlines_enabled: true,
            scanline_strength: 0.6,
            bloom_enabled: true,
            bloom_strength: 0.3,
            curvature_enabled: false,
            curvature_x: 0.15,  // Subtle horizontal barrel distortion
            curvature_y: 0.12,  // Slightly less vertical barrel (more natural)
            corner_radius: 0.08, // Moderate corner rounding
            flicker_enabled: true,
            flicker_strength: 0.035,
        }
    }
}

/// Configuration for the aperture (safe viewing area on physical screen)
#[derive(Clone, Debug)]
pub struct ApertureConfig {
    /// Physical screen rectangle
    pub screen_rect: Rect,
    /// Inset from edges (in pixels) where content is guaranteed safe
    pub safe_margin: f32,
}

/// The complete rendering pipeline configuration
#[derive(Clone, Debug)]
pub struct RenderPipelineConfig {
    pub framebuffer: FramebufferConfig,
    pub scaling: ScalingConfig,
    pub safe_area: SafeAreaConfig,
    pub crt_effects: CrtEffectsConfig,
    pub aperture: ApertureConfig,
}

impl RenderPipelineConfig {
    pub fn new(screen_rect: Rect) -> Self {
        Self {
            framebuffer: FramebufferConfig::default(),
            scaling: ScalingConfig::default(),
            safe_area: SafeAreaConfig::default(),
            crt_effects: CrtEffectsConfig::default(),
            aperture: ApertureConfig {
                screen_rect,
                safe_margin: 10.0,
            },
        }
    }
}

/// Manages the off-screen rendering target for the ZX81 framebuffer
pub struct FramebufferTarget {
    /// Off-screen render target for the logical framebuffer
    target: RenderTarget,
    /// Configuration
    config: FramebufferConfig,
}

impl FramebufferTarget {
    /// Create a new framebuffer render target
    pub fn new(config: FramebufferConfig) -> Self {
        let target = render_target(config.width, config.height);
        Self { target, config }
    }

    /// Begin rendering to the framebuffer
    pub fn begin_render(&mut self) {
        push_camera_state();
        set_camera(&Camera2D {
            render_target: Some(self.target.clone()),
            ..Camera2D::from_display_rect(Rect::new(0.0, 0.0, self.config.width as f32, self.config.height as f32))
        });
    }

    /// End rendering to the framebuffer and restore the previous camera.
    pub fn end_render(&self) {
        pop_camera_state();
    }

    /// Get the texture for the rendered framebuffer
    pub fn texture(&self) -> &Texture2D {
        &self.target.texture
    }

    /// Get the framebuffer configuration
    pub fn config(&self) -> &FramebufferConfig {
        &self.config
    }
}

/// Manages scaling from logical framebuffer to display area while maintaining aspect ratio
#[derive(Clone)]
pub struct DisplayScaler {
    config: ScalingConfig,
}

impl DisplayScaler {
    /// Create a new display scaler
    pub fn new(config: ScalingConfig) -> Self {
        Self { config }
    }

    /// Calculate the scaled rectangle that maintains aspect ratio
    /// within the target display area
    pub fn calculate_display_rect(&self, target_area: Rect) -> Rect {
        if !self.config.maintain_aspect_ratio {
            return target_area;
        }

        let aspect_ratio = 256.0 / 192.0; // 4:3
        let area_ratio = target_area.w / target_area.h;

        let (width, height) = if area_ratio > aspect_ratio {
            // Target area is wider than 4:3
            let h = target_area.h;
            let w = h * aspect_ratio;
            (w, h)
        } else {
            // Target area is taller than 4:3
            let w = target_area.w;
            let h = w / aspect_ratio;
            (w, h)
        };

        let x = target_area.x + (target_area.w - width) / 2.0;
        let y = target_area.y + (target_area.h - height) / 2.0;

        Rect::new(x, y, width, height)
    }

    /// Update the configuration
    pub fn set_config(&mut self, config: ScalingConfig) {
        self.config = config;
    }
}

/// Renders the scaled framebuffer texture to the display area
pub struct FramebufferRenderer {
    scaler: DisplayScaler,
}

impl FramebufferRenderer {
    /// Create a new framebuffer renderer
    pub fn new(scaler: DisplayScaler) -> Self {
        Self { scaler }
    }

    /// Render the framebuffer texture to the specified area
    pub fn render(
        &self,
        texture: &Texture2D,
        target_area: Rect,
        tint: Color,
    ) {
        let display_rect = self.scaler.calculate_display_rect(target_area);

        draw_texture_ex(
            texture,
            display_rect.x,
            display_rect.y,
            tint,
            DrawTextureParams {
                dest_size: Some(vec2(display_rect.w, display_rect.h)),
                flip_y: true,
                ..Default::default()
            },
        );
    }
}

/// Manages the complete rendering pipeline
pub struct RenderPipeline {
    pub framebuffer: FramebufferTarget,
    pub scaler: DisplayScaler,
    pub renderer: FramebufferRenderer,
    config: RenderPipelineConfig,
}

impl RenderPipeline {
    /// Create a new render pipeline with the given configuration
    pub fn new(config: RenderPipelineConfig) -> Self {
        let framebuffer = FramebufferTarget::new(config.framebuffer.clone());
        let scaler = DisplayScaler::new(config.scaling.clone());
        let renderer = FramebufferRenderer::new(scaler.clone());

        Self {
            framebuffer,
            scaler,
            renderer,
            config,
        }
    }

    /// Create a render pipeline with default configuration for a screen area
    pub fn with_screen(screen_rect: Rect) -> Self {
        let config = RenderPipelineConfig::new(screen_rect);
        Self::new(config)
    }

    /// Begin rendering to the off-screen framebuffer
    pub fn begin_framebuffer_render(&mut self) {
        self.framebuffer.begin_render();
    }

    /// End rendering to the framebuffer and restore the previous camera.
    pub fn end_framebuffer_render(&self) {
        self.framebuffer.end_render();
    }

    /// Render the framebuffer to the display area with optional tint
    pub fn render_framebuffer(
        &self,
        target_area: Rect,
        tint: Color,
    ) {
        self.renderer.render(
            self.framebuffer.texture(),
            target_area,
            tint,
        );
    }

    /// Get the logical display rect (accounting for aspect ratio)
    pub fn logical_display_rect(&self, target_area: Rect) -> Rect {
        self.scaler.calculate_display_rect(target_area)
    }

    /// Update CRT effects configuration
    pub fn set_crt_effects(&mut self, effects: CrtEffectsConfig) {
        self.config.crt_effects = effects;
    }

    /// Get current CRT effects configuration
    pub fn crt_effects(&self) -> &CrtEffectsConfig {
        &self.config.crt_effects
    }

    /// Update scaling configuration
    pub fn set_scaling(&mut self, scaling: ScalingConfig) {
        self.scaler.set_config(scaling.clone());
        self.config.scaling = scaling;
    }

    /// Get aperture configuration
    pub fn aperture(&self) -> &ApertureConfig {
        &self.config.aperture
    }

    /// Get the framebuffer configuration
    pub fn framebuffer_config(&self) -> &FramebufferConfig {
        &self.config.framebuffer
    }

    /// Calculate the safe area rectangle for the given target
    /// Safe area prevents CRT distortion from obscuring edge content
    pub fn safe_area_rect(&self, target_area: Rect) -> Rect {
        self.config.safe_area.calculate_safe_rect(target_area)
    }

    /// Get safe area configuration
    pub fn safe_area(&self) -> &SafeAreaConfig {
        &self.config.safe_area
    }

    /// Update safe area configuration
    pub fn set_safe_area(&mut self, safe_area: SafeAreaConfig) {
        self.config.safe_area = safe_area;
    }

    /// Get display rect accounting for safe area
    /// This is the rectangle where framebuffer content should be visible
    pub fn safe_display_rect(&self, target_area: Rect) -> Rect {
        let safe_area = self.safe_area_rect(target_area);
        self.logical_display_rect(safe_area)
    }

    /// Logical rectangle occupied by the ZX81 framebuffer render target.
    pub fn logical_framebuffer_rect(&self) -> Rect {
        Rect::new(
            0.0,
            0.0,
            self.framebuffer.config.width as f32,
            self.framebuffer.config.height as f32,
        )
    }

    /// Render the framebuffer to the display area within safe margins
    pub fn render_framebuffer_safe(
        &self,
        target_area: Rect,
        tint: Color,
    ) {
        // Calculate safe area first, then scale within that
        let safe_rect = self.safe_area_rect(target_area);
        self.render_framebuffer(safe_rect, tint);
    }

    /// Get horizontal barrel distortion strength
    pub fn curvature_x(&self) -> f32 {
        self.config.crt_effects.curvature_x
    }

    /// Get vertical barrel distortion strength
    pub fn curvature_y(&self) -> f32 {
        self.config.crt_effects.curvature_y
    }

    /// Get corner radius
    pub fn corner_radius(&self) -> f32 {
        self.config.crt_effects.corner_radius
    }

    /// Set barrel distortion curvature
    pub fn set_curvature(&mut self, curvature_x: f32, curvature_y: f32) {
        self.config.crt_effects.curvature_x = curvature_x.clamp(0.0, 1.0);
        self.config.crt_effects.curvature_y = curvature_y.clamp(0.0, 1.0);
    }

    /// Set corner radius
    pub fn set_corner_radius(&mut self, radius: f32) {
        self.config.crt_effects.corner_radius = radius.clamp(0.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaling_maintains_aspect_ratio() {
        let scaler = DisplayScaler::new(ScalingConfig::default());
        
        // Wide area
        let wide = Rect::new(0.0, 0.0, 800.0, 400.0);
        let result = scaler.calculate_display_rect(wide);
        let aspect = result.w / result.h;
        assert!((aspect - 4.0 / 3.0).abs() < 0.001);
        
        // Tall area
        let tall = Rect::new(0.0, 0.0, 400.0, 600.0);
        let result = scaler.calculate_display_rect(tall);
        let aspect = result.w / result.h;
        assert!((aspect - 4.0 / 3.0).abs() < 0.001);
    }

    #[test]
    fn scaling_centers_in_target_area() {
        let scaler = DisplayScaler::new(ScalingConfig::default());
        let target = Rect::new(100.0, 50.0, 800.0, 600.0);
        let result = scaler.calculate_display_rect(target);
        
        // Should be centered within target
        assert!(result.x >= target.x);
        assert!(result.y >= target.y);
        assert!(result.x + result.w <= target.x + target.w);
        assert!(result.y + result.h <= target.y + target.h);
    }

    #[test]
    fn safe_area_calculates_correct_inset() {
        let safe_area = SafeAreaConfig::with_margins(0.1, 0.05);
        let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
        let safe_rect = safe_area.calculate_safe_rect(bounds);

        // 10% horizontal = 40px per side = 80px total removed
        // 400 - 80 = 320
        assert!((safe_rect.w - 320.0).abs() < 0.01);
        // 5% vertical = 15px per side = 30px total removed
        // 300 - 30 = 270
        assert!((safe_rect.h - 270.0).abs() < 0.01);
        // Should be centered
        assert!((safe_rect.x - 40.0).abs() < 0.01);
        assert!((safe_rect.y - 15.0).abs() < 0.01);
    }

    #[test]
    fn safe_area_margins_are_symmetric() {
        let safe_area = SafeAreaConfig::with_margins(0.1, 0.1);
        let bounds = Rect::new(0.0, 0.0, 1000.0, 1000.0);
        let safe_rect = safe_area.calculate_safe_rect(bounds);

        // Calculate margins
        let left_margin = safe_rect.x - bounds.x;
        let right_margin = (bounds.x + bounds.w) - (safe_rect.x + safe_rect.w);
        let top_margin = safe_rect.y - bounds.y;
        let bottom_margin = (bounds.y + bounds.h) - (safe_rect.y + safe_rect.h);

        // All margins should be equal (symmetric)
        assert!((left_margin - right_margin).abs() < 0.01);
        assert!((top_margin - bottom_margin).abs() < 0.01);
        assert!((left_margin - 100.0).abs() < 0.01); // 10% of 1000
    }

    #[test]
    fn safe_area_margins_clamp_to_valid_range() {
        // Trying to set margin > 50% should be clamped
        let safe_area = SafeAreaConfig::with_margins(0.6, 0.6);
        assert_eq!(safe_area.margin_x, 0.5);
        assert_eq!(safe_area.margin_y, 0.5);

        // Negative values should be clamped to 0.0
        let safe_area = SafeAreaConfig::with_margins(-0.1, -0.1);
        assert_eq!(safe_area.margin_x, 0.0);
        assert_eq!(safe_area.margin_y, 0.0);
    }

}
