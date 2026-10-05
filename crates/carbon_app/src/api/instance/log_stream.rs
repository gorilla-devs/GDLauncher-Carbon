use axum::extract::ws::Message;
use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::sync::watch;

use crate::managers::instance::log::GameLog;

pub(super) async fn stream_log<S, R, E>(
    mut socket: S,
    mut peer: R,
    mut log_rx: watch::Receiver<GameLog>,
) where
    S: Sink<Message, Error = E> + Unpin,
    R: Stream<Item = Result<Message, E>> + Unpin,
    E: std::fmt::Debug,
{
    let send_logs = async move {
        let mut last_idx = 0;
        loop {
            let new_lines = {
                let log = log_rx.borrow();
                let new_lines = log
                    .get_span(last_idx..)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();
                last_idx = log.len();
                new_lines
            };
            if let Err(error) = socket
                .send(Message::Text(
                    serde_json::to_string(&new_lines)
                        .expect("serialization of a log entry should be infallible"),
                ))
                .await
            {
                tracing::error!(?error, "Failed to send log entry");
                return;
            }
            if log_rx.changed().await.is_err() {
                return;
            }
        }
    };
    let peer_closed = async move {
        while let Some(message) = peer.next().await {
            match message {
                Ok(Message::Close(_)) | Err(_) => return,
                _ => {}
            }
        }
    };
    // Peer closure also cancels a send that is blocked by transport backpressure.
    tokio::select! {
        _ = send_logs => {},
        _ = peer_closed => {},
    }
}

#[cfg(test)]
mod task25_stream_tests {
    use super::*;
    use crate::managers::instance::log::LogEntry;
    use std::{
        pin::Pin,
        task::{Context, Poll},
        time::Duration,
    };
    use tokio::sync::mpsc;

