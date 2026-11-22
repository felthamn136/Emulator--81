use std::path::Path;

use zx81_emulator::tape::{
    encoder::{encode_program_to_pulses, Pulse},
    signal::{generate_pcm_samples, SAMPLE_RATE},
    wav_loader::{load_program, load_program_with_progress},
};

fn program() -> Vec<u8> {
    let mut program: Vec<u8> = (0..160).map(|i| (i * 37) as u8).collect();
    let end = (0x4009 + program.len()) as u16;
    program[11..13].copy_from_slice(&end.to_le_bytes());
    program[3..5].copy_from_slice(&0x407du16.to_le_bytes());
    program
}

fn named_pulses(bytes: &[u8]) -> impl Iterator<Item = Pulse> + '_ {
    [0x26u8, 0x27, 0xa8]
        .into_iter()
        .chain(bytes.iter().copied())
        .flat_map(|byte| {
            (0..8).rev().flat_map(move |bit| {
                let count = if byte & (1 << bit) != 0 { 9 } else { 4 };
                (0..count).map(move |i| Pulse {
                    high_cycles: 503,
                    low_cycles: 472,
                    silence_cycles: if i == count - 1 { 4389 } else { 0 },
                })
            })
        })
}

fn write_recording(path: &Path, samples: &[i16], rate: u32, bits: u16, channels: u16, speed: f64) {
    let spec = hound::WavSpec {
        channels,
        sample_rate: rate,
        bits_per_sample: bits,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    let frames = (samples.len() as f64 * f64::from(rate) / f64::from(SAMPLE_RATE) / speed) as usize;
    for i in 0..frames {
        let source = (i as f64 * f64::from(SAMPLE_RATE) * speed / f64::from(rate)) as usize;
        let noise = ((i * 53 % 241) as i32) - 120;
        let value = (i32::from(samples[source]) / 2 + 4500 + noise) as i16;
        for channel in 0..channels {
            let value = if channel == 0 {
                value
            } else {
                value / 2 - 2000
            };
            if bits == 8 {
                writer.write_sample((value >> 8) as i8).unwrap();
            } else {
                writer.write_sample(value).unwrap();
            }
        }
    }
    writer.finalize().unwrap();
}

#[test]
fn exported_wav_round_trips_and_reports_progress() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.wav");
    let expected = program();
    zx81_emulator::tape::wav_writer::write_wav(
        &path,
        generate_pcm_samples(encode_program_to_pulses(&expected)),
    )
    .unwrap();
    let mut stages = Vec::new();
    assert_eq!(
        load_program_with_progress(&path, |stage| stages.push(stage)).unwrap(),
        expected
    );
    assert_eq!(
        stages,
        [
            "Loading WAV...",
            "Analysing Audio...",
            "Detecting Tape Signal...",
            "Decoding Program..."
        ]
    );
}

#[test]
fn silent_leader_named_header_formats_and_distortions() {
    let dir = tempfile::tempdir().unwrap();
    let expected = program();
    let all: Vec<_> = generate_pcm_samples(named_pulses(&expected)).collect();
    // Drop the exporter's tone, retaining its silent leader and trailer.
    let samples = &all[5 * SAMPLE_RATE as usize..];
    for &(rate, bits, channels, speed) in &[
        (22_050, 8, 1, 0.90),
        (44_100, 16, 2, 1.10),
        (48_000, 8, 2, 1.0),
        (48_000, 16, 1, 1.0),
    ] {
        let path = dir.path().join(format!("{rate}-{bits}-{channels}.wav"));
        write_recording(&path, samples, rate, bits, channels, speed);
        assert_eq!(
            load_program(&path).unwrap(),
            expected,
            "format {rate}/{bits}/{channels}"
        );
    }
    let inverted: Vec<_> = samples.iter().map(|sample| -*sample).collect();
    let path = dir.path().join("inverted.wav");
    write_recording(&path, &inverted, 44_100, 16, 1, 1.0);
    assert_eq!(load_program(&path).unwrap(), expected);

    let mut noisy_leader = samples.to_vec();
    let start = SAMPLE_RATE as usize / 4;
    for i in 0..52 {
        noisy_leader[start + i] = if (i % 13) < 7 { 16_384 } else { -16_384 };
    }
    let path = dir.path().join("noisy-leader.wav");
    write_recording(&path, &noisy_leader, 44_100, 16, 1, 1.0);
    assert_eq!(load_program(&path).unwrap(), expected);
}

