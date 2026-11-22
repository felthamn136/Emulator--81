use std::cell::Cell;

use serde::{Deserialize, Serialize};

const RAM_SIZE: usize = 0x4000;
const FRAME_CYCLES: u32 = 65_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WobbleSeverity {
    Mild,
    Realistic,
    Extreme,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RamPackWobbleConfig {
    pub enabled: bool,
    pub frequency: f32,
    pub severity: WobbleSeverity,
}

impl Default for RamPackWobbleConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            frequency: 0.001,
            severity: WobbleSeverity::Realistic,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WobbleState {
    pub active: bool,
    pub remaining_cycles: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WobbleFault {
    DroppedWrites,
    CorruptedReads,
    AddressGlitch,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "RamPackData")]
pub struct RamPack {
    config: RamPackWobbleConfig,
    wobble_state: WobbleState,
    #[serde(with = "ram_bytes")]
    ram: Box<[u8; RAM_SIZE]>,
    rng: Cell<u64>,
    fault: WobbleFault,
    frame_cycles: u32,
    events: u64,
}

#[derive(Deserialize)]
struct RamPackData {
    config: RamPackWobbleConfig,
    wobble_state: WobbleState,
    #[serde(with = "ram_bytes")]
    ram: Box<[u8; RAM_SIZE]>,
    rng: Cell<u64>,
    fault: WobbleFault,
    frame_cycles: u32,
    events: u64,
}

impl TryFrom<RamPackData> for RamPack {
    type Error = &'static str;

    fn try_from(data: RamPackData) -> Result<Self, Self::Error> {
        if !data.config.frequency.is_finite()
            || !(0.0..=1.0).contains(&data.config.frequency)
            || data.frame_cycles >= FRAME_CYCLES
            || data.wobble_state.active != (data.wobble_state.remaining_cycles > 0)
            || (data.wobble_state.active && !data.config.enabled)
        {
            return Err("invalid RAM Pack state");
        }
        Ok(Self {
            config: data.config,
            wobble_state: data.wobble_state,
            ram: data.ram,
            rng: data.rng,
            fault: data.fault,
            frame_cycles: data.frame_cycles,
            events: data.events,
        })
    }
}

mod ram_bytes {
    use super::RAM_SIZE;
    use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        bytes: &[u8; RAM_SIZE],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        bytes.as_slice().serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Box<[u8; RAM_SIZE]>, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        bytes
            .into_boxed_slice()
            .try_into()
            .map_err(|_| D::Error::custom("invalid RAM Pack size"))
    }
}

impl Default for RamPack {
    fn default() -> Self {
        Self::new()
    }
}

