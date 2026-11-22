use super::wav_loader::TapeError;

/// Streaming detector for mono PCM samples and externally determined DC thresholds.
///
/// Feed frames in order; the sample rate supplied here is used for all timings.
pub struct PulseDetector {
    low: f64,
    high: f64,
    high_state: bool,
    last_edge: Option<u64>,
    waves: u8,
    sample_rate: u32,
    invalid_period: bool,
    frame: u64,
    leading_gap: f64,
}

/// A group of waves ending at an inter-bit gap.
pub struct Burst {
    pub waves: u8,
    pub valid_period: bool,
    pub gap_seconds: f64,
    /// Time before the first rising edge of this burst.
    pub leading_gap: f64,
}

impl PulseDetector {
    /// Thresholds use the same units as the i16 samples and must straddle the DC level.
    pub fn new(low: f64, high: f64, sample_rate: u32) -> Result<Self, TapeError> {
        if !low.is_finite() || !high.is_finite() || low >= high || sample_rate == 0 {
            return Err(TapeError::invalid_signal(
                "invalid detector thresholds or sample rate",
            ));
        }
        Ok(Self {
            low,
            high,
            high_state: false,
            last_edge: None,
            waves: 0,
            sample_rate,
            invalid_period: false,
            frame: 0,
            leading_gap: 0.0,
        })
    }

    /// Consume one sample, returning a burst when the next one starts.
    pub fn sample(&mut self, sample: i16) -> Option<Burst> {
        let index = self.frame;
        self.frame = self.frame.saturating_add(1);
        let sample = f64::from(sample);
        if self.high_state {
            if sample < self.low {
                self.high_state = false;
            }
            return None;
        }
        if sample <= self.high {
            return None;
        }
        self.high_state = true;
        let previous = self.last_edge.replace(index);
        let Some(previous) = previous else {
            self.waves = 1;
            self.leading_gap = index as f64 / f64::from(self.sample_rate);
            return None;
        };
        let seconds = (index - previous) as f64 / f64::from(self.sample_rate);
        if seconds > 0.00065 {
            let burst = Burst {
                waves: self.waves,
                valid_period: !self.invalid_period,
                gap_seconds: seconds,
                leading_gap: self.leading_gap,
            };
            self.waves = 1;
            self.invalid_period = false;
            self.leading_gap = seconds;
            Some(burst)
        } else {
            self.waves = self.waves.saturating_add(1);
            if !(0.00018..=0.00046).contains(&seconds) {
                self.invalid_period = true;
            }
            None
        }
    }

    /// Return the last burst when the recording ends after an adequate gap.
    pub fn finish(&self) -> Option<Burst> {
        let last = self.last_edge?;
        let seconds = (self.frame - last) as f64 / f64::from(self.sample_rate);
        if seconds < 0.00065 {
            return None;
        }
        Some(Burst {
            waves: self.waves,
            valid_period: !self.invalid_period,
            gap_seconds: 0.0,
            leading_gap: self.leading_gap,
        })
    }
}
