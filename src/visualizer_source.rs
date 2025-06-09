use egui_inbox::UiInboxSender;
use rodio::{Sample, Source};

pub struct VisualizerSource<S>
where
    S: Source,
    S::Item: Sample,
{
    inner: S,
    tx: UiInboxSender<f32>,
}

impl<S> VisualizerSource<S>
where
    S: Source,
    S::Item: rodio::Sample,
{
    pub fn new(inner: S, tx: UiInboxSender<f32>) -> Self {
        Self { inner, tx }
    }
}

impl<S> Iterator for VisualizerSource<S>
where
    S: Source,
    S::Item: rodio::Sample,
{
    type Item = S::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.inner.next()?;
    
        // Example of calculating volume (or RMS) of the sample:
        let value = sample.to_f32();
        let rms = value.abs(); // Simple example: just take the absolute value as a proxy for volume
    
        // Send the volume to the UI thread via the inbox
        let _ = self.tx.send(rms);
    
        Some(sample)
    }
}

impl<S> Source for VisualizerSource<S>
where
    S: Source,
    S::Item: rodio::Sample,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.inner.current_frame_len()
    }

    fn channels(&self) -> u16 {
        self.inner.channels()
    }

    fn sample_rate(&self) -> u32 {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        self.inner.total_duration()
    }
}
