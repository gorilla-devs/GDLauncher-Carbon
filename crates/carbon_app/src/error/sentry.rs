use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, SystemTime},
};

/// Capture an error and send it to sentry.
/// Will not send an erorr of the same type more than 3 times a day.
pub fn report_volatile_error(ty: &'static str, error: anyhow::Error) -> anyhow::Error {
    static TRACKER: Mutex<Option<HashMap<&'static str, (u8, SystemTime)>>> = Mutex::new(None);

    let error = error.context(format!("Caught volatile error: '{ty}'"));

    let mut tracker_lock = TRACKER.lock().unwrap();
    if tracker_lock.is_none() {
        *tracker_lock = Some(HashMap::new());
    }

    let tracker = tracker_lock.as_mut().unwrap();
    let e = tracker.entry(ty).or_insert((0, std::time::UNIX_EPOCH));

    if e.1 > SystemTime::now() {
        tracing::error!({ error = ?error }, "Did not report volatile error (type: '{ty}') due to too many repeated occurances");
        return error;
    } else if e.0 == 2 {
        e.0 = 0;
        e.1 = SystemTime::now() + Duration::from_secs(60 * 60 * 24);
    }

    e.0 += 1;
    drop(tracker_lock);

    let id = sentry::integrations::anyhow::capture_anyhow(&error);

    tracing::warn!({ error = ?error }, "Reported volatile error (type '{ty}') with id {id:?}");
    error
}

pub trait VolatileError {
    fn report_volatile(self, ty: &'static str) -> Self;
}

impl VolatileError for anyhow::Error {
    fn report_volatile(self, ty: &'static str) -> Self {
        report_volatile_error(ty, self)
    }
}

impl<T> VolatileError for Result<T, anyhow::Error> {
    fn report_volatile(self, ty: &'static str) -> Self {
        self.map_err(|e| e.report_volatile(ty))
    }
}
