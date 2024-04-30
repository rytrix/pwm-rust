use std::time::Duration;
use std::time::Instant;

pub struct Timer {
    start_time: Instant,
    duration: Duration,
}

impl Timer {
    pub fn new(duration: Duration) -> Timer {
        Timer {
            start_time: Instant::now(),
            duration,
        }
    }

    pub fn is_complete(&self) -> bool {
        if self.start_time.elapsed() >= self.duration {
            return true;
        }

        false
    }
}
