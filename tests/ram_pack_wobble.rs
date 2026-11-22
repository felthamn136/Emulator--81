use zx81_emulator::hardware::ram_pack::{RamPack, WobbleFault, WobbleSeverity};
use zx81_emulator::{emulator::EmulatorCore, memory::RamSize};

fn enabled_pack() -> RamPack {
    let mut pack = RamPack::with_seed(12345);
    pack.set_enabled(true);
    pack
}

#[test]
fn wobble_disabled_is_transparent() {
    let mut pack = RamPack::with_seed(1);
    pack.trigger_wobble();
    pack.update(1_000_000);
    for address in 0..0x4000 {
        pack.write(address, (address & 0xff) as u8);
        assert_eq!(pack.read(address), (address & 0xff) as u8);
    }
    assert!(!pack.is_wobbling());
}

#[test]
fn trigger_wobble_activates_state() {
    let mut pack = enabled_pack();
    pack.trigger_wobble();
    assert!(pack.is_wobbling());
    assert!((100..=500).contains(&pack.state().remaining_cycles));
}

#[test]
fn severity_controls_duration() {
    let mut pack = enabled_pack();
    for (severity, min, max) in [
        (WobbleSeverity::Mild, 50, 100),
        (WobbleSeverity::Realistic, 100, 500),
        (WobbleSeverity::Extreme, 500, 2000),
    ] {
        pack.set_severity(severity);
        for _ in 0..100 {
            pack.trigger_wobble();
            assert!((min..=max).contains(&pack.state().remaining_cycles));
        }
    }
}

#[test]
fn wobble_event_expires() {
    let mut pack = enabled_pack();
    pack.trigger_wobble();
    let duration = pack.state().remaining_cycles;
    pack.update(duration - 1);
    assert!(pack.is_wobbling());
    pack.update(1);
    assert!(!pack.is_wobbling());
    assert_eq!(pack.state().remaining_cycles, 0);
}

#[test]
fn wobble_corrupts_reads() {
    let mut pack = enabled_pack();
    pack.write(0x4000, 0);
    pack.trigger_fault(WobbleFault::CorruptedReads);
    assert!((0..100).any(|_| pack.read(0x4000) != 0));
}

#[test]
fn wobble_can_drop_write() {
    let mut pack = enabled_pack();
    pack.trigger_fault(WobbleFault::DroppedWrites);
    for address in 0x4000..0x4100 {
        pack.write(address, 1);
    }
    pack.set_enabled(false);
    assert!((0x4000..0x4100).any(|address| pack.read(address) == 0));
}

#[test]
fn wobble_can_redirect_address() {
    let mut pack = enabled_pack();
    pack.write(0x4000, 0);
    for bit in 0..14 {
        pack.write(0x4000 ^ (1 << bit), 0x5a);
    }
    pack.trigger_fault(WobbleFault::AddressGlitch);
    assert!((0..100).any(|_| pack.read(0x4000) == 0x5a));
}

#[test]
fn disconnected_ram_pack_returns_ff() {
    let mut pack = enabled_pack();
    pack.write(0x4000, 0x42);
    pack.trigger_fault(WobbleFault::Disconnect);
    assert_eq!(pack.read(0x4000), 0xff);
    pack.write(0x4000, 0x24);
    pack.set_enabled(false);
    assert_eq!(pack.read(0x4000), 0x42);
}

#[test]
fn wobble_state_serializes_correctly() {
    let mut pack = enabled_pack();
    pack.set_severity(WobbleSeverity::Extreme);
    pack.set_frequency(0.5).expect("valid probability");
    pack.write(0x4000, 0x42);
    pack.trigger_fault(WobbleFault::CorruptedReads);
    pack.update(27);
    let saved = serde_json::to_string(&pack).expect("serialize pack");
    let mut restored: RamPack = serde_json::from_str(&saved).expect("restore pack");
    assert_eq!(restored.state(), pack.state());
    assert_eq!(restored.config(), pack.config());
    for _ in 0..100 {
        assert_eq!(restored.read(0x4000), pack.read(0x4000));
    }
    restored.set_enabled(false);
    assert_eq!(restored.read(0x4000), 0x42);
}

#[test]
fn event_generation_uses_emulated_frames() {
    let mut pack = enabled_pack();
    pack.set_frequency(1.0).expect("valid probability");
    pack.update(64_999);
    assert!(!pack.is_wobbling());
    pack.update(1);
    assert!(pack.is_wobbling());
    assert!(pack.set_frequency(f32::NAN).is_err());
}

#[test]
fn invalid_serialized_pack_is_rejected() {
    let pack = enabled_pack();
    let mut value = serde_json::to_value(pack).expect("serialize pack");
    value["frame_cycles"] = 65_000.into();
    assert!(serde_json::from_value::<RamPack>(value).is_err());
}

#[test]
fn disconnect_blocks_cpu_access_only_with_16k_installed() {
    let mut rom = [0; 0x2000];
    // LD A,55; LD (407d),A; JR to the write.
    rom[..7].copy_from_slice(&[0x3e, 0x55, 0x32, 0x7d, 0x40, 0x18, 0xfb]);
    for (size, expected) in [(RamSize::OneK, 0x55), (RamSize::SixteenK, 0x11)] {
        let mut core = EmulatorCore::new();
        core.set_ram_size(size);
        core.load_rom_bytes(&rom).expect("load ROM");
        let pack = core.ram_pack_mut().expect("installed RAM");
        *pack = RamPack::with_seed(12345);
        pack.write(0x407d, 0x11);
        pack.set_enabled(true);
        pack.set_severity(WobbleSeverity::Extreme);
        pack.trigger_fault(WobbleFault::Disconnect);
        core.run_cycles(100).expect("run CPU");
        let pack = core.ram_pack_mut().expect("installed RAM");
        pack.set_enabled(false);
        assert_eq!(pack.read(0x407d), expected);
    }
}
