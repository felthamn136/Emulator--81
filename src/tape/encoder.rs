use super::{GAP_CYCLES, HIGH_CYCLES, WAVE_CYCLES};

/// One cassette wave, followed by any inter-bit silence, in Z80 T-states.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pulse {
    pub high_cycles: u32,
    pub low_cycles: u32,
    pub silence_cycles: u32,
}

/// Encode an unnamed ZX81 tape header and the complete .P memory image.
pub fn encode_program_to_pulses(bytes: &[u8]) -> impl Iterator<Item = Pulse> + '_ {
    std::iter::once(0x80)
        .chain(bytes.iter().copied())
        .flat_map(|byte| {
            (0..8).rev().flat_map(move |bit| {
                let count = if byte & (1 << bit) != 0 { 9 } else { 4 };
                (0..count).map(move |wave| Pulse {
                    high_cycles: HIGH_CYCLES as u32,
                    low_cycles: (WAVE_CYCLES - HIGH_CYCLES) as u32,
                    silence_cycles: if wave + 1 == count {
                        GAP_CYCLES as u32
                    } else {
                        0
                    },
                })
            })
        })
}