#[test]
fn stereo_channels_are_averaged_before_detection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("stereo.wav");
    let expected = program();
    let samples: Vec<_> = generate_pcm_samples(encode_program_to_pulses(&expected)).collect();
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    for (i, sample) in samples.into_iter().enumerate() {
        let tone = sample / 4;
        let noise = if i % 2 == 0 { 12_000 } else { -12_000 };
        writer.write_sample(tone + noise + 1000).unwrap();
        writer.write_sample(tone - noise + 1000).unwrap();
    }
    writer.finalize().unwrap();
    assert_eq!(load_program(&path).unwrap(), expected);
}

#[test]
fn filtered_cassette_with_hiss_and_amplitude_variation_decodes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cassette.wav");
    let expected = program();
    let mut writer = hound::WavWriter::create(
        &path,
        hound::WavSpec {
            channels: 1,
            sample_rate: SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
    .unwrap();
    let mut seed = 0x1234_5678u32;
    let mut filtered = 0.0f32;
    for (index, sample) in generate_pcm_samples(encode_program_to_pulses(&expected)).enumerate() {
        let source = if index < 5 * SAMPLE_RATE as usize {
            0.0
        } else {
            f32::from(sample)
        };
        let gain = 0.7 + 0.2 * (index as f32 / 80_000.0).sin();
        filtered += 0.4 * (source * gain - filtered);
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let noise = ((seed >> 16) as i32 - 32768) as f32 / 45.0;
        writer
            .write_sample((filtered + noise + 3_000.0).round() as i16)
            .unwrap();
    }
    writer.finalize().unwrap();
    assert_eq!(load_program(&path).unwrap(), expected);
}

#[test]
fn rejects_silence_noise_unsupported_and_truncated_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.wav");
    for samples in [
        vec![0; 44_100],
        (0..44_100)
            .map(|i| ((i * 173 % 5000) as i16) - 2500)
            .collect(),
    ] {
        write_recording(&path, &samples, 44_100, 16, 1, 1.0);
        assert!(load_program(&path).is_err());
    }
    write_recording(&path, &[0; 1024], 32_000, 16, 1, 1.0);
    assert!(load_program(&path)
        .unwrap_err()
        .to_string()
        .contains("Unsupported"));
    let expected = program();
    let samples: Vec<_> = generate_pcm_samples(encode_program_to_pulses(&expected)).collect();
    let data_start = 6 * SAMPLE_RATE as usize;
    write_recording(
        &path,
        &samples[..data_start + (samples.len() - data_start) / 3],
        44_100,
        16,
        1,
        1.0,
    );
    assert!(load_program(&path).is_err());
    let mut wav = std::fs::read(&path).unwrap();
    wav.truncate(wav.len() - 3);
    std::fs::write(&path, wav).unwrap();
    assert!(load_program(&path).is_err());
    std::fs::write(&path, b"not a WAV").unwrap();
    assert!(load_program(&path).is_err());
}

#[test]
fn rejects_invalid_end_pointer() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad-program.wav");
    let mut invalid = program();
    invalid[11..13].copy_from_slice(&0xffffu16.to_le_bytes());
    zx81_emulator::tape::wav_writer::write_wav(
        &path,
        generate_pcm_samples(encode_program_to_pulses(&invalid)),
    )
    .unwrap();
    assert!(load_program(&path)
        .unwrap_err()
        .to_string()
        .contains("end address"));
    let mut invalid = program();
    invalid[3..5].copy_from_slice(&0x4000u16.to_le_bytes());
    zx81_emulator::tape::wav_writer::write_wav(
        &path,
        generate_pcm_samples(encode_program_to_pulses(&invalid)),
    )
    .unwrap();
    assert!(matches!(
        load_program(&path),
        Err(zx81_emulator::tape::TapeError::InvalidProgram(_))
    ));
}
