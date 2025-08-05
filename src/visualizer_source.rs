use std::time::Duration;

use log::info;
use rodio::{source::SeekError, ChannelCount, SampleRate, Source};

/// Internal function that builds a `Visualizer` object.
pub fn visualizer<I>(input: I) -> Visualizer<I>
where
    I: Source,
{
    Visualizer { input }
}

#[derive(Clone, Debug)]
pub struct Visualizer<I> {
    input: I,
}

impl<I> Visualizer<I> {
    /// Returns a reference to the inner source.
    #[inline]
    pub fn inner(&self) -> &I {
        &self.input
    }

    /// Returns a mutable reference to the inner source.
    #[inline]
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.input
    }

    /// Returns the inner source.
    #[inline]
    pub fn into_inner(self) -> I {
        self.input
    }
}

impl<I> Iterator for Visualizer<I>
where
    I: Source,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // TODO: send data to another thread for display.
        // For now, just print the time.
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        let seconds = duration.as_secs() % 60;
        let minutes = (duration.as_secs() / 60) % 60;
        let hours = duration.as_secs() / 3600;
        info!("sample timestamp {:02}:{:02}:{:02}", hours, minutes, seconds);

        self.input.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.input.size_hint()
    }
}

impl<I> ExactSizeIterator for Visualizer<I> where I: Source + ExactSizeIterator {}

impl<I> Source for Visualizer<I>
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
