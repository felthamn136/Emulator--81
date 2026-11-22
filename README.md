

# ZX81 Emulator Project

![EMULATOR-81 ZX81 Emulator repository banner](assets/repository-banner.png)

A Rust 2021 / Macroquad front-end with an experimental ZX81 machine. It loads
`rom/zx81.rom` automatically when present; without a ROM it runs the original
visual simulation and cannot execute programs. An original ZX81 ROM has been
tested through the K prompt and a `.P` load; the machine is not cycle-accurate.

## AI-assisted development 

Emulator-81 was conceived, designed and developed by Neil Feltham using Microsoft Copilot AI, guided by architectural planning,
prompts, testing and iterative refinement. 

## Why This Project?

The ZX81 was an extraordinary machine that achieved a great deal with extremely limited hardware. Its unusual architecture makes it one of the most interesting classic computers to emulate accurately.

While researching available emulators, I found that relatively few ZX81 emulators are actively maintained for modern Linux systems. Many existing solutions were originally developed for Windows, depend on compatibility layers, or require additional configuration to integrate cleanly with contemporary Linux distributions.

As a long-time retro-computing enthusiast and Linux user, I wanted a modern ZX81 emulator that would run natively on Linux while remaining portable across other platforms. Rather than adapting an existing emulator, I chose to build one from scratch as both a personal challenge and a practical tool that I could use for running original ZX81 software.

This project therefore serves several purposes:

Preserving and exploring the design of the Sinclair ZX81. Creating a modern native ZX81 emulator for personal use on Linux. Learning the challenges involved in accurate CPU, memory, keyboard and video emulation. Investigating how far AI-assisted development can be taken when building a complex software project from the ground up. Demonstrating the capabilities of Rust as a language for retro-computing and emulation projects. Providing a foundation that can continue to evolve with additional features and improvements over time.

Building the emulator from first principles provides a much deeper appreciation of the clever engineering that allowed Sinclair to deliver a complete home computer using such limited hardware resources. It also creates an opportunity to document the development process and share lessons learned with the wider retro-computing community

## Build and run

Install a current stable Rust toolchain, then run from the repository root:

```sh
cargo build --release
cargo run --release # automatically loads rom/zx81.rom if present
# Or select a different legally obtained original 8K ZX81 ROM:
cargo run --release -- --rom /path/to/zx81.rom
```

The application targets Windows, Linux, and macOS. Linux builds need the
X11/XCB development libraries used by the display information crate and ALSA
development libraries (`libasound2-dev`). On Linux,
file dialogs use the XDG desktop portal, with Zenity as a fallback. A working
graphical session is required.

### Release packages

The x86-64 Linux AppImage, Flatpak and Snap bundles, and Windows installer and
portable ZIP in `dist/` include the artwork, games library, and `rom/zx81.rom`.
They can still select a different legally obtained ROM with
`--rom /path/to/zx81.rom`. Existing packages do not automatically include
later source changes; rebuild them to pick up the latest fixes and artwork.

**AppImage:** run the image directly:

```sh
chmod +x dist/Emulator-81-x86_64.AppImage
./dist/Emulator-81-x86_64.AppImage
```

The AppImage requires a graphical session, compatible system graphics and
X11/XCB or Wayland libraries, ALSA, and glibc 2.39 or newer. FUSE is normally
required; if unavailable, pass `--appimage-extract-and-run` before other
arguments. On Wayland, integrate the AppImage with your desktop (for example
with AppImageLauncher) to install its desktop entry and icon; running it
without desktop integration can show a generic taskbar icon.

**Flatpak:** install the bundle and its GNOME 51 runtime from Flathub:

```sh
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user dist/Emulator-81-x86_64.flatpak
flatpak run io.github.felthamn136.Emulator81
```

**Snap:** the locally built Snap is unsigned, so installing it requires
`--dangerous`. Install only a bundle you trust:

```sh
sudo snap install mesa-2604
sudo snap install --dangerous dist/Emulator-81-x86_64.snap
snap run emulator-81
```

The Snap uses the `core26` base and the `mesa-2604` graphics content provider.
If the graphics plug is not automatically connected, run
`sudo snap connect emulator-81:gpu-2604 mesa-2604:gpu-2604`.

