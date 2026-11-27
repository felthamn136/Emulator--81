use std::borrow::Borrow;

use crate::emulator::CPU_HZ;

use super::{encoder::Pulse, HIGH_CYCLES, WAVE_CYCLES};

pub const SAMPLE_RATE: u32 = 44_100;
const AMPLITUDE: i16 = 16_384;

#[derive(Clone, Copy)]
struct Segment {
    cycles: u32,
    level: i16,
}

fn tone(cycles: u32) -> impl Iterator<Item = Segment> {
    let wave = WAVE_CYCLES as u32;
    let high = HIGH_CYCLES as u32;
    let remainder = cycles % wave;
    (0..cycles / wave)
        .flat_map(move |_| {
            [
                Segment {
                    cycles: high,
                    level: AMPLITUDE,
                },
                Segment {
                    cycles: wave - high,
                    level: -AMPLITUDE,
                },
            ]
        })
        .chain([
            Segment {
                cycles: remainder.min(high),
                level: AMPLITUDE,
            },
            Segment {
                cycles: remainder.saturating_sub(high),
                level: -AMPLITUDE,
            },
        ])
}

fn pulse_segments(pulse: Pulse) -> [Segment; 3] {
    [
        Segment {
            cycles: pulse.high_cycles,
            level: AMPLITUDE,
        },
        Segment {
            cycles: pulse.low_cycles,
            level: -AMPLITUDE,
        },
        Segment {
            cycles: pulse.silence_cycles,
            level: 0,
        },
    ]
}

/// Generate mono PCM without collecting the entire recording in memory.
pub fn generate_pcm_samples<I>(pulses: I) -> impl Iterator<Item = i16>
where
    I: IntoIterator,
    I::Item: Borrow<Pulse>,
{
    let segments = tone(CPU_HZ * 5)
        .chain(std::iter::once(Segment {
            cycles: CPU_HZ,
            level: 0,
        }))
        .chain(
            pulses
                .into_iter()
                .flat_map(|pulse| pulse_segments(*pulse.borrow())),
        )
        .chain(tone(CPU_HZ));
    let mut remainder = 0u64;
    segments.flat_map(move |segment| {
        let duration = remainder + u64::from(segment.cycles) * u64::from(SAMPLE_RATE);
        remainder = duration % u64::from(CPU_HZ);
        std::iter::repeat_n(segment.level, (duration / u64::from(CPU_HZ)) as usize)
    })
}
