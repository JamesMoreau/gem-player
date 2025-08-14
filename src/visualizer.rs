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
// cache fft planner
// Is the other half of fft still being processed?
// use process_with_scratch fft.
// Smoothing. needs to be stateful.

const FFT_SIZE: usize = 1 << 9; // 512
pub const NUM_BUCKETS: usize = 12;

//   The visualizer pipeline is comprised of three components:
//   1. A source wrapper that captures audio samples from the audio stream.
//   2. A processing thread that receives the samples and performs FFT and other processing.
//   3. Visualization UI code in the main thread that displays the processed data.
//
//   Communication between the components is achieved using channels.
pub fn start_visualizer_pipeline() -> (mpsc::Sender<f32>, mpsc::Receiver<[f32; NUM_BUCKETS]>) {
    let (sample_sender, sample_receiver) = mpsc::channel::<f32>();
    let (fft_output_sender, fft_output_receiver) = mpsc::channel::<[f32; NUM_BUCKETS]>();

    thread::spawn(move || {
        let mut buffer = [0f32; FFT_SIZE];
        let mut sample_count = 0;
        let hann_window = hann_window::<FFT_SIZE>();

        loop {
            if let Ok(sample) = sample_receiver.recv_timeout(Duration::from_millis(10)) {
                buffer[sample_count] = sample;
                sample_count += 1;
            }

            if sample_count == FFT_SIZE {
                let fft_result = analyse(&buffer, &hann_window);
                let _ = fft_output_sender.send(fft_result);
                sample_count = 0;
            }
        }
    });

    (sample_sender, fft_output_receiver)
}

// Algorithm implementation taken from tsoding: https://github.com/tsoding/musializer
fn analyse(samples: &[f32; FFT_SIZE], hann_window: &[f32; FFT_SIZE]) -> [f32; NUM_BUCKETS] {
    // Apply Hann window on the input.
    let mut buffer: Vec<Complex<f32>> = samples
        .iter()
        .zip(hann_window.iter())
        .map(|(&sample, &hann)| Complex {
            re: sample * hann,
            im: 0.0,
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(buffer.len());
    fft.process(&mut buffer);

    // Apply a logarithmic scale.
    let half = buffer.len() / 2;
    let mut bars = Vec::new();
    let mut max_amplitude = 1.0_f32;
    let step = 1.06_f32;
    let mut f = 1.0_f32;

    while (f as usize) < half {
        let f1 = (f * step).ceil();
        let start_bin = f as usize;
        let end_bin = f1.min(half as f32) as usize;

        let mut max_val = 0.0_f32;
        buffer.iter().skip(start_bin).take(end_bin - start_bin).for_each(|c| {
            let mag = (c.re.powi(2) + c.im.powi(2)).sqrt();
            if mag > max_val {
                max_val = mag;
            }
        });

        if max_val > max_amplitude {
            max_amplitude = max_val;
        }

        bars.push(max_val);
        f = f1;
    }

    // Normalize.
    if max_amplitude > 0.0 {
        for val in &mut bars {
            *val /= max_amplitude;
        }
    }

    // Sort into buckets by averaging.
    let bucket_size = bars.len() / NUM_BUCKETS;
    let mut buckets = [0.0; NUM_BUCKETS];

    for (i, bucket) in buckets.iter_mut().enumerate() {
        let start = i * bucket_size;

        let is_last_bucket = i == NUM_BUCKETS - 1;
        let end = if is_last_bucket { bars.len() } else { start + bucket_size };

        let slice = &bars[start..end];
        let avg = slice.iter().sum::<f32>() / slice.len() as f32;

        *bucket = avg;
    }

    buckets
}

fn hann_window<const N: usize>() -> [f32; N] {
    let mut array = [0.0f32; N];
    let size_f = (N - 1) as f32;
    let two_pi = 2.0 * PI;

    let mut i = 0;
    while i < N {
        let t = i as f32 / size_f;
        array[i] = 0.5 - 0.5 * (two_pi * t).cos();
        i += 1;
    }

    array
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
