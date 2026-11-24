use std::{fs, path::Path};

use zx81_emulator::{
    emulator::{EmulatorCore, CPU_HZ},
    memory::{Memory, RamSize},
    tape::{
        encoder::encode_program_to_pulses,
        signal::{generate_pcm_samples, SAMPLE_RATE},
        wav_writer::write_wav,
    },
};

fn write_program(path: &Path, program: &[u8]) {
    write_wav(
        path,
        generate_pcm_samples(encode_program_to_pulses(program)),
    )
    .expect("write WAV");
}

fn decode_wav(path: &Path, expected_bytes: usize) -> Vec<u8> {
    let wav = fs::read(path).expect("read WAV");
    assert_eq!(&wav[..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize,
        wav.len() - 8
    );
    assert_eq!(&wav[8..16], b"WAVEfmt ");
    assert_eq!(u16::from_le_bytes(wav[20..22].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(wav[22..24].try_into().unwrap()), 1);
    assert_eq!(u32::from_le_bytes(wav[24..28].try_into().unwrap()), 44_100);
    assert_eq!(u16::from_le_bytes(wav[34..36].try_into().unwrap()), 16);
    assert_eq!(&wav[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize,
        wav.len() - 44
    );
    let samples: Vec<i16> = wav[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    assert!(samples[..SAMPLE_RATE as usize * 5].contains(&16_384));
    assert!(samples[..SAMPLE_RATE as usize * 5].contains(&-16_384));
    assert!(samples[SAMPLE_RATE as usize * 5..SAMPLE_RATE as usize * 6]
        .iter()
        .all(|&sample| sample == 0));
    assert!(samples[samples.len() - SAMPLE_RATE as usize..].contains(&16_384));

    let mut position = SAMPLE_RATE as usize * 6;
    let mut decoded = Vec::with_capacity(expected_bytes);
    for _ in 0..expected_bytes {
        let mut byte = 0u8;
        for _ in 0..8 {
            while samples[position] == 0 {
                position += 1;
            }
            let mut waves = 0;
            let mut previous = 0;
            while samples[position] != 0 {
                let sample = samples[position];
                if sample > 0 && previous <= 0 {
                    waves += 1;
                }
                previous = sample;
                position += 1;
            }
            assert!(waves == 4 || waves == 9, "invalid wave count: {waves}");
            byte = (byte << 1) | u8::from(waves == 9);
        }
        decoded.push(byte);
    }
    decoded
}

#[test]
fn encodes_filename_and_bits_in_rom_order() {
    let pulses: Vec<_> = encode_program_to_pulses(&[0x01]).collect();
    let bit_waves: Vec<usize> = pulses
        .split_inclusive(|pulse| pulse.silence_cycles > 0)
        .map(|bit| bit.len())
        .collect();
    assert_eq!(bit_waves, [9, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 9]);
    assert!(pulses
        .iter()
        .all(|pulse| pulse.high_cycles == 503 && pulse.low_cycles == 472));
}

#[test]
fn wav_round_trip_preserves_small_program() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("hello.wav");
    let mut core = EmulatorCore::new();
    core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
        .unwrap();
    core.run_cycles(CPU_HZ * 15).unwrap();
    let press = |core: &mut EmulatorCore, keys: &[(usize, usize)]| {
        for (index, &(row, column)) in keys.iter().enumerate() {
            core.set_key(row, column, true).unwrap();
            if index + 1 < keys.len() {
                core.run_cycles(CPU_HZ / 5).unwrap();
            }
        }
        core.run_cycles(CPU_HZ / 5).unwrap();
        for &(row, column) in keys {
            core.set_key(row, column, false).unwrap();
        }
        core.run_cycles(CPU_HZ / 5).unwrap();
    };
    for (index, keys) in [
        &[(3, 0)][..],
        &[(4, 0)],
        &[(5, 0)],
        &[(0, 0), (5, 0)],
        &[(6, 4)],
        &[(2, 2)],
        &[(6, 1)],
        &[(6, 1)],
        &[(5, 1)],
        &[(0, 0), (5, 0)],
        &[(6, 0)],
        &[(3, 1)],
        &[(4, 0)],
        &[(1, 4)],
        &[(3, 0)],
        &[(4, 0)],
        &[(6, 0)],
    ]
    .into_iter()
    .enumerate()
    {
        press(&mut core, keys);
        if index == 10 {
            core.run_cycles(CPU_HZ).unwrap();
        }
    }
    core.run_cycles(CPU_HZ).unwrap();
    let program = core.program_image_bytes().unwrap();
    let line = 0x407d - 0x4009;
    assert_eq!(&program[line..line + 2], &[0, 10]);
    let length = usize::from(u16::from_le_bytes([program[line + 2], program[line + 3]]));
    assert_eq!(&program[line + 4 + length..line + 6 + length], &[0, 20]);
    assert_eq!(program[line + 8 + length], 0xec); // GOTO
    write_program(&path, &program);
    let decoded = decode_wav(&path, program.len() + 1);
    assert_eq!(decoded[0], 0x80);
    assert_eq!(&decoded[1..], program);
}

#[test]
fn wav_round_trip_preserves_large_program() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("monster.wav");
    let file = include_bytes!("../Games Library/3D MONSTER MAZE.p");
    let end = usize::from(u16::from_le_bytes([file[11], file[12]])) - 0x4009;
    let program = &file[..end];
    write_program(&path, program);
    let decoded = decode_wav(&path, program.len() + 1);
    assert_eq!(decoded[0], 0x80);
    assert_eq!(&decoded[1..], program);
}

#[test]
fn fresh_machine_exports_its_empty_basic_image() {
    let mut core = EmulatorCore::new();
    assert!(core.program_image_bytes().is_err());
    core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
        .unwrap();
    core.run_cycles(CPU_HZ * 15).unwrap();
    let program = core.program_image_bytes().unwrap();
    assert_eq!(program, core.save_program_bytes().unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.wav");
    write_program(&path, &program);
    assert_eq!(&decode_wav(&path, program.len() + 1)[1..], program);
}

#[test]
fn program_image_rejects_pointers_beyond_installed_ram() {
    let mut core = EmulatorCore::new();
    core.set_ram_size(RamSize::OneK);
    core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
        .unwrap();
    let pack = core.ram_pack_mut().unwrap();
    pack.write(0x400c, 0x7d);
    pack.write(0x400d, 0x40);
    pack.write(0x4014, 0x01);
    pack.write(0x4015, 0x44);
    assert!(core.program_image_bytes().is_err());
}

#[test]
fn zx81_rom_decodes_generated_audio() {
    let program = [0x00, 0x01, 0xa5, 0x7f, 0xff];
    let samples: Vec<i16> = generate_pcm_samples(encode_program_to_pulses(&program)).collect();
    let mut memory = Memory::new(include_bytes!("../rom/zx81.rom")).unwrap();
    let mut cpu = z80::Z80::new();
    cpu.init();
    let mut cycles = u64::from(CPU_HZ) * 6 - 1_000;
    for expected in std::iter::once(0x80).chain(program) {
        cpu.pc = 0x034c;
        cpu.sp = 0x7ffe;
        memory.write8(0x7ffe, 0x00);
        memory.write8(0x7fff, 0x01);
        let deadline = cycles + u64::from(CPU_HZ);
        while cpu.pc != 0x0100 && cycles < deadline {
            let index = (cycles * u64::from(SAMPLE_RATE) / u64::from(CPU_HZ)) as usize;
            memory.set_tape_input(samples[index] > 0);
            cycles += u64::from(cpu.step(&mut memory));
        }
        assert_eq!(cpu.pc, 0x0100, "ROM did not finish reading a byte");
        assert_eq!(cpu.bc() as u8, expected);
    }
}

#[test]
fn zx81_rom_loads_wav_from_leader_to_complete_program() {
    let mut core = EmulatorCore::new();
    core.load_rom_bytes(include_bytes!("../rom/zx81.rom"))
        .unwrap();
    core.run_cycles(CPU_HZ * 15).unwrap();
    let program = core.program_image_bytes().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rom-load.wav");
    write_program(&path, &program);
    let wav = fs::read(&path).unwrap();
    let samples: Vec<i16> = wav[44..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();

    let mut memory = Memory::new(include_bytes!("../rom/zx81.rom")).unwrap();
    let mut cpu = z80::Z80::new();
    cpu.init();
    cpu.iy = 0x4000;
    cpu.sp = 0x7ffe;
    memory.write8(0x7ffe, 0x4a);
    memory.write8(0x7fff, 0x03);
    memory.write8(0x4014, 0x00);
    memory.write8(0x4015, 0x40);
    for (offset, byte) in [0x11, 0x00, 0xc0, 0xc3, 0x4c, 0x03].into_iter().enumerate() {
        memory.write8(0x5000 + offset as u16, byte);
    }
    cpu.pc = 0x5000;
    cpu.step(&mut memory);
    cpu.step(&mut memory);
    assert_eq!(cpu.pc, 0x034c);
    let mut cycles = 0u64;
    while cpu.pc != 0x0207 {
        let index = (cycles * u64::from(SAMPLE_RATE) / u64::from(CPU_HZ)) as usize;
        assert!(index < samples.len(), "ROM did not finish loading the WAV");
        memory.set_tape_input(samples[index] > 0);
        cycles += u64::from(cpu.step(&mut memory));
    }
    for (offset, &byte) in program.iter().enumerate() {
        assert_eq!(
            memory.read8(0x4009 + offset as u16),
            byte,
            "offset {offset}"
        );
    }
}

#[test]
fn wav_write_failure_is_reported() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing").join("export.wav");
    let samples: &[i16] = &[0, 1, -1];
    assert!(write_wav(&path, samples).is_err());
    assert!(!path.exists());
}
