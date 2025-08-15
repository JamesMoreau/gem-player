use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use rustfft::{num_complex::Complex, FftPlanner};
use std::{
    f32::consts::PI,
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

// TODO: potential optimizations
// use ringbuffer

pub const NUM_BUCKETS: usize = 12;
const FFT_SIZE: usize = 1 << 9; // 512
const SMOOTHING_FACTOR: f32 = 0.6;

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
        let mut buffer = Vec::with_capacity(FFT_SIZE);
        let mut sample_count = 0;
        let mut previous_buckets = [0.0_f32; NUM_BUCKETS];

        loop {
            if let Ok(sample) = sample_receiver.recv_timeout(Duration::from_millis(10)) {
                buffer[sample_count] = sample;
                sample_count += 1;
            }

            if sample_count == FFT_SIZE {
                let buckets = analyze(&buffer, &mut previous_buckets);
                let _ = fft_output_sender.send(buckets.to_vec());
                sample_count = 0;
            }
        }
    });

    (sample_sender, fft_output_receiver)
}

// Algorithm implementation inspired by tsoding: https://github.com/tsoding/musializer
fn analyze(samples: &[f32], previous_buckets: &mut [f32; NUM_BUCKETS]) -> [f32; NUM_BUCKETS] {
    // Apply Hann window
    let window = hann_window(samples.len());
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .zip(&window)
        .map(|(&sample, &hann)| Complex::new(sample * hann, 0.0))
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    fft.process(&mut buffer);

    let norm_factor = 1.0 / (buffer.len() as f32).sqrt();
    let nyquist_bin = buffer.len() / 2;
    let amplitudes: Vec<f32> = buffer[..nyquist_bin]
        .iter()
        .map(|c| ((c.re * c.re + c.im * c.im).sqrt()) * norm_factor)
        .collect();

    // Sort into buckets
    let mut buckets = [0.0; NUM_BUCKETS];
    let bucket_size = amplitudes.len() / NUM_BUCKETS;
    for (i, bucket) in buckets.iter_mut().enumerate() {
        let start = i * bucket_size;

        let is_last_bucket = i == NUM_BUCKETS - 1;
        let end = if is_last_bucket { amplitudes.len() } else { start + bucket_size };
        let avg = amplitudes[start..end].iter().sum::<f32>() / (end - start) as f32;
        *bucket = avg;
    }

    // Smooth
    for i in 0..NUM_BUCKETS {
        buckets[i] = previous_buckets[i] * SMOOTHING_FACTOR + buckets[i] * (1.0 - SMOOTHING_FACTOR);
        previous_buckets[i] = buckets[i];
    }

    buckets
}

pub fn hann_window(n: usize) -> Vec<f32> {
    if n == 0 {
        return Vec::new();
    }

    let mut window = Vec::with_capacity(n);

    for i in 0..n {
        let multiplier = 0.5 - 0.5 * ((2.0 * PI * i as f32) / (n - 1) as f32).cos();
        window.push(multiplier);
    }

    window
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
