/// Mono signal statistics are accumulated without retaining the audio.
#[derive(Clone, Copy)]
pub(super) struct Statistics {
    count: u64,
    sum: f64,
    min: i16,
    max: i16,
}

impl Default for Statistics {
    fn default() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            min: i16::MAX,
            max: i16::MIN,
        }
    }
}

impl Statistics {
    pub(super) fn add(&mut self, sample: i16) {
        let value = f64::from(sample);
        self.count += 1;
        self.sum += value;
        self.min = self.min.min(sample);
        self.max = self.max.max(sample);
    }

    pub(super) fn thresholds(&self, bits: u16) -> Option<(f64, f64)> {
        let range = f64::from(self.max) - f64::from(self.min);
        if self.count == 0 || range < if bits == 8 { 8.0 } else { 512.0 } {
            return None;
        }
        let centre = self.sum / self.count as f64;
        let hysteresis = range * 0.15;
        Some((centre - hysteresis, centre + hysteresis))
    }
}
