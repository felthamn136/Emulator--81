use std::io;
use std::path::Path;

use macroquad::prelude::{draw_rectangle, Color, Rect, BLACK};

use crate::{
    experience::ScreenState,
    memory::{Memory, RamSize},
    tape::Tape,
    video,
    viewport::{self, ScreenFrame},
};

pub const SCREEN_WIDTH: usize = 256;
pub const SCREEN_HEIGHT: usize = 192;
pub const CPU_HZ: u32 = 3_250_000;

pub(crate) const LINE_T_STATES: u32 = 207;
const HSYNC_START: u32 = 16;
const HSYNC_END: u32 = 32;

/// Front-end interface for the mock display and experimental ZX81 machine.
#[derive(Default)]
pub struct EmulatorCore {
    frames: u64,
    display_buffer: Option<Vec<u8>>,
    machine: Option<CpuMachine>,
    ram_size: RamSize,
}

struct CpuMachine {
    cpu: z80::Z80,
    memory: Memory,
    line_cycles: u32,
    cycles: u64,
    tape: Tape,
    last_display_fetch: Option<u64>,
}

#[derive(Clone, Copy, Debug)]
pub struct CpuSnapshot {
    pub pc: u16,
    pub sp: u16,
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub ix: u16,
    pub iy: u16,
    pub opcode: u8,
    pub cycles: u64,
}