impl RamPack {
    pub fn new() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9e37_79b9_7f4a_7c15, |time| time.as_nanos() as u64);
        Self::with_seed(seed)
    }

    pub fn with_seed(seed: u64) -> Self {
        Self {
            config: RamPackWobbleConfig::default(),
            wobble_state: WobbleState {
                active: false,
                remaining_cycles: 0,
            },
            ram: Box::new([0; RAM_SIZE]),
            rng: Cell::new(seed),
            fault: WobbleFault::CorruptedReads,
            frame_cycles: 0,
            events: 0,
        }
    }

    fn random(&self) -> u64 {
        let mut x = self.rng.get().wrapping_add(0x9e37_79b9_7f4a_7c15);
        self.rng.set(x);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        x ^ (x >> 31)
    }

    /// Advance the pack at instruction boundaries. Event probability is per PAL frame.
    pub fn update(&mut self, mut cycles: u32) {
        if !self.config.enabled {
            return;
        }
        while cycles > 0 {
            let step = cycles.min(FRAME_CYCLES - self.frame_cycles);
            self.wobble_state.remaining_cycles =
                self.wobble_state.remaining_cycles.saturating_sub(step);
            self.wobble_state.active = self.wobble_state.remaining_cycles > 0;
            self.frame_cycles += step;
            cycles -= step;
            if self.frame_cycles == FRAME_CYCLES {
                self.frame_cycles = 0;
                let probability = (self.random() >> 40) as f32 / (1u32 << 24) as f32;
                if !self.wobble_state.active && probability < self.config.frequency {
                    self.trigger_wobble();
                }
            }
        }
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.read_bus(addr, true)
    }

    pub(crate) fn read_bus(&self, addr: u16, faults: bool) -> u8 {
        let active = faults && self.is_wobbling();
        if active && self.fault == WobbleFault::Disconnect {
            return 0xff;
        }
        let mut index = usize::from(addr & 0x3fff);
        if active && self.fault == WobbleFault::AddressGlitch && self.random().is_multiple_of(4) {
            index ^= 1 << (self.random() % 14);
        }
        let mut value = self.ram[index];
        if active && self.fault == WobbleFault::CorruptedReads && self.random().is_multiple_of(4) {
            let bit = self.random() % 8;
            value ^= 1 << bit;
            if self.random().is_multiple_of(4) {
                value ^= 1 << ((bit + 1 + self.random() % 7) % 8);
            }
        }
        value
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        self.write_bus(addr, value, true);
    }

    pub(crate) fn write_bus(&mut self, addr: u16, value: u8, faults: bool) {
        let active = faults && self.is_wobbling();
        if active
            && (self.fault == WobbleFault::Disconnect
                || (self.fault == WobbleFault::DroppedWrites && self.random().is_multiple_of(4)))
        {
            return;
        }
        let mut index = usize::from(addr & 0x3fff);
        if active && self.fault == WobbleFault::AddressGlitch && self.random().is_multiple_of(4) {
            index ^= 1 << (self.random() % 14);
        }
        self.ram[index] = value;
    }

    pub fn trigger_wobble(&mut self) {
        if !self.config.enabled {
            return;
        }
        let fault = match self.random() % 100 {
            0..=32 => WobbleFault::DroppedWrites,
            33..=65 => WobbleFault::CorruptedReads,
            66..=97 => WobbleFault::AddressGlitch,
            _ => WobbleFault::Disconnect,
        };
        self.trigger_fault(fault);
    }

    /// Trigger a specific connector fault, useful for reproducible diagnostics.
    pub fn trigger_fault(&mut self, fault: WobbleFault) {
        if !self.config.enabled {
            return;
        }
        let (min, max) = match self.config.severity {
            WobbleSeverity::Mild => (50, 100),
            WobbleSeverity::Realistic => (100, 500),
            WobbleSeverity::Extreme => (500, 2000),
        };
        self.fault = fault;
        self.wobble_state = WobbleState {
            active: true,
            remaining_cycles: min + (self.random() % u64::from(max - min + 1)) as u32,
        };
        self.events = self.events.wrapping_add(1);
    }

    pub fn is_wobbling(&self) -> bool {
        self.config.enabled && self.wobble_state.active
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.config.enabled = enabled;
        if !enabled {
            self.wobble_state.active = false;
            self.wobble_state.remaining_cycles = 0;
        }
    }

    pub fn set_severity(&mut self, severity: WobbleSeverity) {
        self.config.severity = severity;
    }

    pub fn set_frequency(&mut self, frequency: f32) -> Result<(), &'static str> {
        if !frequency.is_finite() || !(0.0..=1.0).contains(&frequency) {
            return Err("wobble frequency must be a probability between 0 and 1");
        }
        self.config.frequency = frequency;
        Ok(())
    }

    pub fn config(&self) -> &RamPackWobbleConfig {
        &self.config
    }

    pub fn state(&self) -> &WobbleState {
        &self.wobble_state
    }

    pub(crate) fn clear(&mut self) {
        self.ram.fill(0);
    }

    pub(crate) fn events(&self) -> u64 {
        self.events
    }
}
