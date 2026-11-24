use std::{io, path::Path};

use crate::emulator::{EmulatorCore, CPU_HZ};

const KEY_CYCLES: u64 = CPU_HZ as u64 / 5;
const PROMPT_TIMEOUT: u64 = CPU_HZ as u64 * 20;
const PROMPT_STABLE_CYCLES: u64 = CPU_HZ as u64;
const LOAD_SETTLE_CYCLES: u64 = CPU_HZ as u64 * 5;
const LOAD_COMMAND: &[u8] = &[0xef, 0x0b, 0x0b];
const LOAD_KEYS: &[&[(usize, usize)]] =
    &[&[(6, 3)], &[(0, 0), (5, 0)], &[(0, 0), (5, 0)], &[(6, 0)]];
const RUN_KEYS: &[&[(usize, usize)]] = &[&[(2, 3)], &[(6, 0)]];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadMode {
    #[default]
    Standard,
    Instant,
}

enum Phase {
    AwaitPrompt,
    Typing {
        run: bool,
        key: usize,
        pressed: bool,
    },
    Playing,
    AwaitLoaded {
        verified_self_start: bool,
        report_dismissed: bool,
    },
    DismissingReport {
        verified_self_start: bool,
    },
}

pub struct AutoLoad {
    phase: Phase,
    elapsed: u64,
    prompt_stable_cycles: u64,
    bytes: Vec<u8>,
    display_file: u16,
    self_starting: bool,
    mode: LoadMode,
}

impl AutoLoad {
    pub fn is_playing(&self) -> bool {
        matches!(self.phase, Phase::Playing)
    }

    pub fn is_instant(&self) -> bool {
        self.mode == LoadMode::Instant
    }

    #[cfg(test)]
    pub fn from_path(core: &mut EmulatorCore, path: &Path) -> io::Result<Self> {
        Self::from_path_with_mode(core, path, LoadMode::Standard)
    }

