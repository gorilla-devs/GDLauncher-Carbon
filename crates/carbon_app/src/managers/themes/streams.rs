//! The theme files the asset route is streaming, per theme. On Windows a
//! folder can't be renamed or deleted while a file inside it is open, and a
//! `<video>` background can keep its file open for the whole session, so
//! install and remove end a theme's streams before they touch its folder.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use tokio_util::sync::CancellationToken;

/// How long `close` waits for the ended streams to let go of their files.
const CLOSE_WAIT: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct AssetStreams {
    gates: Mutex<HashMap<String, Gate>>,
}

/// One theme's current token. `live` upgrades while any ticket taken under
/// that token is alive.
#[derive(Default)]
struct Gate {
    token: CancellationToken,
    live: Weak<()>,
}

/// Held by one asset response for as long as it may have the theme's file
/// open. A streamed body drops it only after its file is closed.
pub struct StreamTicket {
    token: CancellationToken,
    _live: Arc<()>,
}

impl StreamTicket {
    /// The theme is being replaced or removed: open nothing, end now.
    pub fn is_cancelled(&self) -> bool {
        self.token.is_cancelled()
    }

    pub async fn cancelled(&self) {
        self.token.cancelled().await
    }
}

/// Returned by `close`. New streams of the theme end at once until it is
/// dropped; after that they get a fresh token.
pub struct Closing<'a> {
    streams: &'a AssetStreams,
    id: String,
    /// Every stream that was open let go of its file in time.
    pub ended: bool,
}

impl Drop for Closing<'_> {
    fn drop(&mut self) {
        self.streams.gates().remove(&self.id);
    }
}

impl AssetStreams {
    fn gates(&self) -> MutexGuard<'_, HashMap<String, Gate>> {
        self.gates.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// A ticket for one response serving a file of theme `id`.
    pub fn ticket(&self, id: &str) -> StreamTicket {
        let mut gates = self.gates();
        let gate = gates.entry(id.to_string()).or_default();
        let live = gate.live.upgrade().unwrap_or_else(|| {
            let live = Arc::new(());
            gate.live = Arc::downgrade(&live);
            live
        });
        StreamTicket {
            token: gate.token.clone(),
            _live: live,
        }
    }

    /// Ends every stream of theme `id` and blocks until each has closed its
    /// file (at most `CLOSE_WAIT`), so the folder can then be renamed or
    /// deleted. Hold the result across the folder work.
    pub fn close(&self, id: &str) -> Closing<'_> {
        self.close_within(id, CLOSE_WAIT)
    }

    fn close_within(&self, id: &str, wait: Duration) -> Closing<'_> {
        let live = {
            let mut gates = self.gates();
            let gate = gates.entry(id.to_string()).or_default();
            gate.token.cancel();
            gate.live.clone()
        };
        let deadline = Instant::now() + wait;
        let mut ended = true;
        while live.strong_count() > 0 {
            if Instant::now() >= deadline {
                tracing::warn!(
                    "theme \"{id}\": a file it is streaming is still open; going on anyway"
                );
                ended = false;
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Closing {
            streams: self,
            id: id.to_string(),
            ended,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Holds `ticket` on a thread until it is cancelled.
    fn hold(ticket: StreamTicket) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || {
            while !ticket.is_cancelled() {
                std::thread::sleep(Duration::from_millis(1));
            }
        })
    }

    #[test]
    fn closing_a_theme_ends_its_streams_only() {
        let streams = AssetStreams::default();
        let x = hold(streams.ticket("x"));
        let y = streams.ticket("y");
        let closing = streams.close("x");
        assert!(closing.ended);
        assert!(!y.is_cancelled());
        x.join().unwrap();
    }

    #[test]
    fn a_theme_being_closed_hands_out_ended_tickets_then_fresh_ones() {
        let streams = AssetStreams::default();
        let closing = streams.close("x");
        assert!(closing.ended, "nothing was open");
        assert!(streams.ticket("x").is_cancelled());
        drop(closing);
        assert!(!streams.ticket("x").is_cancelled());
    }

    #[test]
    fn close_stops_waiting_for_a_stream_that_never_ends() {
        let streams = AssetStreams::default();
        let stuck = streams.ticket("x");
        let closing = streams.close_within("x", Duration::from_millis(30));
        assert!(!closing.ended);
        assert!(stuck.is_cancelled());
    }
}
