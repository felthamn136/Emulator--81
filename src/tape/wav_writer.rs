use std::{
    borrow::Borrow,
    io::{self, BufWriter, Seek, SeekFrom, Write},
    path::Path,
};

use super::signal::SAMPLE_RATE;

/// Write a mono 16-bit PCM WAV atomically, consuming samples incrementally.
pub fn write_wav<I>(path: &Path, samples: I) -> io::Result<()>
where
    I: IntoIterator,
    I::Item: Borrow<i16>,
{
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut output = BufWriter::new(temporary.as_file_mut());
        output.write_all(&[0; 44])?;
        let mut buffer = [0u8; 8192];
        let mut used = 0;
        let mut data_bytes = 0u32;
        for sample in samples {
            data_bytes = data_bytes.checked_add(2).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "WAV data exceeds RIFF limits")
            })?;
            if data_bytes > u32::MAX - 36 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "WAV data exceeds RIFF limits",
                ));
            }
            buffer[used..used + 2].copy_from_slice(&sample.borrow().to_le_bytes());
            used += 2;
            if used == buffer.len() {
                output.write_all(&buffer)?;
                used = 0;
            }
        }
        output.write_all(&buffer[..used])?;
        output.seek(SeekFrom::Start(0))?;
        output.write_all(b"RIFF")?;
        output.write_all(&(36 + data_bytes).to_le_bytes())?;
        output.write_all(b"WAVEfmt ")?;
        output.write_all(&16u32.to_le_bytes())?;
        output.write_all(&1u16.to_le_bytes())?;
        output.write_all(&1u16.to_le_bytes())?;
        output.write_all(&SAMPLE_RATE.to_le_bytes())?;
        output.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?;
        output.write_all(&2u16.to_le_bytes())?;
        output.write_all(&16u16.to_le_bytes())?;
        output.write_all(b"data")?;
        output.write_all(&data_bytes.to_le_bytes())?;
        output.flush()?;
    }
    temporary.persist(path).map_err(|error| error.error)?;
    Ok(())
}
