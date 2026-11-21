use std::{io, path::Path};

use crate::emulator::{EmulatorCore, CPU_HZ};

const KEY_CYCLES: u64 = CPU_HZ as u64 / 5;
const PROMPT_TIMEOUT: u64 = CPU_HZ as u64 * 20;
const LOAD_SETTLE_CYCLES: u64 = CPU_HZ as u64 * 5;
const LOAD_KEYS: &[&[(usize, usize)]] =
    &[&[(6, 3)], &[(0, 0), (5, 0)], &[(0, 0), (5, 0)], &[(6, 0)]];
const RUN_KEYS: &[&[(usize, usize)]] = &[&[(2, 3)], &[(6, 0)]];

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
    bytes: Vec<u8>,
    display_file: u16,
    self_starting: bool,
}

impl AutoLoad {
    pub fn is_playing(&self) -> bool {
        matches!(self.phase, Phase::Playing)
    }

    pub fn from_path(core: &mut EmulatorCore, path: &Path) -> io::Result<Self> {
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
        Self::from_bytes(core, std::fs::read(path)?)
    }

    pub(crate) fn from_bytes(core: &mut EmulatorCore, bytes: Vec<u8>) -> io::Result<Self> {
        // Validate the tape before examining its saved display-file pointer.
        let tape = crate::tape::Tape::from_p(&bytes)?;
        let display_file = u16::from_le_bytes([bytes[3], bytes[4]]);
        let end = usize::from(u16::from_le_bytes([bytes[11], bytes[12]]));
        if !(0x407d..end).contains(&usize::from(display_file)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".P file has an invalid display-file pointer",
            ));
        }
        core.insert_tape(tape)?;
        core.reset();
        let self_starting = has_self_start_line(&bytes, display_file);
        Ok(Self {
            phase: Phase::AwaitPrompt,
            elapsed: 0,
            bytes,
            display_file,
            self_starting,
        })
    }

    /// Advance after the machine has run; returns true when loading is complete.
    pub fn advance(&mut self, core: &mut EmulatorCore, cycles: u32) -> io::Result<bool> {
        self.elapsed += u64::from(cycles);
        match self.phase {
            Phase::AwaitPrompt => {
                if core.basic_cursor_visible() {
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
}
