use egui_inbox::UiInbox;
use log::info;
use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use rustfft::{num_complex::Complex, FftPlanner};
use std::{
    f32::consts::PI,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

// TODO:
// use ringbuffer
// dynamic sample rate

pub const NUM_BANDS: usize = 8; // TODO: REMOVE
const FFT_SIZE: usize = 1 << 11; // 2048
const SAMPLE_RATE: f32 = 48000.0;

//   The visualizer pipeline is comprised of three components:
//   1. A source wrapper that captures audio samples from the audio stream.
//   2. A processing thread that receives the samples, performs FFT, and performs other processing.
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
                let bands = process_samples(&samples, SAMPLE_RATE as u32);

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

pub fn process_samples(samples: &[f32], sample_rate: u32) -> Vec<f32> {
    let n = samples.len();
    let window = hann_window(n);

    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .zip(window.iter())
        .map(|(&s, &w)| Complex { re: s * w, im: 0.0 })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(n);
    fft.process(&mut buffer);

    let magnitudes: Vec<f32> = buffer.iter().take(n / 2 + 1).map(|c| c.norm()).collect();

    let center_freqs = [63.0, 125.0, 500.0, 1000.0, 2000.0, 4000.0, 6000.0, 8000.0];
    let bandwidth = 1.414_f32; // 2^(1/2), half-octave

    let mut bands = vec![0.0f32; center_freqs.len()];

    for (b, &center) in center_freqs.iter().enumerate() {
        let f_start = center / bandwidth;
        let f_end = center * bandwidth;

        let i_min = ((f_start * n as f32) / sample_rate as f32).floor() as usize;
        let i_max = ((f_end * n as f32) / sample_rate as f32).ceil() as usize;
        let i_max = i_max.min(magnitudes.len() - 1); // clamp to available bins

        for mag in &magnitudes[i_min..=i_max] {
            bands[b] += mag * mag; // accumulate power
        }
    }

    // DOES KEIJIRO DO THIS??
    // --- Convert to RMS + dB --- 
    for band in &mut bands {
        *band = band.sqrt().max(1e-10); // RMS
        *band = 20.0 * band.log10(); // dB
    }

    // --- Normalize 0..1 ---
    if let Some(&max_band) = bands.iter().max_by(|a, b| a.partial_cmp(b).unwrap()) {
        if max_band > 0.0 {
            for band in &mut bands {
                *band /= max_band;
            }
        }
    }

    bands
}

pub fn hann_window(n: usize) -> Vec<f32> {
    let denominator = (n - 1) as f32;
    (0..n).map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / denominator).cos())).collect()
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
