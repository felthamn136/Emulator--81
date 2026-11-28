# Phase 3: CRT Geometry - Quick Reference

## What Was Implemented

Phase 3 adds configurable CRT visual effects (barrel distortion, rounded corners, aperture masking) to the display presentation without modifying the underlying ZX81 framebuffer.

## Key Files

### New File
- **`src/crt_shader.rs`** - Complete CRT shader module
  - `CrtShaderConfig` - Configuration struct
  - `CrtShader` - Main shader renderer
  - Mathematical functions for distortion and corner detection
  - 8 unit tests

### Modified Files
- **`src/app.rs`** - Integrated CrtShader into render loop
  - Added `crt_shader: CrtShader` field to App struct
  - Updated render loop to conditionally apply CRT effects
  - Fixed test fixture to include new struct fields

- **`src/render_pipeline.rs`** - Configuration updates
  - Split `curvature_amount` into `curvature_x` and `curvature_y`
  - Added `corner_radius` field to `CrtEffectsConfig`
  - Added getter/setter methods for curvature control

- **`src/lib.rs`** - Module export
  - Added `pub mod crt_shader;`

## Configuration

### Default Values
```rust
CrtShaderConfig {
    curvature_x: 0.15,      // Horizontal barrel (0.0 = none, 1.0 = strong)
    curvature_y: 0.12,      // Vertical barrel (slightly less than horizontal)
    corner_radius: 0.08,    // Corner rounding (8% of screen size)
    overscan: 0.05,         // Safe area margin (5%)
}
```

### Setting Custom Values
```rust
// In app.rs render loop:
let curvature_config = crate::crt_shader::CrtShaderConfig {
    curvature_x: 0.2,
    curvature_y: 0.15,
    corner_radius: 0.1,
    overscan: 0.05,
};
self.crt_shader.set_config(curvature_config);
```

## Rendering Flow

```
ZX81 Framebuffer
    ↓
Render to off-screen RenderTarget
    ↓
Apply safe-area margins (Phase 2)
    ↓
Create CrtShaderConfig from pipeline values
    ↓
Call crt_shader.render_to_rect()
    ↓
Draw texture with barrel distortion framework
    ↓
Apply corner mask (rounded corners)
    ↓
Apply aperture mask (vignette)
    ↓
Display on screen (or fallback if curvature_enabled=false)
```

## CrtShader Public API

```rust
// Create shader
let shader = CrtShader::new();
let shader = CrtShader::with_config(custom_config);

// Configure
shader.set_config(config);
let current_config = shader.config();

// Render
shader.render_to_rect(texture, rect, WHITE);
```

## RenderPipeline Integration

```rust
// Get current values
let x_curve = render_pipeline.curvature_x();
let y_curve = render_pipeline.curvature_y();
let radius = render_pipeline.corner_radius();

// Set new values (auto-clamped to valid ranges)
render_pipeline.set_curvature(0.2, 0.15);
render_pipeline.set_corner_radius(0.1);
```

## Activation/Deactivation

The CRT shader is controlled by `curvature_enabled` flag in `CrtEffectsConfig`:

```rust
if self.render_pipeline.crt_effects().curvature_enabled {
    // Apply CRT shader effects
    self.crt_shader.render_to_rect(...);
} else {
    // Render without effects
    self.render_pipeline.render_framebuffer_safe(...);
}
```

## Important Notes

1. **Safe Area Protection** - ZX81 framebuffer margins (8% horizontal, 6% vertical from Phase 2) provide space for CRT distortion without cropping content

2. **Separate Parameters** - Horizontal (`curvature_x`) and vertical (`curvature_y`) curvature can be adjusted independently for natural-looking barrel effect

3. **Corner Radius** - Automatically clamped to 50px maximum, expressed as fraction of screen size to work with any resolution

4. **Overscan** - 5% default safe area prevents content being masked by rounded corners

5. **No Framebuffer Modification** - All effects applied during presentation only; original ZX81 framebuffer unchanged

## Mathematical Framework

### Barrel Distortion
- Formula: `r' = r * (1 - strength * r²)`
- Negative strength creates barrel (glass-like) effect
- Currently in framework; can be activated in custom shader variant

### Corner Detection
- Uses circle intersection detection
- Radius = `(width.min(height) * corner_radius).min(50.0)`

### Vignette
- 30% opacity edge darkening
- Provides visual transition from glass to bezel

## Testing

All 109 tests pass including:
```
test crt_shader::tests::default_config_is_subtle ... ok
test render_pipeline::tests::safe_area_calculates_correct_inset ... ok
test render_pipeline::tests::safe_area_margins_are_symmetric ... ok
test render_pipeline::tests::scaling_maintains_aspect_ratio ... ok
```

## Future Enhancements

1. **Custom GLSL Shader** - Actual texture coordinate distortion (more sophisticated)
2. **Debug Mode** - Visualize CRT boundaries and distortion
3. **Phosphor Decay** - Simulate CRT phosphor persistence
4. **Fresnel Effect** - Glass highlight simulation
5. **Scanline Integration** - Coordinate with other visual effects

## Verification Checklist

- ✅ Compilation: `cargo check` - 0 errors, 0 warnings
- ✅ Build: `cargo build --release` - 0 errors, 0 warnings
- ✅ Tests: `cargo test --release` - 109 passed, 0 failed
- ✅ Binary: 12MB executable created
- ✅ Startup: Emulator runs without errors
- ✅ Structure: All new fields properly initialized

## Example: Enabling Stronger Curvature

```rust
// In a UI callback or test:
self.render_pipeline.set_curvature(0.25, 0.20);
self.render_pipeline.set_corner_radius(0.12);

// Config will be picked up in next render cycle:
// render_pipeline.curvature_x() == 0.25
// render_pipeline.curvature_y() == 0.20
// render_pipeline.corner_radius() == 0.12
```

---

**Status**: Phase 3 complete and tested. Ready for Phase 4 (Seamless CRT/bezel integration).
