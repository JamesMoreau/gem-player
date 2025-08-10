use std::{sync::mpsc::Sender, time::Duration};

use rodio::{source::SeekError, ChannelCount, SampleRate, Source};

/// Internal function that builds a `Visualizer` object.
const BUFFER_SIZE: usize = 1024;

pub fn visualizer_source<I>(input: I, tx: Sender<Vec<f32>>) -> VisualizerSource<I>
where
    I: Source,
{
    VisualizerSource {
        input,
        tx,
        buffer: Vec::with_capacity(BUFFER_SIZE),
    }
}

/// The `VisualizerSource` struct is a wrapper around a source that collects audio samples
/// and sends them to a channel for visualization purposes.
#[derive(Clone, Debug)]
pub struct VisualizerSource<I> {
    input: I,
    tx: Sender<Vec<f32>>,
    buffer: Vec<f32>,
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

        self.buffer.push(sample);

        if self.buffer.len() >= BUFFER_SIZE {
            let chunk = std::mem::take(&mut self.buffer); // efficient way to replace and send
            let _ = self.tx.send(chunk); // ignore if receiver is gone
        }

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
