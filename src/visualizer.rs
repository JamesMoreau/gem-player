use rodio::{source::SeekError, ChannelCount, SampleRate, Source};
use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

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
        let mut buffer = Vec::with_capacity(1024);

        loop {
            if let Ok(sample) = sample_receiver.recv_timeout(Duration::from_millis(10)) {
                buffer.push(sample);
            }

            if buffer.len() >= 1024 {
                let fft_result = process_fft(&buffer);
                let _ = fft_output_sender.send(fft_result);
                buffer.clear();
            }
        }
    });

    (sample_sender, fft_output_receiver)
}

fn process_fft(samples: &[f32]) -> Vec<f32> {
    println!("Processing FFT for {} samples", samples.len());
    // TODO: replace with real FFT processing
    samples.iter().map(|s| s.abs()).collect()
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