impl EmulatorCore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load externally supplied ROM bytes into the experimental CPU/memory core.
    pub fn load_rom(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let bytes = std::fs::read(path)?;
        self.load_rom_bytes(&bytes)
    }

    pub fn load_rom_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        let memory = Memory::with_ram_size(bytes, self.ram_size)?;
        let mut cpu = z80::Z80::new();
        cpu.init();
        cpu.irq_data = 0xff;
        self.machine = Some(CpuMachine {
            cpu,
            memory,
            line_cycles: 0,
            cycles: 0,
            tape: Tape::default(),
            last_display_fetch: None,
        });
        self.display_buffer = Some(vec![255; SCREEN_WIDTH * SCREEN_HEIGHT]);
        Ok(())
    }

    pub fn set_ram_size(&mut self, ram_size: RamSize) {
        if self.ram_size != ram_size {
            self.ram_size = ram_size;
            if let Some(machine) = &mut self.machine {
                machine.memory.set_ram_size(ram_size);
            }
        }
    }

    pub(crate) fn ram_end(&self) -> usize {
        0x4000 + self.ram_size.bytes()
    }

    pub(crate) fn supports_instant_load(&self) -> bool {
        let Some(machine) = &self.machine else {
            return false;
        };
        let bundled = include_bytes!("../rom/zx81.rom");
        (0x0340..0x03a2).all(|address| machine.memory.read8(address) == bundled[address as usize])
    }

    pub(crate) fn tape_elapsed(&self) -> Option<u64> {
        self.machine.as_ref().map(|machine| machine.tape.elapsed())
    }

    pub(crate) fn tape_audio_samples(&self, start: u64, sample_rate: u32) -> Vec<f32> {
        self.machine.as_ref().map_or_else(Vec::new, |machine| {
            machine.tape.audio_samples(start, sample_rate)
        })
    }

    pub fn is_loaded(&self) -> bool {
        self.machine.is_some()
    }

    pub fn display_signal_active(&self) -> bool {
        self.machine.as_ref().is_some_and(|machine| {
            machine
                .last_display_fetch
                .is_some_and(|last| machine.cycles.saturating_sub(last) <= 100_000)
        })
    }

    pub(crate) fn display_border_luminance(&self) -> f32 {
        if self.is_loaded() && !self.display_signal_active() {
            return 0.0;
        }
        let Some(pixels) = &self.display_buffer else {
            return 0.0;
        };
        let top = &pixels[..SCREEN_WIDTH];
        let bottom = &pixels[(SCREEN_HEIGHT - 1) * SCREEN_WIDTH..];
        let horizontal: u32 = top
            .iter()
            .chain(bottom)
            .map(|&value| u32::from(value))
            .sum();
        let vertical: u32 = pixels
            .chunks_exact(SCREEN_WIDTH)
            .skip(1)
            .take(SCREEN_HEIGHT - 2)
            .map(|row| u32::from(row[0]) + u32::from(row[SCREEN_WIDTH - 1]))
            .sum();
        let count = 2 * SCREEN_WIDTH + 2 * (SCREEN_HEIGHT - 2);
        (horizontal + vertical) as f32 / (count as f32 * 255.0)
    }

    pub fn cpu_snapshot(&self) -> Option<CpuSnapshot> {
        self.machine.as_ref().map(|machine| CpuSnapshot {
            pc: machine.cpu.pc,
            sp: machine.cpu.sp,
            af: machine.cpu.af(),
            bc: machine.cpu.bc(),
            de: machine.cpu.de(),
            hl: machine.cpu.hl(),
            ix: machine.cpu.ix,
            iy: machine.cpu.iy,
            opcode: machine.memory.read8(machine.cpu.pc),
            cycles: machine.cycles,
        })
    }

    /// Execute an instruction and advance the instruction-boundary PAL timer.
    pub fn step_instruction(&mut self) -> io::Result<u32> {
        match &mut self.machine {
            Some(machine) => {
                machine.memory.set_tape_input(machine.tape.level());
                if machine.cpu.pc == 0x034c {
                    let return_address = u16::from_le_bytes([
                        machine.memory.read8(machine.cpu.sp),
                        machine.memory.read8(machine.cpu.sp.wrapping_add(1)),
                    ]);
                    // Let the ROM's initial tape synchronization time out normally.
                    if matches!(return_address, 0x0369 | 0x037f) {
                        if let Some(byte) = machine.tape.take_instant_byte() {
                            let flags = (byte & 0xa8)
                                | if byte == 0 { 0x40 } else { 0 }
                                | if byte.count_ones() % 2 == 0 { 0x04 } else { 0 }
                                | 0x01;
                            machine.cpu.set_bc(u16::from(byte));
                            machine
                                .cpu
                                .set_af((machine.cpu.af() & 0xff00) | u16::from(flags));
                            machine.cpu.pc = 0x03a1;
                        }
                    }
                }
                let was_halted = machine.cpu.halted;
                let start = machine.line_cycles;
                let mut cycles = machine.cpu.step(&mut machine.memory);
                let end = start + cycles;
                let crossed_hsync = (start < HSYNC_START && end >= HSYNC_START)
                    || (end >= LINE_T_STATES + HSYNC_START);
                machine.line_cycles = end % LINE_T_STATES;
                if crossed_hsync && machine.memory.nmi_enabled() {
                    // A HALTed CPU resumes its NMI acknowledge at the end of
                    // HSYNC: WAIT holds the acknowledge after its first two T-states.
                    if was_halted && machine.cpu.halted {
                        let phase = machine.line_cycles;
                        if (HSYNC_START..HSYNC_END).contains(&phase) {
                            let wait = HSYNC_END.saturating_sub(phase + 2);
                            cycles += wait;
                            machine.line_cycles += wait;
                        }
                    }
                    machine.cpu.pulse_nmi();
                    let interrupt_cycles = machine.cpu.service_interrupts(&mut machine.memory);
                    cycles += interrupt_cycles;
                    machine.line_cycles = (machine.line_cycles + interrupt_cycles) % LINE_T_STATES;
                }
                machine.cycles = machine.cycles.saturating_add(u64::from(cycles));
                if self.ram_size == RamSize::SixteenK {
                    machine.memory.ram_pack_mut().update(cycles);
                }
                if machine.memory.take_display_fetch() {
                    machine.last_display_fetch = Some(machine.cycles);
                }
                machine.tape.advance(cycles);
                Ok(cycles)
            }
            None => Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no ZX81 ROM loaded",
            )),
        }
    }

    pub fn set_key(&mut self, row: usize, column: usize, pressed: bool) -> io::Result<()> {
        let machine = self
            .machine
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no ZX81 ROM loaded"))?;
        machine.memory.set_key(row, column, pressed)
    }

    pub fn ram_pack(&self) -> Option<&crate::hardware::ram_pack::RamPack> {
        self.machine
            .as_ref()
            .map(|machine| machine.memory.ram_pack())
    }

    pub fn ram_pack_mut(&mut self) -> Option<&mut crate::hardware::ram_pack::RamPack> {
        self.machine
            .as_mut()
            .map(|machine| machine.memory.ram_pack_mut())
    }

    /// Run at least the requested number of T-states, ending at an instruction boundary.
    pub fn run_cycles(&mut self, budget: u32) -> io::Result<u32> {
        self.run_cycles_inner(budget, false)
    }

    pub(crate) fn run_cycles_until_tape_stops(&mut self, budget: u32) -> io::Result<u32> {
        self.run_cycles_inner(budget, true)
    }

    fn run_cycles_inner(&mut self, budget: u32, stop_at_tape_end: bool) -> io::Result<u32> {
        if self.machine.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no ZX81 ROM loaded",
            ));
        }
        let mut elapsed = 0u32;
        while elapsed < budget {
            let cycles = self.step_instruction()?;
            if cycles == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Z80 instruction returned zero T-states",
                ));
            }
            elapsed = elapsed.saturating_add(cycles);
            if stop_at_tape_end && !self.tape_state().1 {
                break;
            }
        }
        Ok(elapsed)
    }

    /// Insert a .P tape; the ROM performs the actual load from the tape signal.
    pub fn load_program(&mut self, path: impl AsRef<Path>) -> io::Result<()> {
        let path = path.as_ref();
        if self.machine.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no ZX81 ROM loaded",
            ));
        }
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("p"))
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "only .P tapes are supported",
            ));
        }
        let bytes = std::fs::read(path)?;
        self.load_program_bytes(&bytes)
    }

    pub(crate) fn load_program_bytes(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.insert_tape(Tape::from_p(bytes)?)
    }

    /// Export the current BASIC program and system variables as a ZX81 .P file.
    pub fn save_program_bytes(&self) -> io::Result<Vec<u8>> {
        if self.machine.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no ZX81 ROM loaded",
            ));
        }
        if !self.basic_cursor_visible() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "return to the BASIC K prompt before saving",
            ));
        }
        self.program_image_bytes()
    }

    /// Capture the current ZX81 SAVE memory range, including system variables.
    pub fn program_image_bytes(&self) -> io::Result<Vec<u8>> {
        let machine = self
            .machine
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no ZX81 ROM loaded"))?;
        let memory = &machine.memory;
        let end = usize::from(u16::from_le_bytes([
            memory.read8(0x4014),
            memory.read8(0x4015),
        ]));
        let display = usize::from(u16::from_le_bytes([
            memory.read8(0x400c),
            memory.read8(0x400d),
        ]));
        if !(0x407d..=self.ram_end()).contains(&end) || !(0x407d..end).contains(&display) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid BASIC program or display-file pointers",
            ));
        }
        let mut bytes: Vec<u8> = (0x4009..end)
            .map(|address| memory.read8(address as u16))
            .collect();
        // The ROM clears the running SLOW display flag before saving.
        bytes[0x403b - 0x4009] &= 0x7f;
        Ok(bytes)
    }

    pub(crate) fn insert_tape(&mut self, tape: Tape) -> io::Result<()> {
        let machine = self
            .machine
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no ZX81 ROM loaded"))?;
        machine.tape = tape;
        Ok(())
    }

    pub(crate) fn basic_cursor_visible(&self) -> bool {
        let Some(machine) = &self.machine else {
            return false;
        };
        if !self.display_signal_active() || video::render_display_file(&machine.memory).is_err() {
            return false;
        }
        let start =
            u16::from_le_bytes([machine.memory.read8(0x400c), machine.memory.read8(0x400d)]);
        let mut rows = 0;
        for address in usize::from(start) + 1..0x8000 {
            match machine.memory.read8(address as u16) {
                0xb0 => return true,
                0x76 => {
                    rows += 1;
                    if rows == 24 {
                        break;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn basic_input_matches(&self, input: &[u8]) -> bool {
        let Some(machine) = &self.machine else {
            return false;
        };
        let memory = &machine.memory;
        let start = usize::from(u16::from_le_bytes([
            memory.read8(0x4014),
            memory.read8(0x4015),
        ]));
        if start < 0x4000 || start + input.len() > self.ram_end() {
            return false;
        }
        input
            .iter()
            .enumerate()
            .all(|(offset, &byte)| memory.read8((start + offset) as u16) == byte)
    }

    /// The ROM's successful LOAD report must be dismissed before typing RUN.
    pub(crate) fn basic_load_report_visible(&self) -> bool {
        let Some(machine) = &self.machine else {
            return false;
        };
        let memory = &machine.memory;
        if !self.display_signal_active() || video::render_display_file(memory).is_err() {
            return false;
        }
        let mut address = usize::from(u16::from_le_bytes([
            memory.read8(0x400c),
            memory.read8(0x400d),
        ])) + 1;
        for _ in 0..23 {
            while address < 0x8000 && memory.read8(address as u16) != 0x76 {
                address += 1;
            }
            if address >= 0x8000 {
                return false;
            }
            address += 1;
        }
        if address + 2 >= 0x8000 {
            return false;
        }
        if [
            memory.read8(address as u16),
            memory.read8((address + 1) as u16),
            memory.read8((address + 2) as u16),
        ] != [0x1c, 0x18, 0x1c]
        {
            return false;
        }
        address += 3;
        while address < 0x8000 && memory.read8(address as u16) == 0x00 {
            address += 1;
        }
        address < 0x8000 && memory.read8(address as u16) == 0x76
    }

    pub(crate) fn program_matches(&self, bytes: &[u8], display_file: u16) -> bool {
        let Some(machine) = &self.machine else {
            return false;
        };
        let current_display =
            u16::from_le_bytes([machine.memory.read8(0x400c), machine.memory.read8(0x400d)]);
        current_display == display_file
            && usize::from(display_file).saturating_sub(0x4009) <= bytes.len()
            && (0x407d..display_file).all(|address| {
                machine.memory.read8(address) == bytes[usize::from(address) - 0x4009]
            })
    }

    pub fn toggle_tape(&mut self) -> io::Result<()> {
        let machine = self
            .machine
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no ZX81 ROM loaded"))?;
        machine.tape.toggle()
    }

    pub fn tape_state(&self) -> (bool, bool) {
        self.machine
            .as_ref()
            .map_or((false, false), |m| (m.tape.inserted(), m.tape.playing()))
    }

    pub(crate) fn tape(&self) -> Option<&Tape> {
        self.machine.as_ref().map(|machine| &machine.tape)
    }

    pub fn reset(&mut self) {
        self.frames = 0;
        self.display_buffer = self
            .machine
            .as_ref()
            .map(|_| vec![255; SCREEN_WIDTH * SCREEN_HEIGHT]);
        if let Some(machine) = &mut self.machine {
            machine.cpu.init();
            machine.cpu.irq_data = 0xff;
            machine.memory.clear_ram();
            machine.memory.reset_hardware();
            machine.line_cycles = 0;
            machine.cycles = 0;
            machine.last_display_fetch = None;
            machine.tape.stop();
        }
    }

    /// Replace the mock display with 256x192 row-major monochrome luminance bytes.
    pub fn write_display_buffer(&mut self, pixels: &[u8]) -> io::Result<()> {
        if pixels.len() != SCREEN_WIDTH * SCREEN_HEIGHT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "display buffer must contain exactly 256x192 luminance bytes",
            ));
        }
        self.display_buffer = Some(pixels.to_vec());
        Ok(())
    }

    pub fn clear_display_buffer(&mut self) {
        self.display_buffer = None;
    }

    /// Decode the machine's display file into the front-end's monochrome buffer.
    pub fn refresh_display(&mut self) -> io::Result<()> {
        let machine = self
            .machine
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no ZX81 ROM loaded"))?;
        self.display_buffer = Some(video::render_display_file(&machine.memory)?);
        Ok(())
    }

    /// Advance the mock frame counter; this does not execute machine instructions.
    pub fn run_frame(&mut self) {
        self.frames = self.frames.saturating_add(1);
    }

    /// Draw the mock display until a core supplies real video data.
    pub fn render_screen(&self, bounds: Rect, frame: &ScreenFrame<'_>) {
        if self.is_loaded()
            && !self.display_signal_active()
            && !matches!(
                frame.state,
                ScreenState::PowerOff | ScreenState::Booting | ScreenState::Shutdown
            )
        {
            draw_rectangle(bounds.x, bounds.y, bounds.w, bounds.h, BLACK);
            return;
        }
        if let Some(pixels) = &self.display_buffer {
            if !matches!(
                frame.state,
                ScreenState::PowerOff | ScreenState::Booting | ScreenState::Shutdown
            ) {
                draw_rectangle(bounds.x, bounds.y, bounds.w, bounds.h, BLACK);
                let pw = bounds.w / SCREEN_WIDTH as f32;
                let ph = bounds.h / SCREEN_HEIGHT as f32;
                for (y, row) in pixels.as_chunks::<SCREEN_WIDTH>().0.iter().enumerate() {
                    let mut x = 0;
                    while x < SCREEN_WIDTH {
                        let value = row[x];
                        let start = x;
                        while x < SCREEN_WIDTH && row[x] == value {
                            x += 1;
                        }
                        if value != 0 {
                            let shade = value as f32 / 255.0 * frame.brightness.clamp(0.0, 1.0);
                            draw_rectangle(
                                bounds.x + start as f32 * pw,
                                bounds.y + y as f32 * ph,
                                (x - start) as f32 * pw,
                                ph,
                                Color::new(shade, shade, shade, 1.0),
                            );
                        }
                    }
                }
                return;
            }
        }
        viewport::draw_viewport(bounds, frame);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_luminance_follows_display_edges_and_blank_signal() {
        let mut core = EmulatorCore::new();
        assert_eq!(core.display_border_luminance(), 0.0);
        let mut pixels = vec![255; SCREEN_WIDTH * SCREEN_HEIGHT];
        for row in pixels.chunks_exact_mut(SCREEN_WIDTH) {
            row[SCREEN_WIDTH - 1] = 0;
        }
        core.write_display_buffer(&pixels).unwrap();
        assert!((0.7..0.9).contains(&core.display_border_luminance()));
        pixels.fill(0);
        core.write_display_buffer(&pixels).unwrap();
        assert_eq!(core.display_border_luminance(), 0.0);
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        assert_eq!(core.display_border_luminance(), 0.0);
        core.machine.as_mut().unwrap().last_display_fetch = Some(0);
        assert_eq!(core.display_border_luminance(), 1.0);
    }

    #[test]
    fn recognizes_only_a_complete_success_report_on_the_last_display_row() {
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        let machine = core.machine.as_mut().unwrap();
        machine.last_display_fetch = Some(0);
        machine.memory.write8(0x400c, 0x00);
        machine.memory.write8(0x400d, 0x41);
        for address in 0x4100..0x4118 {
            machine.memory.write8(address, 0x76);
        }
        for (offset, byte) in [0x1c, 0x18, 0x1c, 0x00, 0x76].iter().enumerate() {
            machine.memory.write8(0x4118 + offset as u16, *byte);
        }
        assert!(core.basic_load_report_visible());

        core.machine.as_mut().unwrap().memory.write8(0x411a, 0x1d);
        assert!(!core.basic_load_report_visible());
        core.machine.as_mut().unwrap().memory.write8(0x411a, 0x1c);
        core.machine.as_mut().unwrap().memory.write8(0x411b, 0x1d);
        assert!(!core.basic_load_report_visible());
        core.machine.as_mut().unwrap().memory.write8(0x411b, 0x00);
        core.machine.as_mut().unwrap().last_display_fetch = None;
        assert!(!core.basic_load_report_visible());

        let machine = core.machine.as_mut().unwrap();
        machine.last_display_fetch = Some(0);
        for (offset, byte) in [0x1c, 0x18, 0x1c].iter().enumerate() {
            machine.memory.write8(0x4101 + offset as u16, *byte);
        }
        for address in 0x4104..=0x411b {
            machine.memory.write8(address, 0x76);
        }
        assert!(!core.basic_load_report_visible());
    }

    #[test]
    fn core_reports_unsupported_programs_and_resets() {
        let mut core = EmulatorCore::new();
        assert_eq!(
            core.step_instruction().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            core.load_program("missing.p").unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        core.run_frame();
        assert_eq!(core.frames, 1);
        assert_eq!(
            core.write_display_buffer(&[1, 2]).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        core.write_display_buffer(&vec![255; SCREEN_WIDTH * SCREEN_HEIGHT])
            .unwrap();
        assert!(core.display_buffer.is_some());
        core.reset();
        assert_eq!(core.frames, 0);
        assert!(core.display_buffer.is_none());
    }

    #[test]
    fn selected_ram_size_is_used_for_loaded_and_future_machines() {
        let mut core = EmulatorCore::new();
        core.set_ram_size(RamSize::OneK);
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        let machine = core.machine.as_mut().unwrap();
        machine.memory.write8(0x4000, 0x12);
        assert_eq!(machine.memory.read8(0x4400), 0x12);

        core.set_ram_size(RamSize::SixteenK);
        let machine = core.machine.as_mut().unwrap();
        machine.memory.write8(0x4000, 0x34);
        assert_eq!(machine.memory.read8(0x4400), 0);

        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        let machine = core.machine.as_mut().unwrap();
        machine.memory.write8(0x4000, 0x56);
        assert_eq!(machine.memory.read8(0x4400), 0);
    }

    #[test]
    fn synthetic_rom_executes_and_reset_restores_cpu_and_ram() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..6].copy_from_slice(&[0x3e, 0x42, 0x32, 0x00, 0x40, 0x76]);
        core.load_rom_bytes(&rom).unwrap();
        assert_eq!(core.step_instruction().unwrap(), 7);
        assert_eq!(core.step_instruction().unwrap(), 13);
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0x42);
        assert_eq!(core.step_instruction().unwrap(), 4);
        assert!(core.machine.as_ref().unwrap().cpu.halted);
        assert!(core.load_rom_bytes(&rom[..100]).is_err());
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0x42);
        core.reset();
        assert!(!core.machine.as_ref().unwrap().cpu.halted);
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0);
        assert_eq!(core.step_instruction().unwrap(), 7);
    }

    #[test]
    fn arithmetic_prefixes_and_exchange_preserve_expected_flags_and_timing() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..14].copy_from_slice(&[
            0x3e, 0x7f, 0xc6, 0x01, 0xce, 0x7f, 0xd6, 0xff, 0xed, 0x44, 0xcb, 0x07, 0x08, 0x08,
        ]);
        core.load_rom_bytes(&rom).unwrap();
        for (cycles, af) in [
            (7, 0x7fff),
            (7, 0x8094),
            (7, 0xffa8),
            (7, 0x0042),
            (8, 0x0042),
            (8, 0x0044),
            (4, 0x0000),
            (4, 0x0044),
        ] {
            assert_eq!(core.step_instruction().unwrap(), cycles);
            assert_eq!(core.cpu_snapshot().unwrap().af, af);
        }
    }

    #[test]
    fn synthetic_rom_reads_keyboard_through_cpu_port_input() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..9].copy_from_slice(&[0x01, 0xfe, 0xfd, 0xed, 0x78, 0x32, 0x00, 0x40, 0x76]);
        core.load_rom_bytes(&rom).unwrap();
        core.set_key(1, 2, true).unwrap();
        for _ in 0..3 {
            core.step_instruction().unwrap();
        }
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0x7b);
    }

    #[test]
    fn bundled_rom_accepts_shift_period_as_comma() {
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
            .unwrap();
        core.run_cycles(CPU_HZ * 15).unwrap();
        core.set_key(5, 0, true).unwrap();
        core.run_cycles(CPU_HZ / 5).unwrap();
        core.set_key(5, 0, false).unwrap();
        core.run_cycles(CPU_HZ / 5).unwrap();
        core.set_key(0, 0, true).unwrap();
        core.run_cycles(CPU_HZ / 5).unwrap();
        core.set_key(7, 1, true).unwrap();
        core.run_cycles(CPU_HZ / 5).unwrap();
        core.set_key(0, 0, false).unwrap();
        core.set_key(7, 1, false).unwrap();
        core.run_cycles(CPU_HZ / 5).unwrap();
        let memory = &core.machine.as_ref().unwrap().memory;
        let start = u16::from_le_bytes([memory.read8(0x4014), memory.read8(0x4015)]);
        assert_eq!(memory.read8(start), 0xf5); // PRINT
        assert_eq!(memory.read8(start + 1), 0x1a); // comma
    }

    #[test]
    fn bundled_rom_autoloads_a_one_k_basic_program() {
        let mut core = EmulatorCore::new();
        core.set_ram_size(RamSize::OneK);
        core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
            .unwrap();
        core.run_cycles(CPU_HZ * 15).unwrap();
        for key in [(3, 0), (4, 0), (5, 0), (3, 0), (6, 0)] {
            core.set_key(key.0, key.1, true).unwrap();
            core.run_cycles(CPU_HZ / 5).unwrap();
            core.set_key(key.0, key.1, false).unwrap();
            core.run_cycles(CPU_HZ / 5).unwrap();
        }
        core.run_cycles(CPU_HZ).unwrap();
        let program = core.save_program_bytes().unwrap();
        assert_eq!(&program[0x407d - 0x4009..0x407f - 0x4009], &[0, 10]);
        let display_file = u16::from_le_bytes([program[3], program[4]]);
        for mode in [
            crate::autoload::LoadMode::Standard,
            crate::autoload::LoadMode::Instant,
        ] {
            let mut loader =
                crate::autoload::AutoLoad::from_bytes_with_mode(&mut core, program.clone(), mode)
                    .unwrap();
            let mut saw_load_command = false;
            let mut completed = false;
            for _ in 0..5000 {
                let cycles = if loader.is_playing() {
                    core.run_cycles_until_tape_stops(CPU_HZ / 10).unwrap()
                } else {
                    core.run_cycles(CPU_HZ / 60).unwrap()
                };
                saw_load_command |= core.basic_input_matches(&[0xef, 0x0b, 0x0b]);
                if loader.advance(&mut core, cycles).unwrap() {
                    completed = true;
                    break;
                }
            }
            assert!(
                saw_load_command,
                "LOAD command was not entered for {mode:?}"
            );
            assert!(
                completed,
                "automatic LOAD and RUN did not complete for {mode:?}"
            );
            assert!(core.program_matches(&program, display_file));

            core.run_cycles(CPU_HZ * 3).unwrap();
            for key in [(6, 2), (6, 0)] {
                core.set_key(key.0, key.1, true).unwrap();
                core.run_cycles(CPU_HZ / 5).unwrap();
                core.set_key(key.0, key.1, false).unwrap();
                core.run_cycles(CPU_HZ / 5).unwrap();
            }
            core.run_cycles(CPU_HZ).unwrap();
            let memory = &core.machine.as_ref().unwrap().memory;
            let display = u16::from_le_bytes([memory.read8(0x400c), memory.read8(0x400d)]);
            let screen: Vec<u8> = (0..0x100)
                .map(|offset| memory.read8(display + offset))
                .collect();
            assert!(
                screen
                    .windows(8)
                    .any(|line| line == [0x1d, 0x1c, 0x00, 0x35, 0x37, 0x2e, 0x33, 0x39]),
                "LIST did not display line 10 PRINT for {mode:?}: {screen:02x?}"
            );
        }
    }

    #[test]
    fn bundled_rom_autoloads_the_one_k_chess_tape() {
        let mut core = EmulatorCore::new();
        core.set_ram_size(RamSize::OneK);
        core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
            .unwrap();
        let bytes = include_bytes!("../Games Library/1KZXChess.p").to_vec();
        let mut loader = crate::autoload::AutoLoad::from_bytes(&mut core, bytes).unwrap();
        for _ in 0..5000 {
            let cycles = if loader.is_playing() {
                core.run_cycles_until_tape_stops(CPU_HZ / 10).unwrap()
            } else {
                core.run_cycles(CPU_HZ / 60).unwrap()
            };
            if loader.advance(&mut core, cycles).unwrap() {
                return;
            }
        }
        panic!("1K chess tape did not finish loading");
    }

    #[test]
    fn instant_load_uses_rom_for_one_k_and_sixteen_k_programs() {
        use crate::autoload::{AutoLoad, LoadMode};

        for (ram, program) in [
            (
                RamSize::OneK,
                include_bytes!("../Games Library/1KZXChess.p").as_slice(),
            ),
            (
                RamSize::SixteenK,
                include_bytes!("../Games Library/3D MONSTER MAZE.p").as_slice(),
            ),
        ] {
            let mut core = EmulatorCore::new();
            core.set_ram_size(ram);
            core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
                .unwrap();
            let mut loader =
                AutoLoad::from_bytes_with_mode(&mut core, program.to_vec(), LoadMode::Instant)
                    .unwrap();
            let display_file = u16::from_le_bytes([program[3], program[4]]);
            let mut complete = false;
            let mut tape_cycles = 0u64;
            for _ in 0..2000 {
                let cycles = if loader.is_playing() {
                    core.run_cycles_until_tape_stops(CPU_HZ / 60).unwrap()
                } else {
                    core.run_cycles(CPU_HZ / 60).unwrap()
                };
                if loader.is_playing() {
                    tape_cycles += u64::from(cycles);
                }
                if loader.advance(&mut core, cycles).unwrap_or_else(|error| {
                    let machine = core.machine.as_ref().unwrap();
                    let mismatch = (0x407d..display_file).find(|&address| {
                        machine.memory.read8(address) != program[usize::from(address) - 0x4009]
                    });
                    panic!(
                        "{ram:?}: {error}, pc={:04x}, mismatch={mismatch:?}, tape={:?}",
                        machine.cpu.pc,
                        core.tape_state()
                    )
                }) {
                    complete = true;
                    break;
                }
            }
            assert!(complete, "instant load did not complete for {ram:?}");
            assert!(tape_cycles < u64::from(CPU_HZ), "tape was not skipped");
            assert!(core.program_matches(program, display_file));
            if ram == RamSize::SixteenK {
                assert!(!core.basic_cursor_visible(), "self-start did not run");
            }
        }
    }

    #[test]
    fn wobble_affects_loaded_basic_program() {
        use crate::autoload::{AutoLoad, LoadMode};
        use crate::hardware::ram_pack::{RamPack, WobbleFault, WobbleSeverity};

        let mut core = EmulatorCore::new();
        core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
            .unwrap();
        *core.ram_pack_mut().unwrap() = RamPack::with_seed(12345);
        let program = include_bytes!("../Games Library/3D MONSTER MAZE.p");
        let mut loader =
            AutoLoad::from_bytes_with_mode(&mut core, program.to_vec(), LoadMode::Instant).unwrap();
        let mut loaded = false;
        for _ in 0..2000 {
            let cycles = if loader.is_playing() {
                core.run_cycles_until_tape_stops(CPU_HZ / 60).unwrap()
            } else {
                core.run_cycles(CPU_HZ / 60).unwrap()
            };
            if loader.advance(&mut core, cycles).unwrap() {
                loaded = true;
                break;
            }
        }
        assert!(loaded);
        let display = u16::from_le_bytes([program[3], program[4]]);
        assert!(core.program_matches(program, display));
        core.ram_pack_mut()
            .unwrap()
            .set_severity(WobbleSeverity::Extreme);
        let mut corrupted = false;
        for _ in 0..100 {
            let pack = core.ram_pack_mut().unwrap();
            pack.set_enabled(true);
            pack.trigger_fault(WobbleFault::AddressGlitch);
            core.run_cycles(1_000).unwrap();
            core.ram_pack_mut().unwrap().set_enabled(false);
            let memory = &core.machine.as_ref().unwrap().memory;
            if (0x407d..display)
                .any(|address| memory.read8(address) != program[usize::from(address) - 0x4009])
            {
                corrupted = true;
                break;
            }
        }
        assert!(
            corrupted,
            "address glitches did not corrupt the loaded program"
        );
    }

    #[test]
    fn instant_load_rejects_an_unsupported_rom_before_reset() {
        use crate::autoload::{AutoLoad, LoadMode};

        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        core.run_cycles(1000).unwrap();
        let before = core.cpu_snapshot().unwrap().cycles;
        let program = include_bytes!("../Games Library/1KZXChess.p").to_vec();
        assert_eq!(
            AutoLoad::from_bytes_with_mode(&mut core, program, LoadMode::Instant)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(core.cpu_snapshot().unwrap().cycles, before);
    }

    #[test]
    fn refresh_display_requires_a_valid_initialized_display_file() {
        let mut core = EmulatorCore::new();
        assert_eq!(
            core.refresh_display().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        assert!(core.refresh_display().is_err());
        let memory = &mut core.machine.as_mut().unwrap().memory;
        memory.write8(0x400c, 0x00);
        memory.write8(0x400d, 0x41);
        for address in 0x4100..0x4119 {
            memory.write8(address, 0x76);
        }
        core.refresh_display().unwrap();
        assert_eq!(
            core.display_buffer.as_ref().unwrap().len(),
            SCREEN_WIDTH * SCREEN_HEIGHT
        );
        core.machine.as_mut().unwrap().memory.write8(0x4100, 0);
        assert!(core.refresh_display().is_err());
        assert!(core.display_buffer.is_some());
    }

    #[test]
    fn cpu_fetches_display_characters_as_nops_but_reads_data_unchanged() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..9].copy_from_slice(&[0x3a, 0x00, 0xc0, 0x32, 0x02, 0x40, 0xc3, 0x00, 0xc0]);
        core.load_rom_bytes(&rom).unwrap();
        let memory = &mut core.machine.as_mut().unwrap().memory;
        memory.write8(0x4000, 0x3e);
        memory.write8(0x4001, 0x76);
        core.step_instruction().unwrap();
        core.step_instruction().unwrap();
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4002), 0x3e);
        core.step_instruction().unwrap();
        assert_eq!(core.step_instruction().unwrap(), 4);
        assert_eq!(core.machine.as_ref().unwrap().cpu.pc, 0xc001);
        core.step_instruction().unwrap();
        assert!(core.machine.as_ref().unwrap().cpu.halted);
    }

    #[test]
    fn display_signal_expires_without_display_fetches_and_recovers() {
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        assert!(!core.display_signal_active());
        core.machine.as_mut().unwrap().cpu.pc = 0xc000;
        core.step_instruction().unwrap();
        assert!(core.display_signal_active());
        core.machine.as_mut().unwrap().cpu.pc = 0;
        core.run_cycles(100_004).unwrap();
        assert!(!core.display_signal_active());
        core.machine.as_mut().unwrap().cpu.pc = 0xc000;
        core.step_instruction().unwrap();
        assert!(core.display_signal_active());
        core.reset();
        assert!(!core.display_signal_active());
    }

    #[test]
    fn prefix_opcode_fetches_also_pass_through_the_ula() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..5].copy_from_slice(&[0x3e, 0xff, 0xc3, 0x00, 0xc0]);
        core.load_rom_bytes(&rom).unwrap();
        let memory = &mut core.machine.as_mut().unwrap().memory;
        memory.write8(0x4000, 0xcb);
        memory.write8(0x4001, 0x3f);
        core.step_instruction().unwrap();
        core.step_instruction().unwrap();
        assert_eq!(core.step_instruction().unwrap(), 8);
        assert_eq!(core.machine.as_ref().unwrap().cpu.af() >> 8, 0xff);
    }

    #[test]
    fn refresh_interrupt_releases_halt_in_interrupt_mode_one() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..4].copy_from_slice(&[0xed, 0x56, 0xfb, 0x76]);
        core.load_rom_bytes(&rom).unwrap();
        assert_eq!(core.step_instruction().unwrap(), 8);
        assert_eq!(core.step_instruction().unwrap(), 4);
        assert_eq!(core.step_instruction().unwrap(), 17);
        let machine = core.machine.as_ref().unwrap();
        assert!(!machine.cpu.halted);
        assert_eq!(machine.cpu.pc, 0x38);
        assert_eq!(machine.memory.read8(0xfffd), 4);
    }

    #[test]
    fn nmi_after_ei_preserves_interrupt_enable_for_retn() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[0] = 0xfb;
        rom[0x66..0x68].copy_from_slice(&[0xed, 0x45]);
        core.load_rom_bytes(&rom).unwrap();
        core.machine.as_mut().unwrap().cpu.pulse_nmi();
        assert_eq!(core.step_instruction().unwrap(), 15);
        let cpu = &core.machine.as_ref().unwrap().cpu;
        assert!(!cpu.iff1);
        assert!(cpu.iff2);
        core.step_instruction().unwrap();
        assert!(core.machine.as_ref().unwrap().cpu.iff1);
    }

    #[test]
    fn mode_zero_interrupt_uses_floating_bus_rst_cycles() {
        let mut core = EmulatorCore::new();
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..2].copy_from_slice(&[0xfb, 0x76]);
        core.load_rom_bytes(&rom).unwrap();
        assert_eq!(core.step_instruction().unwrap(), 4);
        assert_eq!(core.step_instruction().unwrap(), 17);
        assert_eq!(core.machine.as_ref().unwrap().cpu.pc, 0x38);
    }

    #[test]
    fn nmi_generator_runs_only_when_enabled() {
        let mut core = EmulatorCore::new();
        let rom = vec![0; crate::memory::ROM_SIZE];
        core.load_rom_bytes(&rom).unwrap();
        for _ in 0..52 {
            core.step_instruction().unwrap();
        }
        assert_eq!(core.machine.as_ref().unwrap().cpu.pc, 52);
        let machine = core.machine.as_mut().unwrap();
        z80::Z80_io::port_out(&mut machine.memory, 0xfe, 0);
        for _ in 0..4 {
            core.step_instruction().unwrap();
        }
        let machine = core.machine.as_ref().unwrap();
        assert_eq!(machine.cpu.pc, 0x66);
        assert_eq!(machine.cpu.sp, 0xfffd);
        assert_eq!(machine.memory.read8(0x7ffd), 56);
        assert_eq!(machine.cycles, 235);
    }

    #[test]
    fn hsync_nmi_releases_halt_at_a_consistent_line_phase() {
        for start_phase in 12..16 {
            let mut core = EmulatorCore::new();
            core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
            let machine = core.machine.as_mut().unwrap();
            machine.line_cycles = start_phase;
            machine.cpu.halted = true;
            z80::Z80_io::port_out(&mut machine.memory, 0xfe, 0);

            let cycles = core.step_instruction().unwrap();
            let machine = core.machine.as_ref().unwrap();
            assert_eq!(machine.cpu.pc, 0x66);
            assert_eq!(machine.line_cycles, 41);
            assert_eq!(start_phase + cycles, 41);
        }
    }

    #[test]
    fn synthetic_firmware_initializes_and_renders_a_real_display_file() {
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..23].copy_from_slice(&[
            0x21, 0x00, 0x41, 0x06, 26, 0x36, 0x76, 0x23, 0x10, 0xfb, 0x21, 0x00, 0x41, 0x22, 0x0c,
            0x40, 0x3e, 0x01, 0x32, 0x01, 0x41, 0x76, 0x00,
        ]);
        rom[0x1e08] = 0x80;
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&rom).unwrap();
        assert!(core.run_cycles(5000).unwrap() >= 5000);
        core.refresh_display().unwrap();
        let pixels = core.display_buffer.as_ref().unwrap();
        assert_eq!(&pixels[..2], &[0, 255]);
        let cpu = core.cpu_snapshot().unwrap();
        assert_eq!(cpu.pc, 22);
        assert!(cpu.cycles >= 5000);
    }

    #[test]
    fn tape_signal_reaches_cpu_port_reads_and_stops_with_reset() {
        let mut rom = vec![0; crate::memory::ROM_SIZE];
        rom[..7].copy_from_slice(&[0xdb, 0xfe, 0x32, 0x00, 0x40, 0xc3, 0x00]);
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&rom).unwrap();
        let mut program = vec![0; 0x80];
        program[11..13].copy_from_slice(&0x4089u16.to_le_bytes());
        let machine = core.machine.as_mut().unwrap();
        machine.tape = Tape::from_p(&program).unwrap();
        machine.tape.toggle().unwrap();
        core.step_instruction().unwrap();
        core.step_instruction().unwrap();
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0x7f);
        let machine = core.machine.as_mut().unwrap();
        machine.tape.advance(3_250_000 * 5);
        machine.cpu.pc = 0;
        core.step_instruction().unwrap();
        core.step_instruction().unwrap();
        assert_eq!(core.machine.as_ref().unwrap().memory.read8(0x4000), 0xff);
        core.reset();
        assert_eq!(core.tape_state(), (true, false));
    }

    #[test]
    #[ignore = "requires ZX81_ROM pointing to a legally obtained original ROM"]
    fn original_rom_reaches_k_prompt() {
        let path = std::env::var_os("ZX81_ROM").expect("set ZX81_ROM to the original ROM path");
        let mut core = EmulatorCore::new();
        core.load_rom(path).unwrap();
        for _ in 0..150 {
            core.run_cycles(325_000).unwrap();
            let machine = core.machine.as_ref().unwrap();
            if video::render_display_file(&machine.memory).is_err() {
                continue;
            }
            if !core.display_signal_active() {
                continue;
            }
            let start = usize::from(u16::from_le_bytes([
                machine.memory.read8(0x400c),
                machine.memory.read8(0x400d),
            ]));
            let mut rows = 0;
            for address in start + 1..0x8000 {
                let byte = machine.memory.read8(address as u16);
                if byte == 0xb0 {
                    return;
                }
                if byte == 0x76 {
                    rows += 1;
                    if rows == 24 {
                        break;
                    }
                }
            }
        }
        panic!("ROM did not reach an inverse K cursor within 15 emulated seconds");
    }

    #[test]
    #[ignore = "requires ZX81_ROM pointing to a legally obtained original ROM"]
    fn original_rom_loads_a_p_tape() {
        let path = std::env::var_os("ZX81_ROM").expect("set ZX81_ROM to the original ROM path");
        let mut core = EmulatorCore::new();
        core.load_rom(path).unwrap();
        core.run_cycles(3_250_000 * 15).unwrap();

        fn press(core: &mut EmulatorCore, keys: &[(usize, usize)]) {
            for &(row, column) in keys {
                core.set_key(row, column, true).unwrap();
            }
            core.run_cycles(650_000).unwrap();
            for &(row, column) in keys {
                core.set_key(row, column, false).unwrap();
            }
            core.run_cycles(650_000).unwrap();
        }

        for key in [(3, 0), (4, 0), (5, 0), (3, 0), (6, 0)] {
            press(&mut core, &[key]);
        }
        core.run_cycles(3_250_000).unwrap();
        let machine = core.machine.as_ref().unwrap();
        let end = usize::from(u16::from_le_bytes([
            machine.memory.read8(0x4014),
            machine.memory.read8(0x4015),
        ]));
        assert!(end > 0x407d && end <= 0x8000);
        let program = core.save_program_bytes().unwrap();
        let display = usize::from(u16::from_le_bytes([
            machine.memory.read8(0x400c),
            machine.memory.read8(0x400d),
        ]));
        assert!(display > 0x407d && display <= end);
        assert_eq!(&program[0x407d - 0x4009..0x407f - 0x4009], &[0, 10]);
        core.machine.as_mut().unwrap().tape = Tape::from_p(&program).unwrap();
        core.machine.as_mut().unwrap().memory.write8(0x407e, 20);
        press(&mut core, &[(6, 3)]);
        press(&mut core, &[(0, 0), (5, 0)]);
        press(&mut core, &[(0, 0), (5, 0)]);
        press(&mut core, &[(6, 0)]);
        core.toggle_tape().unwrap();
        while core.tape_state().1 {
            core.run_cycles(3_250_000).unwrap();
        }

        core.run_cycles(3_250_000 * 5).unwrap();
        let memory = &core.machine.as_ref().unwrap().memory;
        let loaded: Vec<u8> = (0x407d..display)
            .map(|address| memory.read8(address as u16))
            .collect();
        assert_eq!(loaded, program[0x407d - 0x4009..display - 0x4009]);
        assert!(video::render_display_file(memory).is_ok());
        assert!(core.display_signal_active());
        press(&mut core, &[(2, 3)]);
        press(&mut core, &[(6, 0)]);
        core.run_cycles(3_250_000 * 3).unwrap();
        let memory = &core.machine.as_ref().unwrap().memory;
        let display = u16::from_le_bytes([memory.read8(0x400c), memory.read8(0x400d)]);
        assert_eq!(memory.read8(display + 1), 0x1d);

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("zx81-autoload-{}-{unique}.P", std::process::id()));
        std::fs::write(&path, &program).unwrap();
        let result = crate::autoload::AutoLoad::from_path(&mut core, &path);
        std::fs::remove_file(&path).unwrap();
        let mut loader = result.unwrap();
        let mut started = false;
        for _ in 0..5000 {
            let cycles = if loader.is_playing() {
                core.run_cycles_until_tape_stops(CPU_HZ / 2).unwrap()
            } else {
                core.run_cycles(CPU_HZ / 60).unwrap()
            };
            if loader.advance(&mut core, cycles).unwrap() {
                started = true;
                break;
            }
        }
        assert!(started, "automatic LOAD and RUN did not complete");
        core.run_cycles(CPU_HZ * 3).unwrap();
        let memory = &core.machine.as_ref().unwrap().memory;
        let display = u16::from_le_bytes([memory.read8(0x400c), memory.read8(0x400d)]);
        assert_eq!(memory.read8(display + 1), 0x1d);
    }

    #[test]
    fn saving_requires_an_initialized_basic_prompt() {
        let mut core = EmulatorCore::new();
        assert_eq!(
            core.save_program_bytes().unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        core.load_rom_bytes(&[0; crate::memory::ROM_SIZE]).unwrap();
        assert_eq!(
            core.save_program_bytes().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    #[ignore = "requires ZX81_ROM and ZX81_PROGRAM paths"]
    fn supplied_program_loads_and_runs() {
        let rom = std::env::var_os("ZX81_ROM").expect("set ZX81_ROM");
        let program = std::env::var_os("ZX81_PROGRAM").expect("set ZX81_PROGRAM");
        let mut core = EmulatorCore::new();
        core.load_rom(rom).unwrap();
        let mut loader =
            crate::autoload::AutoLoad::from_path(&mut core, std::path::Path::new(&program))
                .unwrap();
        let mut started = false;
        for _ in 0..10_000 {
            let budget = if loader.is_playing() {
                CPU_HZ / 2
            } else {
                CPU_HZ / 10
            };
            let cycles = if loader.is_playing() {
                core.run_cycles_until_tape_stops(budget).unwrap()
            } else {
                core.run_cycles(budget).unwrap()
            };
            if loader.advance(&mut core, cycles).unwrap() {
                started = true;
                break;
            }
        }
        assert!(started, "automatic LOAD and RUN did not complete");
        core.run_cycles(CPU_HZ * 5).unwrap();
        core.refresh_display().unwrap();
        assert!(core.display_signal_active());
        assert!(core.display_buffer.as_ref().unwrap().contains(&0));
    }
}
