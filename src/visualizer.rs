use log::info;
use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use rustfft::{num_complex::Complex, FftPlanner};
use std::{
    f32::consts::{PI, SQRT_2},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Duration,
};

const FFT_SIZE: usize = 1 << 10; // 1024
pub const CENTER_FREQUENCIES: [f32; 7] = [63.0, 125.0, 250.0, 500.0, 1000.0, 3000.0, 8000.0];

pub enum VisualizerCommand {
    Sample(f32),
    SampleRate(f32),
    Shutdown,
}

//   The visualizer pipeline is comprised of three components:
//   1. A source wrapper that captures audio samples from the audio stream.
//   2. A processing thread that receives the samples, performs FFT, and performs other processing.
//   3. Visualization UI code in the main thread that displays the processed data.

pub fn setup_visualizer_pipeline() -> (Sender<VisualizerCommand>, Receiver<Vec<f32>>) {
    let (command_sender, commands_receiver) = channel::<VisualizerCommand>();
    let (bands_sender, bands_receiver) = channel::<Vec<f32>>();

    thread::spawn(move || {
        let mut sample_rate = 44100.0;
        let mut samples = Vec::with_capacity(FFT_SIZE);

        loop {
            match commands_receiver.recv() {
                Ok(VisualizerCommand::Sample(sample)) => {
                    samples.push(sample);

                    if samples.len() == FFT_SIZE {
                        let half_octave_bandwidth = SQRT_2;
                        let bands = process_samples(&samples, sample_rate as u32, &CENTER_FREQUENCIES, half_octave_bandwidth);

                        let result = bands_sender.send(bands);
                        if result.is_err() {
                            info!("Processing receiver dropped. Shutting down the visualizer pipeline.");
                            return;
                        }

                        samples.clear();
                    }
                }
                Ok(VisualizerCommand::SampleRate(sr)) => {
                    sample_rate = sr;
                    samples.clear();
                }
                Ok(VisualizerCommand::Shutdown) => {
                    info!("Received shutdown message. Shutting down the visualizer pipeline.");
                    return;
                }
                Err(_) => {
                    info!("Message channel dropped. Shutting down the visualizer pipeline.");
                    return;
                }
            }
        }
    });

    (command_sender, bands_receiver)
}

pub fn process_samples(samples: &[f32], sample_rate: u32, band_center_frequencies: &[f32], bandwidth: f32) -> Vec<f32> {
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

    let mut bands = vec![0.0f32; band_center_frequencies.len()];

    for (b, &center) in band_center_frequencies.iter().enumerate() {
        let f_start = center / bandwidth;
        let f_end = center * bandwidth;

        let i_min = ((f_start * n as f32) / sample_rate as f32).floor() as usize;
        let i_max = ((f_end * n as f32) / sample_rate as f32).ceil() as usize;
        let i_max = i_max.min(magnitudes.len() - 1); // clamp to available bins

        for mag in &magnitudes[i_min..=i_max] {
            bands[b] += mag * mag; // accumulate power
        }
    }

    for band in &mut bands {
        *band = band.sqrt().max(1e-10); // avoid log(0)
        *band = 20.0 * band.log10();
    }

    // Normalize 0..1
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
    (0..n)
        .map(|i| 0.5 * (1.0 - (2.0 * PI * i as f32 / (n as f32 - 1.0)).cos()))
        .collect()
}

pub fn visualizer_source<I>(input: I, sender: Sender<VisualizerCommand>) -> VisualizerSource<I>
where
    I: Source,
{
    VisualizerSource { input, sender }
}

pub struct VisualizerSource<I> {
    input: I,
    sender: Sender<VisualizerCommand>, // single f32 samples now
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
        let _ = self.sender.send(VisualizerCommand::Sample(sample));

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
