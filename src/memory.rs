use std::{cell::Cell, io};

use z80::Z80_io;

pub const ROM_SIZE: usize = 0x2000;
pub const RAM_SIZE: usize = 0x4000;

/// 8K ROM and a 16K RAM pack, including their standard address mirrors.
pub struct Memory {
    rom: Box<[u8; ROM_SIZE]>,
    ram: Box<[u8; RAM_SIZE]>,
    keyboard: [u8; 8],
    nmi_enabled: bool,
    tape_input: bool,
    display_fetch: Cell<bool>,
}

impl Memory {
    pub fn new(rom: &[u8]) -> io::Result<Self> {
        let rom: [u8; ROM_SIZE] = rom.try_into().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "ZX81 ROM must be exactly 8192 bytes",
            )
        })?;
        Ok(Self {
            rom: Box::new(rom),
            ram: Box::new([0; RAM_SIZE]),
            keyboard: [0x1f; 8],
            nmi_enabled: false,
            tape_input: false,
            display_fetch: Cell::new(false),
        })
    }

    pub fn read8(&self, address: u16) -> u8 {
        let address = address & 0x7fff;
        if address < 0x4000 {
            self.rom[usize::from(address & 0x1fff)]
        } else {
            self.ram[usize::from(address - 0x4000)]
        }
    }

    pub fn write8(&mut self, address: u16, value: u8) {
        let address = address & 0x7fff;
        if address >= 0x4000 {
            self.ram[usize::from(address - 0x4000)] = value;
        }
    }

    pub fn clear_ram(&mut self) {
        self.ram.fill(0);
    }

    /// Set a key in the ZX81's active-low eight-row, five-column matrix.
    pub fn set_key(&mut self, row: usize, column: usize, pressed: bool) -> io::Result<()> {
        if row >= 8 || column >= 5 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "ZX81 keyboard row must be 0..8 and column 0..5",
            ));
        }
        let bit = 1 << column;
        if pressed {
            self.keyboard[row] &= !bit;
        } else {
            self.keyboard[row] |= bit;
        }
        Ok(())
    }

    pub fn clear_keyboard(&mut self) {
        self.keyboard.fill(0x1f);
    }

    pub fn nmi_enabled(&self) -> bool {
        self.nmi_enabled
    }

    pub fn reset_hardware(&mut self) {
        self.nmi_enabled = false;
        self.tape_input = false;
        self.display_fetch.set(false);
        self.clear_keyboard();
    }

    pub fn set_tape_input(&mut self, high: bool) {
        self.tape_input = high;
    }

    pub fn take_display_fetch(&self) -> bool {
        self.display_fetch.replace(false)
    }
}

impl Z80_io for Memory {
    fn read_byte(&self, address: u16) -> u8 {
        self.read8(address)
    }

    fn read_opcode(&self, address: u16) -> u8 {
        let value = self.read8(address);
        if address & 0x8000 != 0 {
            if value & 0x40 == 0 {
                self.display_fetch.set(true);
                return 0;
            }
            if value == 0x76 {
                self.display_fetch.set(true);
            }
        }
        value
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.write8(address, value);
    }

    fn port_in(&self, address: u16) -> u8 {
        if address & 1 != 0 {
            return 0xff;
        }
        let selected = !(address >> 8) as u8;
        let mut keys = 0x1f;
        for (row, value) in self.keyboard.iter().enumerate() {
            if selected & (1 << row) != 0 {
                keys &= value;
            }
        }
        0x60 | if self.tape_input { 0x80 } else { 0 } | keys
    }

    fn port_out(&mut self, address: u16, _value: u8) {
        if address & 1 == 0 {
            self.nmi_enabled = true;
        } else if address & 2 == 0 {
            self.nmi_enabled = false;
        }
    }

    fn irq_active(&self, refresh: u8) -> bool {
        refresh & 0x40 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rom_is_validated_mirrored_and_read_only() {
        assert!(Memory::new(&[0; ROM_SIZE - 1]).is_err());
        assert!(Memory::new(&[0; ROM_SIZE + 1]).is_err());
        let mut rom = [0; ROM_SIZE];
        rom[0x1234] = 0xa5;
        let mut memory = Memory::new(&rom).unwrap();
        for address in [0x1234, 0x3234, 0x9234, 0xb234] {
            memory.write8(address, 0xff);
            assert_eq!(memory.read8(address), 0xa5);
        }
    }

    #[test]
    fn ram_is_mirrored_and_cleared() {
        let mut memory = Memory::new(&[0; ROM_SIZE]).unwrap();
        memory.write8(0x4000, 0x12);
        memory.write8(0xffff, 0x34);
        assert_eq!(memory.read8(0xc000), 0x12);
        assert_eq!(memory.read8(0x7fff), 0x34);
        memory.clear_ram();
        assert_eq!(memory.read8(0x4000), 0);
        assert_eq!(memory.read8(0xffff), 0);
    }

    #[test]
    fn keyboard_reads_selected_rows_as_active_low() {
        let mut memory = Memory::new(&[0; ROM_SIZE]).unwrap();
        memory.set_key(0, 0, true).unwrap();
        memory.set_key(1, 2, true).unwrap();
        assert_eq!(memory.port_in(0xfefe), 0x7e);
        assert_eq!(memory.port_in(0xfdfe), 0x7b);
        assert_eq!(memory.port_in(0xfcfe), 0x7a);
        assert_eq!(memory.port_in(0xfffe), 0x7f);
        assert_eq!(memory.port_in(0xfcff), 0xff);
        assert!(memory.set_key(8, 0, true).is_err());
        assert!(memory.set_key(0, 5, true).is_err());
        memory.clear_keyboard();
        assert_eq!(memory.port_in(0xfcfe), 0x7f);
        memory.set_tape_input(true);
        assert_eq!(memory.port_in(0xfcfe), 0xff);
    }

    #[test]
    fn every_keyboard_position_is_read_from_its_selected_row() {
        let mut memory = Memory::new(&[0; ROM_SIZE]).unwrap();
        for row in 0..8 {
            for column in 0..5 {
                memory.set_key(row, column, true).unwrap();
                for selected_row in 0..8 {
                    let port = (u16::from(!(1u8 << selected_row)) << 8) | 0xfe;
                    let expected = if selected_row == row {
                        0x7f & !(1 << column)
                    } else {
                        0x7f
                    };
                    assert_eq!(memory.port_in(port), expected, "key ({row}, {column})");
                }
                memory.set_key(row, column, false).unwrap();
            }
        }
    }

    #[test]
    fn opcode_fetches_are_distinct_from_data_reads() {
        let mut memory = Memory::new(&[0; ROM_SIZE]).unwrap();
        memory.write8(0x4000, 0x3e);
        memory.write8(0x4001, 0x76);
        assert_eq!(memory.read_byte(0xc000), 0x3e);
        assert_eq!(memory.read_opcode(0xc000), 0);
        assert_eq!(memory.read_opcode(0x4000), 0x3e);
        assert_eq!(memory.read_opcode(0xc001), 0x76);
    }

    #[test]
    fn ports_control_nmi_and_refresh_controls_irq() {
        let mut memory = Memory::new(&[0; ROM_SIZE]).unwrap();
        memory.port_out(0x00fe, 0);
        assert!(memory.nmi_enabled());
        memory.port_out(0x00ff, 0);
        assert!(memory.nmi_enabled());
        memory.port_out(0x00fd, 0);
        assert!(!memory.nmi_enabled());
        assert!(memory.irq_active(0x80));
        assert!(!memory.irq_active(0xc0));
    }
}
