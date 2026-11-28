# Phase 4: Seamless CRT/Bezel Integration - Quick Reference

## What Was Implemented

Phase 4 adds layered CRT composition effects that seamlessly integrate the display with the monitor bezel artwork, creating the illusion of a glass CRT tube physically recessed into the monitor casing.

## Key Components

### New Module: `src/crt_composition.rs`

**Configuration struct: CrtCompositionConfig**
```rust
pub struct CrtCompositionConfig {
    pub inner_shadow_strength: f32,      // 0.0 - 1.0, default 0.5
    pub shadow_blur_radius: f32,         // pixels, default 12.0
    pub edge_darkening: f32,             // 0.0 - 1.0, default 0.3
    pub glass_highlight_opacity: f32,    // 0.0 - 1.0, default 0.04
    pub fresnel_strength: f32,           // 0.0 - 1.0, default 0.15
    pub vignette_softness: f32,          // fraction, default 0.15
}
```

**Main struct: CrtComposition**
```rust
pub struct CrtComposition {
    config: CrtCompositionConfig,
}

impl CrtComposition {
    pub fn new() -> Self
    pub fn with_config(config: CrtCompositionConfig) -> Self
    pub fn set_config(&mut self, config: CrtCompositionConfig)
    pub fn config(&self) -> &CrtCompositionConfig
    
    // Rendering methods
    pub fn apply_all_effects(&self, aperture: Rect)
    pub fn apply_all_effects_with_brightness(&self, aperture: Rect, brightness: f32)
    
    // Individual effects
    pub fn draw_inner_shadow(&self, aperture: Rect)
    pub fn draw_edge_darkening(&self, aperture: Rect)
    pub fn draw_glass_highlight(&self, aperture: Rect)
    pub fn draw_glass_bezel_transition(&self, aperture: Rect)
}
```

## Rendering Layers

The seamless CRT/bezel integration uses a 5-layer composition:

```
1. Bezel/Background     ← TV cabinet artwork
2. CRT Glass/Display    ← ZX81 framebuffer + Phase 3 shader
3. Inner CRT Shadow     ← NEW: Recession effect
4. Glass Highlight      ← NEW: Subtle reflection
5. Foreground Bezel     ← Control labels, reflections
```

## Visual Effects

### Inner Shadow (Recession)
- **Purpose**: Creates illusion that glass is recessed into monitor casing
- **Implementation**: 4-layer frame effect with decreasing alpha
- **Configuration**: `inner_shadow_strength`, `shadow_blur_radius`, `vignette_softness`
- **Default**: 0.5 strength, 12px blur, creates obvious but subtle effect

### Edge Darkening (Vignette)
- **Purpose**: Subtle darkening at edges, adds depth
- **Implementation**: Top/bottom/left/right rectangles at aperture edges
- **Configuration**: `edge_darkening`
- **Default**: 0.3 (30% darkening)

### Glass Highlight
- **Purpose**: Very subtle light reflection on curved glass surface
- **Implementation**: Circle-based glow in top-left quadrant
- **Configuration**: `glass_highlight_opacity`, `fresnel_strength`
- **Default**: 0.04 opacity (barely visible)

### Glass-Bezel Transition
- **Purpose**: Eliminate harsh rectangular boundary between glass and bezel
- **Implementation**: Anti-aliased border using thin dark rectangles
- **Configuration**: Width = 3% of aperture height, 15% opacity
- **Default**: Smooth 1-2 pixel border

## Integration with App.rs

### Struct Field
```rust
struct App {
    // ... other fields ...
    crt_composition: CrtComposition,
}
```

### Initialization
```rust
let crt_composition = CrtComposition::new();
// In App struct initialization:
crt_composition,
```

### Render Loop Integration
In the `RenderLayer::CrtShadowMask` case:
```rust
RenderLayer::CrtShadowMask => {
    // Existing vignette
    self.crt.draw_vignette(&self.bezel, &crt_scaling, screen);
    
    // NEW: Composition effects
    self.crt_composition.apply_all_effects_with_brightness(
        aperture,
        effects.brightness,
    );
}
```

## Configuration Usage

### Create with Defaults
```rust
let composition = CrtComposition::new();
```

### Custom Configuration
```rust
let mut config = CrtCompositionConfig::default();
config.set_inner_shadow_strength(0.6);
config.set_edge_darkening(0.4);
config.set_glass_highlight(0.08);

let composition = CrtComposition::with_config(config);
```