    pub fn from_path_with_mode(
        core: &mut EmulatorCore,
        path: &Path,
        mode: LoadMode,
    ) -> io::Result<Self> {
        if !path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("p"))
        {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "expected a .P file",
            ));
        }
        Self::from_bytes_with_mode(core, std::fs::read(path)?, mode)
    }

    #[cfg(test)]
    pub(crate) fn from_bytes(core: &mut EmulatorCore, bytes: Vec<u8>) -> io::Result<Self> {
        Self::from_bytes_with_mode(core, bytes, LoadMode::Standard)
    }

    pub(crate) fn from_bytes_with_mode(
        core: &mut EmulatorCore,
        bytes: Vec<u8>,
        mode: LoadMode,
    ) -> io::Result<Self> {
        // Validate the tape before examining its saved display-file pointer.
        let mut tape = crate::tape::Tape::from_p(&bytes)?;
        let display_file = u16::from_le_bytes([bytes[3], bytes[4]]);
        let end = usize::from(u16::from_le_bytes([bytes[11], bytes[12]]));
        if end > core.ram_end() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".P file exceeds the selected RAM size",
            ));
        }
        if !(0x407d..end).contains(&usize::from(display_file)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".P file has an invalid display-file pointer",
            ));
        }
        if mode == LoadMode::Instant {
            if !core.supports_instant_load() {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "instant loading requires a compatible ZX81 ROM",
                ));
            }
            tape.enable_instant(&bytes);
        }
        core.insert_tape(tape)?;
        core.reset();
        let self_starting = has_self_start_line(&bytes, display_file);
        Ok(Self {
            phase: Phase::AwaitPrompt,
            elapsed: 0,
            prompt_stable_cycles: 0,
            bytes,
            display_file,
            self_starting,
            mode,
        })
    }

    /// Advance after the machine has run; returns true when loading is complete.
    pub fn advance(&mut self, core: &mut EmulatorCore, cycles: u32) -> io::Result<bool> {
        self.elapsed += u64::from(cycles);
        match self.phase {
            Phase::AwaitPrompt => {
                if core.basic_cursor_visible() {
                    self.prompt_stable_cycles += u64::from(cycles);
                } else {
                    self.prompt_stable_cycles = 0;
                }
                if self.prompt_stable_cycles >= PROMPT_STABLE_CYCLES {
                    self.start_typing(core, false)?;
                } else if self.elapsed >= PROMPT_TIMEOUT {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "ZX81 did not reach the BASIC prompt",
                    ));
                }
            }
            Phase::Typing { run, key, pressed } if self.elapsed >= KEY_CYCLES => {
                let keys = if run { RUN_KEYS } else { LOAD_KEYS };
                self.elapsed = 0;
                if pressed {
                    set_keys(core, keys[key], false)?;
                    self.phase = Phase::Typing {
                        run,
                        key,
                        pressed: false,
                    };
                } else if key + 1 < keys.len() {
                    if !run && key + 1 == keys.len() - 1 && !core.basic_input_matches(LOAD_COMMAND)
                    {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "ZX81 did not accept the LOAD command",
                        ));
                    }
                    set_keys(core, keys[key + 1], true)?;
                    self.phase = Phase::Typing {
                        run,
                        key: key + 1,
                        pressed: true,
                    };
                } else if run {
                    return Ok(true);
                } else {
                    core.toggle_tape()?;
                    self.phase = Phase::Playing;
                }
            }
            Phase::Playing if !core.tape_state().1 => {
                self.phase = Phase::AwaitLoaded {
                    verified_self_start: self.self_starting
                        && core.program_matches(&self.bytes, self.display_file),
                    report_dismissed: false,
                };
                self.elapsed = 0;
            }
            Phase::AwaitLoaded {
                verified_self_start,
                report_dismissed,
            } => {
                if self.elapsed >= LOAD_SETTLE_CYCLES && core.display_signal_active() {
                    if verified_self_start && !core.basic_cursor_visible() {
                        return Ok(true);
                    }
                    if core.basic_cursor_visible()
                        && core.program_matches(&self.bytes, self.display_file)
                    {
                        self.start_typing(core, true)?;
                        return Ok(false);
                    }
                    if !self.self_starting
                        && !report_dismissed
                        && core.program_matches(&self.bytes, self.display_file)
                        && core.basic_load_report_visible()
                    {
                        set_keys(core, RUN_KEYS[1], true)?;
                        self.phase = Phase::DismissingReport {
                            verified_self_start,
                        };
                        self.elapsed = 0;
                    }
                }
                if self.elapsed >= PROMPT_TIMEOUT {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "ROM did not finish loading the .P program",
                    ));
                }
            }
            Phase::DismissingReport {
                verified_self_start,
            } if self.elapsed >= KEY_CYCLES => {
                set_keys(core, RUN_KEYS[1], false)?;
                self.phase = Phase::AwaitLoaded {
                    verified_self_start,
                    report_dismissed: true,
                };
                self.elapsed = 0;
            }
            Phase::Typing { .. } | Phase::Playing | Phase::DismissingReport { .. } => {}
        }
        Ok(false)
    }

    fn start_typing(&mut self, core: &mut EmulatorCore, run: bool) -> io::Result<()> {
        let keys = if run { RUN_KEYS } else { LOAD_KEYS };
        set_keys(core, keys[0], true)?;
        self.phase = Phase::Typing {
            run,
            key: 0,
            pressed: true,
        };
        self.elapsed = 0;
        Ok(())
    }
}

fn has_self_start_line(bytes: &[u8], display_file: u16) -> bool {
    let next = usize::from(u16::from_le_bytes([bytes[0x20], bytes[0x21]]));
    let end = usize::from(display_file);
    let mut address = 0x407d;
    while address + 4 <= end {
        let offset = address - 0x4009;
        let length = usize::from(u16::from_le_bytes([bytes[offset + 2], bytes[offset + 3]]));
        if length == 0 || address + 4 + length > end {
            return false;
        }
        if address == next {
            return true;
        }
        address += 4 + length;
    }
    false
}