**Windows installer:** run `dist/Emulator-81-Setup.msi` to install the
application. The installer is unsigned and may trigger an operating-system
warning. Uninstall any older NSIS-based installation first; the WiX installer
does not automatically replace it. The basic installer uses
`C:\Program Files\Emulator-81`; to choose another folder, run:

```powershell
msiexec /i dist\Emulator-81-Setup.msi INSTALLFOLDER="C:\Emulator-81"
```

**Windows portable:** extract all of
`dist/Emulator-81-Windows-Portable-x86_64.zip`, then run `zx81_emulator.exe`
from the extracted folder. Keep the `assets`, `rom`, and `Games Library`
folders alongside the executable. Windows builds do not open a console, so
stderr-only diagnostics may not be visible when launched from the desktop.

To rebuild the AppImage, WiX installer and Windows portable ZIP from the project
directory, run `sh packaging/build-release.sh`. It requires `appimagetool`,
`zip`, WiX 5, the `x86_64-pc-windows-gnu` Rust target, and a MinGW-w64
cross-compiler and resource compiler (`x86_64-w64-mingw32-windres`). Windows
MSVC builds require `rc.exe` from the Windows SDK. WiX requires Windows; on
Linux, `WIX` can point to a wrapper
that runs WiX under Wine and translates the path arguments to Windows paths.
Wine is not an officially supported WiX host.

To rebuild the Flatpak and Snap on an x86-64 Ubuntu 26.04 host, install Cargo,
Flatpak, Snap, `apt-get`, `dpkg-deb`, `mksquashfs`, and the Flatpak
`org.gnome.Sdk//51` and `org.gnome.Platform//51` runtimes. Then run:

```sh
sh packaging/build-linux-bundles.sh /path/to/a/new/build-directory
```

The script downloads the Ubuntu X11, keyboard data and ALSA packages for the Snap,
leaves intermediate files in that build directory, and writes the bundles to
`dist/`. Both release scripts respect `CARGO_TARGET_DIR` if it is set.
No macOS installer is currently provided; a macOS build requires an Apple
toolchain and SDK.

The window starts at 800 × 600, is resizable, requests high-DPI rendering and
VSync, and is centered on the primary display when the platform supports
positioning. macOS centers the native window; Wayland compositors decide
window placement. The window manager and graphics driver may override these
requests.

| Key | Action |
| --- | --- |
| F11 | Toggle fullscreen |
| Esc | Close About, a menu or settings, cancel fullscreen confirmation, or leave fullscreen |
| F1 | Open/close settings |
| F2 | Toggle scanlines |
| F3 | Toggle reflections |
| F4 | Toggle placeholder curvature shading |
| F6 | Toggle flicker |
| F7 | Toggle the status panel |
| F8 | Pause/run the experimental machine |
| F9 | Single-step while paused when wobble is disabled; trigger wobble when enabled |
| F10 | Reset the experimental machine |
| Ctrl+R | Power cycle |
| F12 | Toggle viewport outline and debug information |

The top menu bar provides:

- **File**: open a `.P` program normally or instantly, load a WAV tape, save a
  program as `.p` or `.wav`, browse the games library, or save a PNG screenshot of the CRT
  (including its visual effects, but not UI overlays).
- **Settings**: open the settings panel, toggle fullscreen, tape loading sound or CRT effects, and
  adjust **SCANLINE STRENGTH** from 0% to 100% with Left/Right in the panel.
  CRT and fullscreen-confirmation toggles show ON or OFF.
- **Machine**: pause or resume, single-step, reset, power cycle, or select 1K
  or 16K RAM. Changing the RAM size resets the machine.
- **Hardware**: enable RAM Pack Wobble, choose its severity, or trigger it.
  Wobble is off by default and affects only the 16K expansion. It can cause
  lost writes, corrupt reads, address glitches, or a temporary disconnection;
  unsaved programs can be damaged. F9 triggers it when enabled, and the Machine
  menu still permits single-stepping.
- **View**: toggle fullscreen, the virtual keyboard, status, or the debug overlay.
- **Help**: open the About window.

