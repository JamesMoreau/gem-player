use egui_inbox::UiInbox;
use log::info;
use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use spectrum_analyzer::{
    samples_fft_to_spectrum,
    scaling::{divide_by_N, divide_by_N_sqrt, scale_20_times_log10},
    windows::hann_window,
    FrequencyLimit,
};
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

// TODO:
// use ringbuffer
// dynamic sample rate
// perhaps convert energy to decibels?

// is divide_by_N_sqrt the most applicable??
// Will using the wrong sample rate affect the results a lot?
// check for negatives?

pub const NUM_BANDS: usize = 8;
const FFT_SIZE: usize = 1 << 11; // 2048
const SAMPLE_RATE: f32 = 48000.0;

//   The visualizer pipeline is comprised of three components:
//   1. A source wrapper that captures audio samples from the audio stream.
//   2. A processing thread that receives the samples and performs FFT and other processing.
//   3. Visualization UI code in the main thread that displays the processed data.
pub fn start_visualizer_pipeline() -> (mpsc::Sender<f32>, UiInbox<Vec<f32>>) {
    let (sample_sender, sample_receiver) = mpsc::channel::<f32>();
    let processing_inbox: UiInbox<Vec<f32>> = UiInbox::new();
    let processing_sender = processing_inbox.sender();

    thread::spawn(move || {
        let mut samples = Vec::with_capacity(FFT_SIZE);

        loop {
            let result = sample_receiver.recv();
            if let Ok(sample) = result {
                samples.push(sample);
            } else {
                info!("Sample channel dropped. Shutting down the visualizer pipeline.");
                return;
            }

            if samples.len() == FFT_SIZE {
                let bands = process_samples(&samples, SAMPLE_RATE as u32, NUM_BANDS);

                let result = processing_sender.send(bands);
                if result.is_err() {
                    info!("Processing inbox dropped. Shutting down the visualizer pipeline.");
                    return;
                }

                // Keep half the samples for overlap. This improves continuity / smoothness.
                samples.drain(0..FFT_SIZE / 2);
            }
        }
    });

    (sample_sender, processing_inbox)
}

pub fn process_samples(samples: &[f32], sample_rate: u32, num_bands: usize) -> Vec<f32> {
    let windowed = hann_window(samples);

    let spectrum = samples_fft_to_spectrum(&windowed, sample_rate, FrequencyLimit::All, None).unwrap();

    let magnitudes: Vec<f32> = spectrum.data().iter().map(|(_, mag)| mag.val()).collect();

    let nyquist = sample_rate as f32 / 2.0;
    let fft_size = magnitudes.len();

    let mut bands = vec![0.0; num_bands];

    // Determing band boundaries
    let f_min = 31.25;
    let f_max = nyquist;
    let mut band_boundaries = Vec::with_capacity(num_bands + 1);
    for b in 0..=num_bands {
        let frac = b as f32 / num_bands as f32;
        let f = f_min * (f_max / f_min).powf(frac);
        band_boundaries.push(f);
    }

    // Group FFT bins into bands
    for (i, &mag) in magnitudes.iter().enumerate() {
        let freq = (i as f32 / fft_size as f32) * nyquist;
        if freq < f_min {
            continue;
        }

        // Find which band this frequency belongs to
        for b in 0..num_bands {
            if freq >= band_boundaries[b] && freq < band_boundaries[b + 1] {
                bands[b] += mag * mag;
                break;
            }
        }
    }

    // Convert accumulated power to RMS and dB
    for band in &mut bands {
        *band = (*band).sqrt().max(1e-10); // avoid log(0)
        *band = 20.0 * band.log10();
    }

    // Normalize 0..1
    let max_band = bands.iter().fold(0.0f32, |a, &b| a.max(b));
    if max_band > 0.0 {
        for val in &mut bands {
            *val /= max_band;
        }
    }

    bands
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
