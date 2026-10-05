use core::fmt;
use std::sync::{Arc, Mutex, Weak};

pub struct LivenessTracker {
    permits: Mutex<usize>,
    changed_fn: Box<dyn Fn(usize) + Send + Sync>,
}

pub struct LivenessMarker {
    tracker: Weak<LivenessTracker>,
}

impl LivenessTracker {
    /// Callbacks run synchronously under the counter gate. They must not panic,
    /// reenter this tracker or take a lock retained by marker creation/drop.
    pub fn new(changed_fn: impl Fn(usize) + Send + Sync + 'static) -> Arc<Self> {
        Arc::new(Self {
            permits: Mutex::new(0),
            changed_fn: Box::new(changed_fn),
        })
    }

    pub fn marker(self: &Arc<Self>) -> LivenessMarker {
        let mut count = self
            .permits
            .lock()
            .expect("liveness callback must not panic");
        *count += 1;
        (self.changed_fn)(*count);

        LivenessMarker {
            tracker: Arc::downgrade(self),
        }
    }
}

impl Drop for LivenessMarker {
    fn drop(&mut self) {
        if let Some(tracker) = self.tracker.upgrade() {
            let mut count = tracker
                .permits
                .lock()
                .expect("liveness callback must not panic");
            *count -= 1;
            (tracker.changed_fn)(*count);
        }
    }
}

impl fmt::Debug for LivenessTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LivenessTracker")
    }
}

#[cfg(test)]
mod task21_tests {
    use super::*;
    use std::sync::{Mutex, mpsc};
    use std::time::Duration;

    #[test]
    fn task21_liveness_publication_is_ordered() {
        let (running_tx, running) = tokio::sync::watch::channel(false);
        let (entered_tx, entered) = mpsc::channel();
        let (release_tx, release) = mpsc::channel();
        let release = Mutex::new(release);
        let counts = Arc::new(Mutex::new(Vec::new()));
        let observed = counts.clone();
        let tracker = LivenessTracker::new(move |count| {
            if std::thread::current().name() == Some("task21-first-drop") {
                entered_tx.send(()).unwrap();
                release
                    .lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
            observed.lock().unwrap().push(count);
            running_tx.send_replace(count != 0);
        });
        let a = tracker.marker();
        let b = tracker.marker();
        assert!(*running.borrow());
        let first = std::thread::Builder::new()
            .name("task21-first-drop".into())
            .spawn(move || drop(a))
            .unwrap();
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        let (started_tx, started) = mpsc::channel();
        let (done_tx, done) = mpsc::channel();
        let second = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            drop(b);
            done_tx.send(()).unwrap();
        });
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        let overtook = done.recv_timeout(Duration::from_millis(100)).is_ok();
        release_tx.send(()).unwrap();
        first.join().unwrap();
        second.join().unwrap();
        // The metadata pause bridge copies this watch value, so it must be false.
        assert!(
            !*running.borrow(),
            "zero markers left must resume metadata caching"
        );
        assert!(
            !overtook,
            "later transition must not publish before earlier callback"
        );
        assert_eq!(*counts.lock().unwrap(), [1, 2, 1, 0]);
    }

    #[test]
    fn task21_liveness_sequential_and_weak_owner_controls() {
        let (tx, rx) = tokio::sync::watch::channel(false);
        let tracker = LivenessTracker::new(move |count| {
            tx.send_replace(count != 0);
        });
        let a = tracker.marker();
        let b = tracker.marker();
        drop(a);
        assert!(*rx.borrow());
        drop(b);
        assert!(!*rx.borrow());
        let marker = tracker.marker();
        drop(tracker);
        drop(marker);
    }
}