Fullscreen confirmation is ON by default. Change it in the Settings menu or
settings panel to choose whether entering and leaving fullscreen require a
Yes/No confirmation. Use the mouse or Left/Right and Enter; Esc cancels.

The virtual keyboard opens in a separate movable window. Click its keys to use
them; click SHIFT to latch it for the next key, or click SHIFT again to cancel.
Click the red FUNCTION label on NEW LINE to enter function mode (SHIFT+NEW LINE),
then click the desired function key. Close the keyboard window, press Esc in
that window, or toggle it off from the View menu.

**File > Save CRT Screenshot** captures the CRT after its visual effects and
opens a save dialog for a `.png` file. The surrounding cabinet and interface
overlays are not included. **Help > About Emulator-81** displays the project
information in a movable panel; close it with its X button or Esc.

Click a menu and then an item; click outside or press Esc to dismiss it.
Available keyboard shortcuts are shown beside their menu items. The red-grid background fills a 4:3
window, and the TV and title scale with it below the menu. The background and
screen retain their 4:3 proportions when the window is resized.
Normal window resizing requests a 4:3 content area. Window managers and
compositors may override this, particularly when maximized, fullscreen or
tiled; the scene remains fitted without stretching in those cases.

Close the window to quit. In settings, use Up/Down to select and Enter, Space,
Left or Right to change an option or activate Reset/Power Cycle. Settings last
for the current run. F4 currently shades the edges rather than warping the
display image; startup beam edges are curved procedurally. Bloom affects the
startup and shutdown beams and simulated text; phosphor glow affects simulated
text only. Neither effect is applied to the active ROM display. Fullscreen status
reports the application's most recent request; the platform may decline a
fullscreen request.

## ZX81 experience

Startup takes about 3.5 seconds: a cold screen, a point and line, vertical
expansion, stabilization and a phosphor warm-up lead to the normal display.
With a ROM, this is the ZX81 display rather than the simulated READY message.
Ctrl+R plays a 1.5-second shutdown and then restarts the warm-up. The F1 CRT
warm-up setting can skip the startup animation; it does not skip shutdown.
The cabinet power indicator is lit while powered on and flashes during a power
cycle.
`CrtWarmupSettings` in `src/experience.rs` contains the startup duration,
bloom and flicker strengths, scanline fade time and startup randomization.
These settings are code defaults, not a persistent configuration file.

With a ROM loaded, choose **File > Load Program (.p)** or drop a `.P` file onto the
window to load and run it. The file dialog requests the home folder when it
can be determined; the Linux Zenity fallback may use its own starting folder.
Choose **File > Games Library (Standard Load)** or **Games Library (Instant
Load)** to open the file picker at the `Games Library` directory and select a
game, including one in a subfolder.
The library is found in the working directory, the `ZX81_Emulator` subfolder
of the working directory, beside the executable, or in the project directory.
The folder must exist; a missing or inaccessible library is reported. Some
desktop file-picker implementations may override the requested starting folder.
The machine pauses while a file dialog is open, but the window remains responsive.
The emulator resets first, so the current program and machine state are lost.
The ROM boots, receives `LOAD ""`, and reads the synthesized tape signal.
Files whose saved data exceeds the selected RAM size are rejected before reset;
choose 16K RAM for programs that do not fit in 1K.
Programs with a saved self-start line can continue automatically; otherwise,
the emulator dismisses the ROM's successful load report when necessary and
enters `RUN` at the BASIC prompt. Tape input is accelerated at a frame-bounded
rate to keep the application responsive, though larger programs can still take
time. Black-and-white zigzag bands follow the emulated tape
signal during playback, approximating the ZX81 loading display; they are not
beam-accurate.
For a faster alternative, choose **File > Open File Instantly**. This still
uses the ROM's load and run logic, but skips the tape pulse timing and
fast-forwards the startup. Instant loading requires a compatible ROM; an
unsupported ROM is reported before the machine is reset.

**Settings > Tape loading sound** enables the optional emulated tape signal
through the default audio output. It is off by default. When enabled, standard
loading runs at tape speed so the sound matches playback; instant loads remain
silent. If audio cannot be initialized or fails, the emulator reports the
error and continues without sound.
F8 pauses and resumes the process; F10 cancels it by resetting the machine.
Invalid files and loading failures are reported.
Without a ROM, dropping a program displays an error instead of simulating a
successful load.