fn set_keys(core: &mut EmulatorCore, keys: &[(usize, usize)], pressed: bool) -> io::Result<()> {
    for &(row, column) in keys {
        core.set_key(row, column, pressed)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ROM_SIZE;

    #[test]
    fn recognises_only_valid_self_start_line_boundaries() {
        let mut bytes = vec![0; 0x80];
        bytes[0x20..0x22].copy_from_slice(&0x407du16.to_le_bytes());
        bytes[0x76..0x78].copy_from_slice(&2u16.to_le_bytes());
        assert!(has_self_start_line(&bytes, 0x4083));
        bytes[0x20..0x22].copy_from_slice(&0x407eu16.to_le_bytes());
        assert!(!has_self_start_line(&bytes, 0x4083));
        bytes[0x20..0x22].copy_from_slice(&0x407du16.to_le_bytes());
        bytes[0x76..0x78].copy_from_slice(&7u16.to_le_bytes());
        assert!(!has_self_start_line(&bytes, 0x4083));
    }

    fn program() -> Vec<u8> {
        let mut bytes = vec![0; 0x80];
        bytes[3..5].copy_from_slice(&0x407du16.to_le_bytes());
        bytes[11..13].copy_from_slice(&0x4089u16.to_le_bytes());
        bytes
    }

    #[test]
    fn rejects_invalid_files_and_requires_a_rom() {
        let mut core = EmulatorCore::new();
        assert_eq!(
            AutoLoad::from_path(&mut core, Path::new("program.tap"))
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            AutoLoad::from_bytes(&mut core, program())
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::NotFound
        );
        core.load_rom_bytes(&[0; ROM_SIZE]).unwrap();
        let mut invalid = program();
        invalid[3..5].copy_from_slice(&0x8000u16.to_le_bytes());
        assert_eq!(
            AutoLoad::from_bytes(&mut core, invalid)
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn reports_a_missing_basic_prompt() {
        let mut core = EmulatorCore::new();
        core.load_rom_bytes(&[0; ROM_SIZE]).unwrap();
        let mut loader = AutoLoad::from_bytes(&mut core, program()).unwrap();
        assert_eq!(
            loader
                .advance(&mut core, PROMPT_TIMEOUT as u32)
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
    }

    #[test]
    fn rejects_a_program_larger_than_selected_ram_without_resetting() {
        let mut core = EmulatorCore::new();
        core.set_ram_size(crate::memory::RamSize::OneK);
        core.load_rom_bytes(&[0; ROM_SIZE]).unwrap();
        core.run_cycles(1000).unwrap();
        let before = core.cpu_snapshot().unwrap().cycles;
        let mut bytes = vec![0; 0x4401 - 0x4009];
        bytes[3..5].copy_from_slice(&0x407du16.to_le_bytes());
        bytes[11..13].copy_from_slice(&0x4401u16.to_le_bytes());
        for mode in [LoadMode::Standard, LoadMode::Instant] {
            let error = AutoLoad::from_bytes_with_mode(&mut core, bytes.clone(), mode)
                .err()
                .unwrap();
            assert_eq!(error.kind(), io::ErrorKind::InvalidData);
            assert!(error.to_string().contains("selected RAM size"));
            assert_eq!(core.cpu_snapshot().unwrap().cycles, before);
        }

        let mut fits = vec![0; 0x4400 - 0x4009];
        fits[3..5].copy_from_slice(&0x407du16.to_le_bytes());
        fits[11..13].copy_from_slice(&0x4400u16.to_le_bytes());
        assert!(AutoLoad::from_bytes(&mut core, fits).is_ok());
    }

    #[test]
    fn exported_wav_decodes_and_loads_through_the_rom() {
        use crate::tape::{
            encoder::encode_program_to_pulses, signal::generate_pcm_samples, wav_loader,
            wav_writer::write_wav,
        };

        let file = include_bytes!("../Games Library/3D MONSTER MAZE.p");
        let end = usize::from(u16::from_le_bytes([file[11], file[12]])) - 0x4009;
        let original = &file[..end];
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("maze.wav");
        write_wav(
            &path,
            generate_pcm_samples(encode_program_to_pulses(original)),
        )
        .unwrap();
        let decoded = wav_loader::load_program(&path).unwrap();
        assert_eq!(decoded, original);

        let mut core = EmulatorCore::new();
        core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
            .unwrap();
        let mut loader =
            AutoLoad::from_bytes_with_mode(&mut core, decoded, LoadMode::Instant).unwrap();
        let display = u16::from_le_bytes([original[3], original[4]]);
        for _ in 0..2000 {
            let cycles = if loader.is_playing() {
                core.run_cycles_until_tape_stops(CPU_HZ / 60).unwrap()
            } else {
                core.run_cycles(CPU_HZ / 60).unwrap()
            };
            if loader.advance(&mut core, cycles).unwrap() {
                assert!(core.program_matches(original, display));
                return;
            }
        }
        panic!("decoded WAV did not finish loading");
    }
}
