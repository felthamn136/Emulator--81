use std::io;

const START: usize = 0x4009;
const END_POINTER_OFFSET: usize = 0x4014 - START;
const LEADER_CYCLES: u64 = crate::emulator::CPU_HZ as u64 * 5;
const WAVE_CYCLES: u64 = 975;
const HIGH_CYCLES: u64 = 503;
const GAP_CYCLES: u64 = 4389;

pub mod decoder;
pub mod encoder;
mod filters;
pub mod pulse_detector;
pub mod signal;
mod tape_image;
pub mod wav_loader;
pub mod wav_writer;
pub use wav_loader::TapeError;

#[derive(Default)]
pub struct Tape {
    bit_ends: Vec<u64>,
    elapsed: u64,
    playing: bool,
    instant_bytes: Option<Vec<u8>>,
    instant_position: usize,
}

impl Tape {
    /// Create a signal for LOAD "", with an unnamed ZX81 tape header.
    pub fn from_p(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < 0x407d - START {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".P file is too short",
            ));
        }
        let end = usize::from(u16::from_le_bytes([
            bytes[END_POINTER_OFFSET],
            bytes[END_POINTER_OFFSET + 1],
        ]));
        let length = end
            .checked_sub(START)
            .filter(|length| *length <= bytes.len());
        let Some(length) = length.filter(|_| end <= 0x8000) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                ".P file length does not match E_LINE or exceeds 16K RAM",
            ));
        };
        match &bytes[length..] {
            [] => {}
            [byte] => eprintln!("Ignoring an extra 0x{byte:02x} byte after the .P program."),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    ".P file length does not match E_LINE or exceeds 16K RAM",
                ));
            }
        }
        let bytes = &bytes[..length];
        let mut bit_ends = Vec::with_capacity((bytes.len() + 1) * 8);
        let mut end_cycle = LEADER_CYCLES;
        for byte in std::iter::once(&0x80).chain(bytes.iter()) {
            for bit in (0..8).rev() {
                let waves = if byte & (1 << bit) != 0 { 9 } else { 4 };
                end_cycle += waves * WAVE_CYCLES + GAP_CYCLES;
                bit_ends.push(end_cycle);
            }
        }
        Ok(Self {
            bit_ends,
            elapsed: 0,
            playing: false,
            instant_bytes: None,
            instant_position: 0,
        })
    }

    pub(crate) fn enable_instant(&mut self, bytes: &[u8]) {
        let end = usize::from(u16::from_le_bytes([
            bytes[END_POINTER_OFFSET],
            bytes[END_POINTER_OFFSET + 1],
        ]));
        self.instant_bytes = Some(
            std::iter::once(0x80)
                .chain(bytes[..end - START].iter().copied())
                .collect(),
        );
        self.instant_position = 0;
    }

    pub(crate) fn take_instant_byte(&mut self) -> Option<u8> {
        if !self.playing {
            return None;
        }
        let bytes = self.instant_bytes.as_ref()?;
        let byte = bytes[self.instant_position];
        self.instant_position += 1;
        if self.instant_position == bytes.len() {
            self.playing = false;
        }
        Some(byte)
    }

    pub fn inserted(&self) -> bool {
        !self.bit_ends.is_empty()
    }

    pub fn playing(&self) -> bool {
        self.playing
    }

    pub fn stop(&mut self) {
        self.playing = false;
    }

    pub fn toggle(&mut self) -> io::Result<()> {
        if !self.inserted() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no .P tape inserted",
            ));
        }
        if !self.playing && self.elapsed >= *self.bit_ends.last().unwrap() {
            self.elapsed = 0;
        }
        if !self.playing && self.instant_bytes.is_some() {
            self.instant_position = 0;
        }
        self.playing = !self.playing;
        Ok(())
    }

    pub fn level(&self) -> bool {
        self.level_ago(0)
    }

    pub(crate) fn level_ago(&self, cycles: u64) -> bool {
        let Some(elapsed) = self.elapsed.checked_sub(cycles) else {
            return false;
        };
        if !self.playing {
            return false;
        }
        self.level_at(elapsed)
    }

    fn level_at(&self, elapsed: u64) -> bool {
        if elapsed < LEADER_CYCLES {
            return false;
        }
        let index = self.bit_ends.partition_point(|end| *end <= elapsed);
        let Some(&end) = self.bit_ends.get(index) else {
            return false;
        };
        let start = if index == 0 {
            LEADER_CYCLES
        } else {
            self.bit_ends[index - 1]
        };
        let offset = elapsed - start;
        offset < end - start - GAP_CYCLES && offset % WAVE_CYCLES < HIGH_CYCLES
    }

    pub(crate) fn elapsed(&self) -> u64 {
        self.elapsed
    }

    pub(crate) fn audio_samples(&self, start: u64, sample_rate: u32) -> Vec<f32> {
        let rate = u64::from(sample_rate);
        let cpu_rate = u64::from(crate::emulator::CPU_HZ);
        let first = start * rate / cpu_rate;
        let end = self.elapsed * rate / cpu_rate;
        (first..end)
            .map(|sample| {
                let cycle = sample * cpu_rate / rate;
                let index = self.bit_ends.partition_point(|end| *end <= cycle);
                let Some(&bit_end) = self.bit_ends.get(index) else {
                    return 0.0;
                };
                let bit_start = if index == 0 {
                    LEADER_CYCLES
                } else {
                    self.bit_ends[index - 1]
                };
                if cycle < bit_start || cycle >= bit_end - GAP_CYCLES {
                    0.0
                } else if self.level_at(cycle) {
                    0.15
                } else {
                    -0.15
                }
            })
            .collect()
    }

    pub fn advance(&mut self, cycles: u32) {
        if self.playing && self.instant_bytes.is_none() {
            self.elapsed = self.elapsed.saturating_add(u64::from(cycles));
            if self.elapsed >= *self.bit_ends.last().unwrap() {
                self.playing = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_program() -> Vec<u8> {
        let mut bytes = vec![0; 0x80];
        let end = (START + bytes.len()) as u16;
        bytes[END_POINTER_OFFSET..END_POINTER_OFFSET + 2].copy_from_slice(&end.to_le_bytes());
        bytes
    }

    #[test]
    fn audio_samples_follow_tape_pulses_and_leave_gaps_silent() {
        let mut tape = Tape::from_p(&valid_program()).unwrap();
        tape.elapsed = LEADER_CYCLES + WAVE_CYCLES * 9 + GAP_CYCLES;
        let rate = 44_100;
        let start = LEADER_CYCLES - WAVE_CYCLES;
        let samples = tape.audio_samples(start, rate);
        let first = start * u64::from(rate) / u64::from(crate::emulator::CPU_HZ);
        for (index, &sample) in samples.iter().enumerate() {
            let cycle =
                (first + index as u64) * u64::from(crate::emulator::CPU_HZ) / u64::from(rate);
            assert_eq!(sample > 0.0, tape.level_at(cycle));
        }
        assert!(samples.contains(&0.15));
        assert!(samples.contains(&-0.15));
        assert!(samples.contains(&0.0));
    }

    #[test]
    fn validates_lengths_before_creating_tape() {
        assert!(Tape::from_p(&[]).is_err());
        let mut bytes = valid_program();
        assert!(Tape::from_p(&bytes).is_ok());
        bytes.pop();
        assert!(Tape::from_p(&bytes).is_err());
    }

    #[test]
    fn accepts_one_extra_byte_without_putting_it_on_tape() {
        let mut bytes = valid_program();
        let original_bits = Tape::from_p(&bytes).unwrap().bit_ends.len();
        bytes.push(0x8e);
        assert_eq!(Tape::from_p(&bytes).unwrap().bit_ends.len(), original_bits);
        bytes.push(0x8e);
        assert!(Tape::from_p(&bytes).is_err());
        bytes.pop();
        *bytes.last_mut().unwrap() = 0x0b;
        assert_eq!(Tape::from_p(&bytes).unwrap().bit_ends.len(), original_bits);
    }

    #[test]
    fn playback_generates_leader_and_one_and_zero_bursts() {
        let mut tape = Tape::from_p(&valid_program()).unwrap();
        tape.toggle().unwrap();
        tape.advance(LEADER_CYCLES as u32);
        assert!(tape.level());
        tape.advance(HIGH_CYCLES as u32);
        assert!(!tape.level());
        tape.advance((WAVE_CYCLES * 9 + GAP_CYCLES - HIGH_CYCLES) as u32);
        assert!(tape.level());
        tape.advance((WAVE_CYCLES * 4) as u32);
        assert!(!tape.level());
        tape.toggle().unwrap();
        assert!(!tape.level());
    }

    #[test]
    fn playback_history_tracks_the_real_signal_and_stops_with_tape() {
        let mut tape = Tape::from_p(&valid_program()).unwrap();
        tape.toggle().unwrap();
        tape.advance((LEADER_CYCLES + HIGH_CYCLES) as u32);
        assert!(!tape.level());
        assert!(tape.level_ago(1));
        assert!(!tape.level_ago(HIGH_CYCLES + 1));
        assert!(!tape.level_ago(LEADER_CYCLES + HIGH_CYCLES + 1));
        tape.stop();
        assert!(!tape.level_ago(1));
    }

    #[test]
    fn synthesized_signal_decodes_to_the_header_and_complete_program() {
        let program = valid_program();
        let mut tape = Tape::from_p(&program).unwrap();
        tape.toggle().unwrap();
        tape.advance(LEADER_CYCLES as u32);
        let mut decoded = Vec::new();
        for _ in 0..program.len() + 1 {
            let mut byte = 0u8;
            for _ in 0..8 {
                while !tape.level() {
                    assert!(tape.playing());
                    tape.advance(51);
                }
                let mut samples = 0;
                let mut silent = 0;
                while silent < 26 {
                    if tape.level() {
                        silent = 0;
                    } else {
                        silent += 1;
                    }
                    samples += 1;
                    tape.advance(51);
                }
                assert!(samples > 62);
                byte = (byte << 1) | u8::from(samples > 148);
            }
            decoded.push(byte);
        }
        assert_eq!(decoded[0], 0x80);
        assert_eq!(&decoded[1..], program);
    }
}