To save the current BASIC program, first finish entering any line and return
to the K prompt, then choose **File > Save Program (.p)**. The native save dialog
allows navigation to the home folder or elsewhere; overwrite confirmation
depends on the platform. Use a `.p` filename. This exports the ZX81 program and
system variables, not a complete emulator save state. Loading it resets the
machine and automatically runs the program.

**File > Save Program as WAV...** exports a snapshot of the current ZX81
SAVE memory range as mono 44.1 kHz, 16-bit PCM. The export runs in the
background and adds `.wav` when the filename has no extension. It uses an
unnamed tape header for `LOAD ""`, followed by the same memory bytes as a
`.p` save. The recording includes a five-second tone, one second of silence
to synchronize the ZX81 loader, and a one-second trailing tone. The tones are
added for playback convenience; the original ZX81 ROM uses a silent leader.

**File > Load WAV Tape...** decodes a PCM WAV recording in the background,
then loads the recovered program using the same ROM-assisted instant-load
workflow as a `.P` file. The machine is reset and the program starts as with
instant `.P` loading; a compatible ZX81 ROM and sufficient selected RAM are
required. The current program is lost once loading starts. F10 cancels loading.
The loader supports mono and stereo 8- or 16-bit PCM recordings at 22.05, 44.1,
and 48 kHz, including the WAVs exported by the emulator. Audio decoding errors
are reported without resetting the machine.

`Experience` owns the power and display states. `EmulatorCore::write_display_buffer`
accepts exactly 256 × 192 row-major 8-bit luminance values to replace the mock
display; `clear_display_buffer` restores it.

With a ROM, `EmulatorCore::refresh_display` decodes an initialized ZX81 display
file using the character glyphs in the loaded ROM. It supports collapsed and
full-width rows and inverse characters; invalid display files return an error.
This is a high-level memory snapshot, not beam-accurate ULA rendering.
The display blanks after approximately 100,000 T-states without a display-file
fetch, so FAST-mode execution does not leave a permanently frozen picture.
This signal detection is an approximation, not a reconstruction of TV sync.

## Experimental CPU and ROM support

