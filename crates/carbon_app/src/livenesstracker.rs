use core::fmt;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};

pub struct LivenessTracker {
    permits: AtomicUsize,
    changed_fn: Box<dyn Fn(usize) + Send + Sync>,
}

pub struct LivenessMarker {
    tracker: Weak<LivenessTracker>,
}

impl LivenessTracker {
    pub fn new(changed_fn: impl Fn(usize) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            permits: AtomicUsize::new(0),
            changed_fn: Box::new(changed_fn),
        })
    }

    pub fn marker(self: &Arc<Self>) -> LivenessMarker {
        let count = self.permits.fetch_add(1, Ordering::Relaxed) + 1;
        (self.changed_fn)(count);

        LivenessMarker {
            tracker: Arc::downgrade(self),
        }
    }
}

impl Drop for LivenessMarker {
    fn drop(&mut self) {
        if let Some(tracker) = self.tracker.upgrade() {
            let count = tracker.permits.fetch_sub(1, Ordering::Relaxed) - 1;
            (tracker.changed_fn)(count);
        }
    }
}

impl fmt::Debug for LivenessTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LivenessTracker")
    }
}
