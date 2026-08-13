//! Worker wakeup: commands, translate completion, or a timed capture interval.

use std::{sync::Arc, time::Duration};

use parking_lot::{Condvar, Mutex};

#[derive(Debug)]
pub(crate) struct Wake {
    signaled: Mutex<bool>,
    cv: Condvar,
}

impl Wake {
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            signaled: Mutex::new(false),
            cv: Condvar::new(),
        })
    }

    pub(crate) fn notify(&self) {
        *self.signaled.lock() = true;
        self.cv.notify_one();
    }

    /// Wait until [`Self::notify`] or `timeout`. `None` waits indefinitely.
    pub(crate) fn wait(&self, timeout: Option<Duration>) {
        let mut signaled = self.signaled.lock();
        if *signaled {
            *signaled = false;
            return;
        }
        match timeout {
            Some(d) => {
                self.cv.wait_for(&mut signaled, d);
            }
            None => {
                self.cv.wait(&mut signaled);
            }
        }
        *signaled = false;
    }
}

/// Unblocks [`Wake::wait`] when a translate task ends, including panic/abort.
pub(crate) struct NotifyOnDrop(pub Arc<Wake>);

impl Drop for NotifyOnDrop {
    fn drop(&mut self) {
        self.0.notify();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use super::*;

    #[test]
    fn notify_before_wait_does_not_block() {
        let wake = Wake::new();
        wake.notify();
        let start = Instant::now();
        wake.wait(Some(Duration::from_secs(2)));
        assert!(start.elapsed() < Duration::from_millis(200));
    }

    #[test]
    fn wait_times_out() {
        let wake = Wake::new();
        let start = Instant::now();
        wake.wait(Some(Duration::from_millis(30)));
        assert!(start.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn notify_clears_so_next_wait_blocks() {
        let wake = Wake::new();
        wake.notify();
        wake.wait(None);
        let start = Instant::now();
        wake.wait(Some(Duration::from_millis(30)));
        assert!(start.elapsed() >= Duration::from_millis(20));
    }

    #[test]
    fn drop_notifies() {
        let wake = Wake::new();
        drop(NotifyOnDrop(Arc::clone(&wake)));
        let start = Instant::now();
        wake.wait(Some(Duration::from_secs(2)));
        assert!(start.elapsed() < Duration::from_millis(200));
    }
}