The vendored MIT-licensed `z80` 2.0.0 core supplies instruction execution.
Its modifications add M1 opcode-fetch and refresh interrupt callbacks,
register access for debugging, explicit interrupt servicing, and corrections
to EI/NMI handling and interrupt-mode-0 timing. Its license
is in `vendor/z80/LICENSE`. `Memory` models
the 8K ROM and selectable 1K or 16K RAM, with their normal mirrors and ROM
write protection.
`EmulatorCore::load_rom` or `load_rom_bytes` validates an externally supplied
8192-byte ROM; `step_instruction` executes one CPU instruction and returns its
T-state count. In ROM mode, host alphanumeric keys, both Shift keys, Enter,
Space and period drive the active-low matrix. A host comma key also sends
ZX81 Shift+period, supporting keyboards that translate that combination to
a comma before sending it over USB. The release packages include the
repository's 8 KB ZX81 ROM.
The [ZX80/ZX81 ROM archive](https://github.com/spectrumforeveryone/zx-roms/tree/main/zx80-81)
states that its ROMs are used with permission from Nine Tiles.

The `.P` file is validated against its `E_LINE` and display-file pointers and
translated into a synthetic cassette input signal; the ROM completes the load.
For compatibility with library files that contain it, a single trailing byte
beyond `E_LINE` is reported on stderr and ignored.
The generated tape has an unnamed header. `.P81` and `.TAP` are not
supported. The F12 overlay shows CPU registers, the next raw opcode and elapsed
T-states.

For `set_key(row, column, pressed)`, columns are listed left to right:

| Row | Columns 0–4 |
| --- | --- |
| 0 | Shift, Z, X, C, V |
| 1 | A, S, D, F, G |
| 2 | Q, W, E, R, T |
| 3 | 1, 2, 3, 4, 5 |
| 4 | 0, 9, 8, 7, 6 |
| 5 | P, O, I, U, Y |
| 6 | New Line, L, K, J, H |
| 7 | Space, ., M, N, B |

M1 character substitution, refresh-derived interrupts and a 207-T-state line
timer are modeled at instruction boundaries. The NMI begins at horizontal
sync; a CPU already in HALT is held during its NMI acknowledge until sync
ends. WAIT during other bus cycles, sync resets, and beam-accurate video remain
incomplete; compatibility and exact cycle accuracy are not guaranteed. Host
frame stalls are carried forward as emulated time, with catch-up work bounded
per rendered frame. The ZX81 550 and 649 ROMs were tested reaching the inverse
K cursor, entering a BASIC line, loading it from a generated `.P` tape and
running it. Other ROMs and programs have not been comprehensively tested.
For example, Rocket Man currently loads but displays a blank screen in the
automated check; its compatibility remains unresolved.

With a legally obtained original ROM, run the opt-in boot and load checks:

```sh
ZX81_ROM=/path/to/zx81.rom cargo test original_rom -- --ignored
```

To also exercise a specific `.P` file through automatic loading, starting and
display:

```sh
ZX81_ROM=/path/to/zx81.rom ZX81_PROGRAM=/path/to/program.p cargo test --release supplied_program_loads_and_runs -- --ignored
```

Linux file drops use a locally patched Miniquad 0.4.11, with licenses in
`vendor/miniquad/LICENSE-MIT` and `vendor/miniquad/LICENSE-APACHE`. X11 file
drops have been tested; Wayland file-manager drops have not yet been verified.

## Assets

The TV artwork uses matching 2048 × 2048 transparent layers on a shared canvas.
The front-facing studio CRT cabinet has a transparent 4:3 screen opening, with
the screen position set by the 4096-based calibration coordinates in
`src/config.rs`.
The `assets/red_grid.png` background is fitted without stretching, leaving dark
margins when the window is not 4:3. The title artwork in
`assets/title.png` uses the lettering and colours from the project banner;
simple text is shown if the artwork cannot be loaded.
The screen opening is exactly 4:3 and scales uniformly with the window.
The files loaded from `assets/bezel/` are `tv_cabinet.png`, `tv_shadow.png`,
`crt_glass.png`, `crt_reflections.png`, and `crt_shadow_mask.png`. Replacement
images must be square, transparent full-canvas layers matching the bundled
alignment. Non-square layers are rejected with a warning and use the
same fallback as missing artwork.

The application and Linux launchers use `packaging/emulator-81-icon.png`;
the Windows installer uses `packaging/emulator-81.ico` for its shortcut.

### Rendering and calibration

The back-to-front order is configured by `RENDER_ORDER` in `src/config.rs`:

1. TV shadow
2. TV cabinet
3. ZX81 viewport (including optional scanlines and curvature shading)
4. CRT shadow mask
5. CRT glass
6. CRT reflections
Edit that array and rebuild to change the order. The viewport and effects should
remain above the cabinet.
`SCREEN_CALIBRATION` in `src/config.rs` defines
`tv_screen_x`, `tv_screen_y`, `tv_screen_width`, and `tv_screen_height` in the
original 4096 × 4096 artwork coordinates. The display is centered and fitted to
4:3 inside the calibrated opening. Optional scanlines use a subtle 240-line
pattern that fades at small display sizes to reduce aliasing. CRT image layers
are clipped to the viewport even if
replacement artwork contains pixels elsewhere.

`TV_CANVAS` and `SCENE_BOUNDS` in `src/bezel.rs` define
the artwork coordinates and the TV area used for fitting. F12 reports its
transformed position, scale and viewport rectangle.

Assets are looked up in `ZX81_ASSETS_DIR` if set, otherwise in an `assets`
directory in the working directory, beside the executable, or in this Cargo
project directory. Set `ZX81_ASSETS_DIR` to an empty directory to try the
procedural fallback. Missing or invalid image assets are reported on stderr;
the application still draws an operable fallback. Apart from the optional tape
loading sound, the emulator does not currently emulate sound.

Run unit tests with `cargo test` and static checks with
`cargo clippy --all-targets -- -D warnings`.
