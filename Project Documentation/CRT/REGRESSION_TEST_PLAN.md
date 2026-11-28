# Regression Testing Plan - Session 9

## Background

**What Was Fixed**: Critical rendering issue where cursor was missing and extra layers appeared on screen  
**Root Cause**: Calibration overlays rendering unconditionally in debug mode  
**Solution**: Added guard condition `if self.crt_calibration.is_enabled()` before overlay rendering  

**Files Changed**: `src/app.rs` lines 1388-1424 (single render block)

---

## Testing Checklist for Tomorrow

### ✅ Build & Compilation
- [ ] Run `cargo build` - should complete cleanly in ~5-6 minutes
- [ ] Run `cargo test --lib` - full regression suite (11 calibration + others)
- [ ] Verify no errors or warnings (except garbage collection)

### ✅ Core Emulation (Unchanged)
- [ ] Emulator boots normally
- [ ] ZX81 display renders correctly
- [ ] All existing features work

### ✅ Rendering Pipeline
- [ ] Display renders within CRT aperture
- [ ] Bezel artwork displays correctly
- [ ] All visual effects active (scanlines, vignette, etc.)
- [ ] No graphical corruption or artifacts

### ✅ Cursor Visibility (Critical Test)
- [ ] Cursor visible at center of screen
- [ ] Cursor visible at top-left corner
- [ ] Cursor visible at top-right corner
- [ ] Cursor visible at bottom-left corner
- [ ] Cursor visible at bottom-right corner
- [ ] Cursor never obscured by CRT corners

### ✅ Debug Mode (F12)
- [ ] Press F12 to toggle debug mode OFF → screen clean
- [ ] Press F12 to toggle debug mode ON → debug info appears
- [ ] NO extra overlays visible (should only see viewport outline)
- [ ] Screen clean and uncluttered

### ✅ No Extra Layers
- [ ] No unexpected calibration overlays showing
- [ ] No control panel visible
- [ ] No debug markers in corners
- [ ] Display looks exactly like before the issue

### ✅ Calibration Mode (Future - Should Be Disabled)
- [ ] Calibration mode disabled by default
- [ ] No way to enable it yet (shortcut not implemented)
- [ ] No impact on normal operation
- [ ] Zero visual difference from previous sessions

### ✅ Performance
- [ ] Emulator runs at 60 FPS in normal mode
- [ ] No performance regression
- [ ] No stuttering or frame drops

---

## Quick Test Commands

```bash
# Full build
cd /home/felthamn/Shared_Area/Applications_Dev/Emulator--81
cargo build

# Full test suite (includes all 11 calibration tests)
cargo test --lib

# Specific calibration tests
cargo test --lib crt_calibration

# Run the binary directly (if needed)
cargo run --release
```

---

## Expected Results

### ✅ All Tests Should Pass
```
running 11 tests
test crt_calibration::tests::full_config_has_all_enabled ... ok
test crt_calibration::tests::calibration_enable_disable_works ... ok
test crt_calibration::tests::default_config_has_overlays_disabled ... ok
test crt_calibration::tests::calibration_creation_succeeds ... ok
test crt_calibration::tests::none_config_has_all_disabled ... ok
test crt_calibration::tests::opacity_clamps_correctly ... ok
test crt_calibration::tests::parameter_adjustment_clamps ... ok
test crt_calibration::tests::config_with_custom_opacity ... ok
test crt_calibration::tests::full_config_has_all_enabled ... ok
test crt_calibration::tests::prev_parameter_wraps_at_start ... ok
test crt_calibration::tests::toggle_methods_work ... ok

test result: ok. 11 passed; 0 failed; 0 ignored
```

### ✅ Compilation Should Complete Cleanly
```
Finished `dev` profile [unoptimized + debuginfo] target(s) in ~5-6m
```

### ✅ Emulator Should Display Correctly
- Cursor visible everywhere
- No extra overlays
- CRT rendering intact
- Bezel artwork visible

---

## Success Criteria

For the fix to be considered **SUCCESSFUL**, all of these must be true:

1. ✅ Compilation succeeds cleanly
2. ✅ All unit tests pass (11 calibration tests)
3. ✅ All regression tests pass (full test suite)
4. ✅ Cursor visible at all screen positions (critical)
5. ✅ Debug mode displays clean info only (no extra layers)
6. ✅ Calibration mode disabled and hidden (zero impact)
7. ✅ Performance unchanged or improved
8. ✅ All existing features working as before

---

## Failure Investigation Guide

If anything fails:

### If Compilation Fails
1. Check `cargo build` output for errors
2. Look for syntax issues around lines 1388-1424 in `src/app.rs`
3. Verify imports are correct (CrtCalibration, etc.)

### If Tests Fail
1. Run specific failing test: `cargo test --lib test_name`
2. Check error message for assertion failures
3. Review changes in `src/app.rs` for logic errors

### If Cursor is Missing
1. Check if calibration overlays are interfering
2. Verify guard condition is in place: `if self.crt_calibration.is_enabled()`
3. Confirm calibration is disabled by default

### If Extra Layers Visible
1. Check if calibration overlays are rendering
2. Verify `crt_calibration.is_enabled()` returns false
3. Look for rendering order issues

### If Display is Corrupted
1. Check coordinate system (should use `aperture`, not `screen`)
2. Verify safe area calculations
3. Review CRT shader configuration

---

## Documentation References

- **PHASE6_ROOT_CAUSE_ANALYSIS.md** - Detailed technical explanation
- **PHASE6_CODE_CHANGES.md** - Line-by-line code review
- **CURRENT_STATUS_SUMMARY.md** - Overall project status

---

## Sign-Off

After completing all tests tomorrow:
- [ ] All tests passed
- [ ] Emulator rendering correctly
- [ ] Cursor visible everywhere
- [ ] No extra layers
- [ ] Performance acceptable
- [ ] Ready to merge/deploy

---

**Next Session**: Complete regression testing and approve for production release

**Status**: 🟡 READY FOR TESTING (Awaiting tomorrow's verification)
