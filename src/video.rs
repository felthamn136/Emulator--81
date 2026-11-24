use std::io;

use crate::{emulator::SCREEN_HEIGHT, emulator::SCREEN_WIDTH, memory::Memory};

const DISPLAY_FILE_POINTER: u16 = 0x400c;
const CHARACTER_BASE: u16 = 0x1e00;
const NEWLINE: u8 = 0x76;
const ROWS: usize = 24;
const COLUMNS: usize = 32;

/// Render a ZX81 display file using glyphs from the installed ROM.
pub fn render_display_file(memory: &Memory) -> io::Result<Vec<u8>> {
    let pointer = u16::from_le_bytes([
        memory.read8(DISPLAY_FILE_POINTER),
        memory.read8(DISPLAY_FILE_POINTER + 1),
    ]);
    let mut address = usize::from(pointer);
    if !(0x4000..0x8000).contains(&address) || memory.read8(pointer) != NEWLINE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ZX81 display file does not start with a RAM newline marker",
        ));
    }
    address += 1;
    let mut pixels = vec![255; SCREEN_WIDTH * SCREEN_HEIGHT];
    for row in 0..ROWS {
        let mut terminated = false;
        for column in 0..=COLUMNS {
            if address >= 0x8000 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ZX81 display file extends beyond installed RAM",
                ));
            }
            let code = memory.read8(address as u16);
            address += 1;
            if code == NEWLINE {
                terminated = true;
                break;
            }
            if column == COLUMNS || code & 0x40 != 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid character or missing newline in ZX81 display file",
                ));
            }
            for scanline in 0..8 {
                let glyph_address = CHARACTER_BASE + u16::from(code & 0x3f) * 8 + scanline as u16;
                let glyph = memory.read8(glyph_address);
                for bit in 0..8 {
                    let mut ink = glyph & (0x80 >> bit) != 0;
                    if code & 0x80 != 0 {
                        ink = !ink;
                    }
                    let x = column * 8 + bit;
                    let y = row * 8 + scanline;
                    pixels[y * SCREEN_WIDTH + x] = if ink { 0 } else { 255 };
                }
            }
        }
        if !terminated {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "ZX81 display row has no newline",
            ));
        }
    }
    Ok(pixels)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::ROM_SIZE;

    fn memory_with_display() -> Memory {
        let mut rom = vec![0; ROM_SIZE];
        rom[usize::from(CHARACTER_BASE) + 8] = 0b1010_0000;
        let mut memory = Memory::new(&rom).unwrap();
        memory.write8(0x400c, 0x00);
        memory.write8(0x400d, 0x41);
        memory.write8(0x4100, NEWLINE);
        for row in 0..ROWS {
            memory.write8(0x4101 + row as u16, NEWLINE);
        }
        memory
    }

    #[test]
    fn uses_rom_glyphs_and_inverse_video() {
        let mut memory = memory_with_display();
        memory.write8(0x4101, 1);
        memory.write8(0x4102, 0x81);
        for row in 0..ROWS {
            memory.write8(0x4103 + row as u16, NEWLINE);
        }
        let pixels = render_display_file(&memory).unwrap();
        assert_eq!(&pixels[..4], &[0, 255, 0, 255]);
        assert_eq!(&pixels[8..12], &[255, 0, 255, 0]);
        assert_eq!(pixels[(SCREEN_HEIGHT - 1) * SCREEN_WIDTH], 255);
    }

    #[test]
    fn handles_full_width_rows_and_the_last_row() {
        let mut memory = memory_with_display();
        let mut address = 0x4101;
        for _ in 0..32 {
            memory.write8(address, 1);
            address += 1;
        }
        memory.write8(address, NEWLINE);
        address += 1;
        for _ in 0..22 {
            memory.write8(address, NEWLINE);
            address += 1;
        }
        memory.write8(address, 1);
        memory.write8(address + 1, NEWLINE);
        let pixels = render_display_file(&memory).unwrap();
        assert_eq!(pixels[31 * 8], 0);
        assert_eq!(pixels[23 * 8 * SCREEN_WIDTH], 0);
    }

    #[test]
    fn rejects_bad_pointers_and_unterminated_rows() {
        let mut memory = memory_with_display();
        memory.write8(0x4100, 0);
        assert!(render_display_file(&memory).is_err());
        memory.write8(0x4100, NEWLINE);
        memory.write8(0x4101, 0x40);
        assert!(render_display_file(&memory).is_err());
        for address in 0x4101..0x4122 {
            memory.write8(address, 1);
        }
        assert!(render_display_file(&memory).is_err());

        memory.write8(0x400c, 0xff);
        memory.write8(0x400d, 0x7f);
        memory.write8(0x7fff, NEWLINE);
        assert!(render_display_file(&memory).is_err());
    }
}