    struct TestSink {
        sent: mpsc::UnboundedSender<String>,
        attempted: mpsc::UnboundedSender<usize>,
        calls: usize,
        fail_at: Option<usize>,
        block_at: Option<usize>,
    }
    impl Sink<Message> for TestSink {
        type Error = &'static str;
        fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            let this = self.get_mut();
            if this.block_at == Some(this.calls) {
                let _ = this.attempted.send(this.calls);
                Poll::Pending
            } else {
                Poll::Ready(Ok(()))
            }
        }
        fn start_send(self: Pin<&mut Self>, message: Message) -> Result<(), Self::Error> {
            let this = self.get_mut();
            let call = this.calls;
            this.calls += 1;
            let _ = this.attempted.send(call);
            if this.fail_at == Some(call) {
                return Err("synthetic send failure");
            }
            if let Message::Text(text) = message {
                let _ = this.sent.send(text);
            }
            Ok(())
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
    }
    fn fixture(
        fail_at: Option<usize>,
        block_at: Option<usize>,
    ) -> (
        watch::Sender<GameLog>,
        watch::Receiver<GameLog>,
        TestSink,
        mpsc::UnboundedReceiver<String>,
        mpsc::UnboundedReceiver<usize>,
        mpsc::UnboundedSender<Result<Message, &'static str>>,
        impl Stream<Item = Result<Message, &'static str>> + Unpin,
    ) {
        let mut log = GameLog::new();
        log.add_entry(LogEntry::system_error("history"));
        let (tx, rx) = watch::channel(log);
        let (sent, received) = mpsc::unbounded_channel();
        let (attempted, attempts) = mpsc::unbounded_channel();
        let (peer_tx, peer_rx) = mpsc::unbounded_channel();
        let peer = Box::pin(futures::stream::unfold(peer_rx, |mut rx| async move {
            rx.recv().await.map(|message| (message, rx))
        }));
        (
            tx,
            rx,
            TestSink {
                sent,
                attempted,
                calls: 0,
                fail_at,
                block_at,
            },
            received,
            attempts,
            peer_tx,
            peer,
        )
    }
    async fn stopped(mut handle: tokio::task::JoinHandle<()>) {
        let result = tokio::time::timeout(Duration::from_millis(200), &mut handle).await;
        if result.is_err() {
            handle.abort();
            let _ = handle.await;
        }
        assert!(
            result.is_ok(),
            "production stream retained a disconnected subscription"
        );
        assert!(
            result.unwrap().is_ok(),
            "production stream panicked during disconnect cleanup"
        );
    }
    async fn next<T>(rx: &mut mpsc::UnboundedReceiver<T>) -> T {
        tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .unwrap()
            .unwrap()
    }
    #[tokio::test]
    async fn first_send_failure_releases_live_receiver() {
        let (tx, rx, sink, _sent, mut attempts, _peer_tx, peer) = fixture(Some(0), None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut attempts).await;
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn later_send_failure_releases_live_receiver() {
        let (tx, rx, sink, mut sent, mut attempts, _peer_tx, peer) = fixture(Some(1), None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut sent).await;
        next(&mut attempts).await;
        tx.send_modify(|log| log.add_entry(LogEntry::system_error("append")));
        next(&mut attempts).await;
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn quiet_close_releases_live_receiver() {
        let (tx, rx, sink, mut sent, _attempts, peer_tx, peer) = fixture(None, None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut sent).await;
        peer_tx.send(Ok(Message::Close(None))).unwrap();
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn quiet_eof_releases_live_receiver() {
        let (tx, rx, sink, mut sent, _attempts, peer_tx, peer) = fixture(None, None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut sent).await;
        drop(peer_tx);
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn quiet_receive_error_releases_live_receiver() {
        let (tx, rx, sink, mut sent, _attempts, peer_tx, peer) = fixture(None, None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut sent).await;
        peer_tx.send(Err("synthetic receive error")).unwrap();
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn close_during_blocked_initial_send_releases_receiver() {
        let (tx, rx, sink, _sent, mut attempts, peer_tx, peer) = fixture(None, Some(0));
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut attempts).await;
        peer_tx.send(Ok(Message::Close(None))).unwrap();
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn close_during_blocked_append_releases_receiver() {
        let (tx, rx, sink, mut sent, mut attempts, peer_tx, peer) = fixture(None, Some(1));
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        next(&mut sent).await;
        next(&mut attempts).await;
        tx.send_modify(|log| log.add_entry(LogEntry::system_error("append")));
        next(&mut attempts).await;
        peer_tx.send(Ok(Message::Close(None))).unwrap();
        stopped(task).await;
        assert_eq!(tx.receiver_count(), 1);
    }
    #[tokio::test]
    async fn array_history_append_order_and_sender_close_are_preserved() {
        let (tx, rx, sink, mut sent, _attempts, peer_tx, peer) = fixture(None, None);
        let task = tokio::spawn(stream_log(sink, peer, rx.clone()));
        let first: serde_json::Value = serde_json::from_str(&next(&mut sent).await).unwrap();
        assert_eq!(first[0]["message"], "history");
        peer_tx
            .send(Ok(Message::Text("ignored inbound".into())))
            .unwrap();
        tx.send_modify(|log| {
            log.add_entry(LogEntry::system_error("one"));
            log.add_entry(LogEntry::system_error("two"));
        });
        let append: serde_json::Value = serde_json::from_str(&next(&mut sent).await).unwrap();
        assert_eq!(append.as_array().unwrap().len(), 2);
        assert_eq!(append[0]["message"], "one");
        assert_eq!(append[1]["message"], "two");
        drop(tx);
        stopped(task).await;
    }
    #[tokio::test]
    async fn completed_sender_still_delivers_initial_array_history() {
        let (tx, rx, sink, mut sent, _attempts, _peer_tx, peer) = fixture(None, None);
        drop(tx);
        let task = tokio::spawn(stream_log(sink, peer, rx));
        let history: serde_json::Value = serde_json::from_str(&next(&mut sent).await).unwrap();
        assert_eq!(history.as_array().unwrap().len(), 1);
        assert_eq!(history[0]["message"], "history");
        stopped(task).await;
    }
}
