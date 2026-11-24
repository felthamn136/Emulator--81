use super::wav_loader::TapeError;
use super::{Tape, END_POINTER_OFFSET, START};

pub(super) struct TapeImage {
    filename_len: usize,
    filename_done: bool,
    bytes: Vec<u8>,
    expected: Option<usize>,
}

impl TapeImage {
    pub(super) fn new() -> Self {
        Self {
            filename_len: 0,
            filename_done: false,
            bytes: Vec::new(),
            expected: None,
        }
    }

    pub(super) fn push(&mut self, byte: u8) -> Result<Option<Vec<u8>>, TapeError> {
        if !self.filename_done {
            self.filename_len += 1;
            self.filename_done = byte & 0x80 != 0;
            if self.filename_len > 128 {
                return Err(TapeError::invalid_signal(
                    "unterminated ZX81 filename; check the recording",
                ));
            }
            return Ok(None);
        }
        self.bytes.push(byte);
        if self.bytes.len() == END_POINTER_OFFSET + 2 {
            let end = usize::from(u16::from_le_bytes([
                self.bytes[END_POINTER_OFFSET],
                self.bytes[END_POINTER_OFFSET + 1],
            ]));
            if !(0x407d..=0x8000).contains(&end) {
                return Err(TapeError::InvalidProgram(
                    "invalid end address; check the recording".into(),
                ));
            }
            self.expected = Some(end - START);
        }
        if self.expected == Some(self.bytes.len()) {
            Tape::from_p(&self.bytes)
                .map_err(|error| TapeError::InvalidProgram(error.to_string()))?;
            let display = usize::from(u16::from_le_bytes([self.bytes[3], self.bytes[4]]));
            let end = START + self.bytes.len();
            if !(0x407d..end).contains(&display) {
                return Err(TapeError::InvalidProgram(
                    "invalid display-file pointer; check the recording".into(),
                ));
            }
            return Ok(Some(std::mem::take(&mut self.bytes)));
        }
        Ok(None)
    }

    pub(super) fn filename_done(&self) -> bool {
        self.filename_done
    }
}
