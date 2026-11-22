use super::{pulse_detector::Burst, tape_image::TapeImage, wav_loader::TapeError};

/// Streaming ZX81 burst decoder. Filename bytes are consumed, not returned.
pub struct Decoder {
    started: bool,
    byte: u8,
    bits: u8,
    image: TapeImage,
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            started: false,
            byte: 0,
            bits: 0,
            image: TapeImage::new(),
        }
    }

    /// Consume a burst and return the validated RAM image once complete.
    pub fn push(&mut self, burst: Burst) -> Result<Option<Vec<u8>>, TapeError> {
        if !self.started && burst.leading_gap < 0.010 {
            return Ok(None);
        }
        if self.started
            && !self.image.filename_done()
            && (burst.gap_seconds > 0.010 || !burst.valid_period || !matches!(burst.waves, 4 | 9))
        {
            *self = Self::new();
            return Ok(None);
        }
        if !burst.valid_period || !matches!(burst.waves, 4 | 9) {
            if self.started {
                return Err(TapeError::invalid_signal(
                    "invalid pulse burst; try a cleaner recording",
                ));
            }
            return Ok(None);
        }
        if burst.gap_seconds > 0.010 {
            if !self.started {
                return Ok(None);
            }
            return Err(TapeError::invalid_signal(
                "unexpected gap in tape data; check the recording",
            ));
        }
        self.started = true;
        self.byte = (self.byte << 1) | u8::from(burst.waves == 9);
        self.bits += 1;
        if self.bits != 8 {
            return Ok(None);
        }
        let byte = self.byte;
        self.byte = 0;
        self.bits = 0;
        self.image.push(byte)
    }

    pub fn started(&self) -> bool {
        self.started
    }
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}
