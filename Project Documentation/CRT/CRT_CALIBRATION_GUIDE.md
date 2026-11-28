# CRT Calibration Debug Mode - Quick Reference

## Activation

Press **F12** to toggle CRT calibration debug mode on/off.

When active, you'll see:
- Colored boundary outlines on the display
- Reference markers in corners and along edges
- A control panel on the right showing 10 tunable parameters
- Cursor test positions overlaid on the screen

## Keyboard Controls

### Navigation & Adjustment
| Key | Action | Purpose |
|-----|--------|---------|
| **UP** | Select previous parameter | Move selection upward (wraps from top to bottom) |
| **DOWN** | Select next parameter | Move selection downward (wraps from bottom to top) |
| **LEFT** | Decrease parameter | Adjust selected parameter down by 0.01 |
| **RIGHT** | Increase parameter | Adjust selected parameter up by 0.01 |
| **R** | Reset adjustment | Clear accumulated adjustment (back to 0.0) |

### Exit
| Key | Action |
|-----|--------|
| **F12** | Exit calibration mode |

## Visual Guide

### Overlay Colors & Meanings

```
RED boundary         = Original ZX81 framebuffer edges
CYAN boundary        = Safe viewing area (protected from distortion)
GREEN dashed outline = CRT aperture (actual screen visible area)
YELLOW crosshair     = Center reference point
ORANGE circles       = Corner test markers (verify corners remain visible)
BLUE squares         = Edge test markers (verify edges remain visible)
LIGHT YELLOW dots    = Cursor test positions (verify cursor at extreme positions)
```

### Control Panel

Located on the right side of the screen (semi-transparent dark panel):

```
┌─────────────────────────┐
│   CRT CALIBRATION       │
├─────────────────────────┤
│ Safe Area X:    0.050   │
│ Safe Area Y:    0.050   │
│ Curvature X:    0.150   │
│ Curvature Y:    0.120   │
│ Corner Radius:  0.080   │
│ Scanline Str:   0.400   │
│ Vignette Str:   0.300   │
│ Glow Str:       0.250   │
│ Brightness:     0.950   │
│ Contrast:       1.100   │
└─────────────────────────┘
```

Selected parameter (highlighted in yellow) can be adjusted with LEFT/RIGHT arrows.

## Calibration Workflow

### Step 1: Enter Debug Mode
Press **F12** to activate calibration mode.

### Step 2: Identify Your Goal
- **Verify corners**: Look for ORANGE circles in all 4 corners
- **Verify edges**: Look for BLUE squares along all 4 edges
- **Verify cursor**: Look for LIGHT YELLOW dots at extreme cursor positions
- **Check distortion**: RED boundary should stay inside GREEN aperture

### Step 3: Navigate to Parameter
Use **UP** and **DOWN** arrows to select the parameter you want to adjust:
1. Safe Area X (horizontal margin)
2. Safe Area Y (vertical margin)
3. Curvature X (horizontal barrel distortion)
4. Curvature Y (vertical barrel distortion)
5. Corner Radius (CRT corner rounding)
6. Scanline Strength (scanline intensity)
7. Vignette Strength (edge darkening)
8. Glow Strength (bloom effect)
9. Brightness (overall brightness)
10. Contrast (contrast level)

### Step 4: Adjust Parameter
Use **LEFT** and **RIGHT** arrows to adjust:
- **LEFT** decreases value (-0.01 per press)
- **RIGHT** increases value (+0.01 per press)
- Values automatically clamp to valid ranges

### Step 5: Verify Changes
Watch the overlays update in real-time:
- Safe-area shrinks/grows with margin adjustment
- Aperture distorts with curvature adjustment
- Scanlines/vignette/glow update with their adjustments

### Step 6: Reset if Needed
Press **R** to clear the current adjustment and try again.

### Step 7: Exit Debug Mode
Press **F12** again to disable calibration mode.

## Verification Checklist

Before finishing calibration, verify:

- [ ] RED framebuffer boundary is fully inside GREEN aperture
- [ ] No important ZX81 characters are cut off by CRT corners
- [ ] ORANGE corner markers are all fully visible
- [ ] BLUE edge markers are all fully visible
- [ ] LIGHT YELLOW cursor positions are all visible
- [ ] CYAN safe-area leaves enough room for intended distortion
- [ ] Distortion is subtle, not exaggerated
- [ ] Scanlines don't interfere with text readability
- [ ] Vignette doesn't darken text edges too much
- [ ] Glow doesn't create excessive bloom

