use log::error;
use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, windows::hann_window, FrequencyLimit};
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

// TODO:
// use ringbuffer
// Smoothing
// dynamic sample rate
// do proper error handling instead of recv_timeout
// extract loop into process() procedure.
// Use egui inbox.
// don't normalize using the max. this makes it so there is always one bar==1, that is, max height.

pub const NUM_BUCKETS: usize = 8;
const FFT_SIZE: usize = 1 << 9; // 512
const SAMPLE_RATE: f32 = 44100.0;

//   The visualizer pipeline is comprised of three components:
//   1. A source wrapper that captures audio samples from the audio stream.
//   2. A processing thread that receives the samples and performs FFT and other processing.
//   3. Visualization UI code in the main thread that displays the processed data.
//
//   Communication between the components is achieved using channels.
pub fn start_visualizer_pipeline() -> (mpsc::Sender<f32>, mpsc::Receiver<Vec<f32>>) {
    let (sample_sender, sample_receiver) = mpsc::channel::<f32>();
    let (fft_output_sender, fft_output_receiver) = mpsc::channel::<Vec<f32>>();

    thread::spawn(move || {
        let mut samples = Vec::with_capacity(FFT_SIZE);

        loop {
            if let Ok(sample) = sample_receiver.recv_timeout(Duration::from_millis(10)) {
                samples.push(sample);
            }

            if samples.len() == FFT_SIZE {
                let hann_window = hann_window(&samples);

                let spectrum = samples_fft_to_spectrum(
                    &hann_window,
                    SAMPLE_RATE as u32, // For now this is hardcoded. In the future this should be dynamic.
                    FrequencyLimit::All,
                    Some(&divide_by_N_sqrt),
                )
                .unwrap();

                let spectrum: Vec<f32> = spectrum.data().iter().map(|(_, mag)| mag.val()).collect();
                let spectrum_length = spectrum.len();

                let mut buckets = Vec::with_capacity(NUM_BUCKETS);

                let min_frequency: f32 = 20.0; // 20 Hz is roughly the lower limit of human hearing
                let nyquist_frequency: f32 = SAMPLE_RATE / 2.0;

                // Apply log-spacing. This helps avoid the left side of the spectrum dominating visually.
                let log_min = min_frequency.ln();
                let log_max = nyquist_frequency.ln();
                let log_step = (log_max - log_min) / NUM_BUCKETS as f32;

                for i in 0..NUM_BUCKETS {
                    let bucket_start_freq = (log_min + i as f32 * log_step).exp();
                    let bucket_end_freq = (log_min + (i + 1) as f32 * log_step).exp();

                    let start_bin = ((bucket_start_freq / (SAMPLE_RATE / 2.0)) * spectrum_length as f32).floor() as usize;
                    let end_bin = ((bucket_end_freq / (SAMPLE_RATE / 2.0)) * spectrum_length as f32).ceil() as usize;

                    let slice = &spectrum[start_bin.min(spectrum_length)..end_bin.min(spectrum_length)];
                    let avg = if !slice.is_empty() {
                        slice.iter().sum::<f32>() / slice.len() as f32
                    } else {
                        0.0
                    };

                    buckets.push(avg);
                }

                // Normalize
                let max_val = buckets.iter().fold(0.0_f32, |max, &val| max.max(val));
                if max_val > 0.0 {
                    for value in &mut buckets {
                        *value /= max_val;
                    }
                }

                let result = fft_output_sender.send(buckets);
                if result.is_err() {
                    error!("Failed to send FFT output. Closing thread.");
                    return;
                }

                samples.clear();
            }
        }
    });

    (sample_sender, fft_output_receiver)
}

pub fn visualizer_source<I>(input: I, sample_sender: Sender<f32>) -> VisualizerSource<I>
where
    I: Source,
{
    VisualizerSource { input, tx: sample_sender }
}

pub struct VisualizerSource<I> {
    input: I,
    tx: Sender<f32>, // single f32 samples now
}

impl<I> VisualizerSource<I> {
    /// Returns a reference to the inner source.
    #[inline]
    pub fn _inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn _inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    #[inline]
    pub fn _into_inner(self) -> I {
        self.input
    }
}

impl<I> Iterator for VisualizerSource<I>
where
    I: Source,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.input.next()?;

        // Send sample to the processing thread, ignore if channel is closed
        let _ = self.tx.send(sample);

        Some(sample)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for VisualizerSource<I> where I: Source + ExactSizeIterator {}

impl<I> Source for VisualizerSource<I>
where
    I: Source,
{
    #[inline]
    fn current_span_len(&self) -> Option<usize> {
        self.input.current_span_len()
    }

    #[inline]
    fn channels(&self) -> ChannelCount {
        self.input.channels()
    }

    #[inline]
    fn sample_rate(&self) -> SampleRate {
        self.input.sample_rate()
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }

    #[inline]
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.input.try_seek(pos)
    }
}