### Runtime Updates
```rust
let mut composition = app.crt_composition;
let mut config = composition.config().clone();
config.set_inner_shadow_strength(0.3); // Reduce shadow
composition.set_config(config);
app.crt_composition = composition;
```

### Brightness Scaling
```rust
// Effects automatically fade with display brightness
composition.apply_all_effects_with_brightness(aperture, 0.5);
// At 0.5 brightness: all opacities halved
// At 0.0 brightness: effects disabled completely
// At 1.0 brightness: full configured opacity
```

## Default Values Explained

| Parameter | Default | Range | Purpose |
|-----------|---------|-------|---------|
| `inner_shadow_strength` | 0.5 | 0.0–1.0 | Recession opacity (50% at max) |
| `shadow_blur_radius` | 12.0 | pixels | Soft edge transition width |
| `edge_darkening` | 0.3 | 0.0–1.0 | Vignette intensity (30%) |
| `glass_highlight_opacity` | 0.04 | 0.0–1.0 | Reflection glow (4%, very subtle) |
| `fresnel_strength` | 0.15 | 0.0–1.0 | Edge highlight (15% additional) |
| `vignette_softness` | 0.15 | fraction | Transition zone (15% of aperture) |

## Effect Characteristics

- **Style**: Subtle, period-appropriate 1970s/1980s CRT aesthetic
- **Performance**: Minimal - immediate-mode drawing with no texture processing
- **Responsiveness**: All measurements use relative percentages, scales with window
- **Brightness control**: Automatically fades with display illumination
- **GPU overhead**: Minimal; primarily CPU-side geometric calculations

## Testing

7 unit tests covering:
- Default configuration is subtle
- Parameter bounds clamping
- Builder pattern functionality
- Object initialization
- Configuration mutation
- Brightness scaling reduces effects
- Zero brightness disables effects

All tests passing (116/116 total suite).

## Rendering Pipeline Context

**Complete render order after Phase 4:**

1. TvShadow - Bezel shadow (existing)
2. TvCabinet - Bezel artwork (existing)
3. Viewport - ZX81 display (Phase 1-3 combined)
   - Rendered to off-screen target (Phase 1)
   - Safe-area scaling (Phase 2)
   - CRT shader effects (Phase 3)
4. **CrtShadowMask** - **NEW composition effects added here**
   - Original vignette (existing)
   - Inner shadow (Phase 4)
   - Edge darkening (Phase 4)
5. CrtGlass - Glass texture (existing)
6. CrtReflections - Screen reflections (existing)

## Common Customizations

### Stronger Recession Effect
```rust
let config = CrtCompositionConfig::with_shadow_strength(0.8);
composition.set_config(config);
```

### Disable Glass Highlight
```rust
let mut config = composition.config().clone();
config.set_glass_highlight(0.0);
composition.set_config(config);
```

### Increased Edge Vignetting
```rust
let mut config = composition.config().clone();
config.set_edge_darkening(0.5);
composition.set_config(config);
```

### Subtle Everything (Minimal Effect)
```rust
let config = CrtCompositionConfig {
    inner_shadow_strength: 0.2,
    edge_darkening: 0.1,
    glass_highlight_opacity: 0.01,
    fresnel_strength: 0.05,
    ..Default::default()
};
composition.set_config(config);
```

## Files Modified

| File | Type | Changes |
|------|------|---------|
| `src/crt_composition.rs` | NEW | Complete composition system (520 lines) |
| `src/app.rs` | MODIFIED | Import, field, initialization, render integration (~15 lines) |
| `src/lib.rs` | MODIFIED | Module export (1 line) |

## Compilation Status

- ✅ `cargo check` - Passed (0 errors, 0 warnings)
- ✅ `cargo build --release` - Passed (12MB binary)
- ✅ `cargo test --lib --release` - 116/116 passed

## Next Steps (Phase 5)

After Phase 4 provides seamless integration, Phase 5 will add:
- Scanline effects
- Phosphor simulation
- Enhanced vignetting
- Display glow
- Brightness/contrast controls
- Optional screen noise
- Luminance persistence

---

**Status**: Phase 4 complete. Seamless CRT/bezel integration ready for use.
**Integration point**: Automatically applied during CrtShadowMask render layer.
**Brightness responsive**: Effects fade with display power state.