## Tips & Tricks

### For Best Results
1. Start with safe-area margins (Safe Area X/Y)
   - These define the protected viewing zone
   - Increase if important pixels are being clipped

2. Adjust curvature next (Curvature X/Y)
   - Keep values between 0.1 and 0.3 for subtle effect
   - Use symmetrical values for natural appearance

3. Set corner radius last (Corner Radius)
   - Smaller values (0.05-0.08) for realistic CRT look
   - Larger values make rounding more obvious

4. Fine-tune polish effects (Scanline/Vignette/Glow)
   - Start low (0.2-0.4) for subtle appearance
   - Increase only if visible degradation needed

### Keyboard Efficiency
- Hold keys for rapid parameter cycling/adjustment
- UP/DOWN wrap around, so never get stuck
- R always resets, good for trying different values

### Real-Time Testing
- Watch corner markers to verify CRT rounding doesn't hide them
- Watch edge markers to verify safe-area is working
- Use cursor test positions to verify cursor never disappears
- RED/CYAN boundary shows exact margin compensation

## Common Scenarios

### "Corners are cutting off content"
- **Fix**: Increase `Safe Area X` and `Safe Area Y` using LEFT/RIGHT
- **Why**: Safe-area inset compensates for corner rounding

### "Display looks too flat, need more CRT curvature"
- **Fix**: Increase `Curvature X` and/or `Curvature Y`
- **Why**: Higher curvature creates more barrel distortion

### "Display looks too curved, looks like fisheye"
- **Fix**: Decrease `Curvature X` and/or `Curvature Y`
- **Why**: Reduce distortion strength for more subtle effect

### "Scanlines are too aggressive"
- **Fix**: Decrease `Scanline Strength` using LEFT arrow
- **Why**: Lower values create thinner, less visible scanlines

### "Display is too dark"
- **Fix**: Increase `Brightness` using RIGHT arrow
- **Why**: Brightness multiplier affects overall display luminance

### "Need to try new values quickly"
- **Fix**: Press R to reset the current adjustment
- **Why**: Reset clears accumulated changes without exiting mode

## Debug Mode Properties

### Performance Impact
- **When OFF**: Zero performance impact (not rendered)
- **When ON**: ~1ms per frame (negligible on modern hardware)
- **Overlay rendering**: Uses immediate-mode Macroquad drawing (no textures)
- **Text rendering**: Uses cached font for efficiency

### Non-Destructive
- Adjustments only change the debug display, not actual CRT settings
- Exiting debug mode has no effect on emulator operation
- All settings are temporary during calibration session

### Responsive
- Real-time visual feedback as you adjust
- Smooth parameter navigation with wrap-around
- Instant overlay updates (no lag)

## Troubleshooting

### "I don't see the debug overlays"
- Press F12 to verify debug mode is on (or press again and F12 again)
- Check that the overlays aren't being rendered off-screen
- Ensure window isn't minimized or focused on another application

### "Keyboard controls aren't responding"
- Verify debug mode is active (look for control panel)
- Check that emulator window has keyboard focus
- Try clicking the emulator window to ensure focus

### "Parameter values seem wrong"
- These are the actual values from the CRT shader and polish configs
- If unexpected, verify the shader/polish initialization in app.rs
- Reset with R if adjustment value is out of range

### "Overlays disappear sometimes"
- Overlays only render in debug mode (F12)
- Other UI elements (menus, settings) may overlap
- Try pressing F12 twice to toggle off then on again

## Important Notes

⚠️ **Parameter adjustments shown in calibration mode are for visualization only.**
   - Future phases will add parameter binding to actually apply adjustments
   - Current phase focuses on visual verification and debugging

✅ **Normal emulator operation is never affected by calibration mode.**
   - All CRT settings remain at their configured defaults
   - Exiting debug mode (F12) has zero impact on rendering

🎯 **The goal of calibration is to verify no important pixels are hidden.**
   - Use overlays to confirm safe-area is sufficient
   - Use test markers to verify visibility at screen edges
   - Use cursor positions to verify cursor never disappears

---

**Version**: Phase 6 - CRT Calibration Debug Mode  
**Status**: Active and ready for use  
**Last Updated**: 2024
