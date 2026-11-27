use std::{
    collections::VecDeque,
    sync::{mpsc, Arc, Mutex},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    FromSample, SampleFormat, SizedSample, Stream, StreamConfig,
};

pub struct TapeAudio {
    _stream: Stream,
    samples: Arc<Mutex<VecDeque<f32>>>,
    errors: mpsc::Receiver<String>,
    sample_rate: u32,
}

impl TapeAudio {
    pub fn open() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no audio output device is available")?;
        let supported = device
            .default_output_config()
            .map_err(|error| error.to_string())?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.into();
        let samples = Arc::new(Mutex::new(VecDeque::new()));
        let (sender, errors) = mpsc::channel();
        let stream = match format {
            SampleFormat::I8 => build_stream::<i8>(&device, &config, &samples, sender),
            SampleFormat::F32 => build_stream::<f32>(&device, &config, &samples, sender),
            SampleFormat::I16 => build_stream::<i16>(&device, &config, &samples, sender),
            SampleFormat::I32 => build_stream::<i32>(&device, &config, &samples, sender),
            SampleFormat::I64 => build_stream::<i64>(&device, &config, &samples, sender),
            SampleFormat::U8 => build_stream::<u8>(&device, &config, &samples, sender),
            SampleFormat::U16 => build_stream::<u16>(&device, &config, &samples, sender),
            SampleFormat::U32 => build_stream::<u32>(&device, &config, &samples, sender),
            SampleFormat::U64 => build_stream::<u64>(&device, &config, &samples, sender),
            SampleFormat::F64 => build_stream::<f64>(&device, &config, &samples, sender),
            other => return Err(format!("unsupported audio output format: {other:?}")),
        }?;
        stream.play().map_err(|error| error.to_string())?;
        Ok(Self {
            _stream: stream,
            samples,
            errors,
            sample_rate: config.sample_rate.0,
        })
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn enqueue(&self, samples: impl IntoIterator<Item = f32>) -> Result<(), String> {
        let mut queue = self.samples.lock().map_err(|error| error.to_string())?;
        queue.extend(samples);
        if queue.len() > self.sample_rate as usize / 2 {
            return Err("tape audio could not keep up with emulation".to_owned());
        }
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.samples
            .lock()
            .map_err(|error| error.to_string())?
            .clear();
        Ok(())
    }

    pub fn error(&self) -> Option<String> {
        self.errors.try_recv().ok()
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: &Arc<Mutex<VecDeque<f32>>>,
    errors: mpsc::Sender<String>,
) -> Result<Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let samples = Arc::clone(samples);
    let channels = usize::from(config.channels);
    let callback_errors = errors.clone();
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                let mut queue = match samples.lock() {
                    Ok(queue) => queue,
                    Err(error) => {
                        let _ = callback_errors.send(error.to_string());
                        output.fill(T::from_sample(0.0));
                        return;
                    }
                };
                fill_output(output, channels, &mut queue);
            },
            move |error| {
                let _ = errors.send(error.to_string());
            },
            None,
        )
        .map_err(|error| error.to_string())
}

fn fill_output<T: SizedSample + FromSample<f32>>(
    output: &mut [T],
    channels: usize,
    queue: &mut VecDeque<f32>,
) {
    for frame in output.chunks_mut(channels) {
        let value = T::from_sample(queue.pop_front().unwrap_or(0.0));
        frame.fill(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_repeats_mono_samples_for_each_channel_and_silences_underruns() {
        let mut queue = VecDeque::from([0.15, -0.15]);
        let mut output = [1.0_f32; 6];
        fill_output(&mut output, 2, &mut queue);
        assert_eq!(output, [0.15, 0.15, -0.15, -0.15, 0.0, 0.0]);
        assert!(queue.is_empty());
    }

    #[test]
    #[ignore = "requires an audio output device"]
    fn output_stream_starts_and_stops() {
        let audio = TapeAudio::open().unwrap();
        audio.enqueue([0.0, 0.15, -0.15]).unwrap();
        audio.clear().unwrap();
        assert!(audio.error().is_none());
    }

    #[test]
    #[ignore = "requires an intentionally unavailable audio output device"]
    fn unavailable_output_is_reported() {
        assert!(TapeAudio::open().is_err());
    }
}
