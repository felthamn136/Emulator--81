use std::{error::Error, fmt, io, path::Path};

use super::{decoder::Decoder, filters::Statistics, pulse_detector::PulseDetector};

/// A WAV could not be opened, decoded, or interpreted as a ZX81 program.
#[derive(Debug)]
pub enum TapeError {
    UnsupportedWav,
    NoSignal,
    CorruptData(String),
    InvalidProgram(String),
    Io(io::Error),
    Wav(hound::Error),
}

impl TapeError {
    pub(super) fn invalid_signal(message: impl Into<String>) -> Self {
        Self::CorruptData(message.into())
    }

    fn wav(error: hound::Error) -> Self {
        match error {
            hound::Error::IoError(error) => Self::Io(error),
            error => Self::Wav(error),
        }
    }
}

impl fmt::Display for TapeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedWav => write!(f, "Unsupported WAV format. Use 8- or 16-bit PCM, mono or stereo, at 22050, 44100 or 48000 Hz"),
            Self::NoSignal => write!(f, "ZX81 tape leader not detected. Check the recording and its level"),
            Self::CorruptData(message) => write!(f, "Unable to decode ZX81 tape data: {message}"),
            Self::InvalidProgram(message) => write!(f, "Tape data recovered but program is invalid: {message}"),
            Self::Io(error) => write!(f, "Cannot read WAV file: {error}; check the file and permissions"),
            Self::Wav(error) => write!(f, "Cannot decode WAV file: {error}; use a valid PCM WAV"),
        }
    }
}

impl Error for TapeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Wav(error) => Some(error),
            _ => None,
        }
    }
}

/// Load a ZX81 RAM program image from a PCM WAV recording.
pub fn load_program(path: &Path) -> Result<Vec<u8>, TapeError> {
    load_program_with_progress(path, |_| {})
}

/// As [`load_program`], reporting coarse stages on the calling thread.
pub fn load_program_with_progress(
    path: &Path,
    mut progress: impl FnMut(&'static str),
) -> Result<Vec<u8>, TapeError> {
    progress("Loading WAV...");
    let mut reader = hound::WavReader::open(path).map_err(TapeError::wav)?;
    let spec = reader.spec();
    if spec.sample_format != hound::SampleFormat::Int
        || !matches!(spec.bits_per_sample, 8 | 16)
        || !matches!(spec.sample_rate, 22_050 | 44_100 | 48_000)
        || !matches!(spec.channels, 1 | 2)
    {
        return Err(TapeError::UnsupportedWav);
    }

    progress("Analysing Audio...");
    let channels = usize::from(spec.channels);
    let mut stats = Statistics::default();
    let mut count = 0usize;
    let mut sum = 0i32;
    for sample in reader.samples::<i16>() {
        sum += i32::from(sample.map_err(TapeError::wav)?);
        count += 1;
        if count.is_multiple_of(channels) {
            stats.add((sum / channels as i32) as i16);
            sum = 0;
        }
    }
    if count == 0 || !count.is_multiple_of(channels) {
        return Err(TapeError::invalid_signal(
            "WAV contains no complete audio frames; check the recording",
        ));
    }
    let (low, high) = stats
        .thresholds(spec.bits_per_sample)
        .ok_or(TapeError::NoSignal)?;

    progress("Detecting Tape Signal...");
    reader.seek(0).map_err(TapeError::Io)?;
    let mut detector = PulseDetector::new(low, high, spec.sample_rate)?;
    let mut decoder = Decoder::new();
    let mut announced = false;
    let mut sum = 0i32;
    for (index, sample) in reader.samples::<i16>().enumerate() {
        sum += i32::from(sample.map_err(TapeError::wav)?);
        if !(index + 1).is_multiple_of(channels) {
            continue;
        }
        let sample = (sum / channels as i32) as i16;
        sum = 0;
        if let Some(burst) = detector.sample(sample) {
            let result = decoder.push(burst)?;
            if decoder.started() && !announced {
                progress("Decoding Program...");
                announced = true;
            }
            if let Some(program) = result {
                return Ok(program);
            }
        }
    }
    if let Some(burst) = detector.finish() {
        if let Some(program) = decoder.push(burst)? {
            return Ok(program);
        }
    }
    if decoder.started() {
        Err(TapeError::invalid_signal(
            "incomplete ZX81 program; check the tape recording and try again",
        ))
    } else {
        Err(TapeError::NoSignal)
    }
}
