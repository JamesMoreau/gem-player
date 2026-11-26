use log::error;
use nosleep::{NoSleep, NoSleepType};

pub struct NoSleepManager {
    no_sleep: Option<NoSleep>,
}

impl NoSleepManager {
    pub fn new() -> Self {
        Self { no_sleep: None }
    }

    pub fn enable(&mut self) {
        if self.no_sleep.is_none()
            && let Ok(mut ns) = NoSleep::new()
        {
            if let Err(e) = ns.start(NoSleepType::PreventUserIdleDisplaySleep) {
                error!("Unable to enable no sleep mode: {e}");
            } else {
                self.no_sleep = Some(ns);
            }
        }
    }

    pub fn disable(&mut self) {
        if let Some(ns) = self.no_sleep.take()
            && let Err(e) = ns.stop()
        {
            error!("Failed to stop sleep inhibitor: {e}");
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.no_sleep.is_some()
    }
}
