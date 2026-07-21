//! Owned sqlite executor: one dedicated writer thread + a small read pool.
//! Closures run entirely on the owning thread, so holding a connection
//! across an .await is impossible by construction.

use crate::db_error::{DbError, DbResult};
use rusqlite::Connection;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

type Job = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

struct Actor {
    tx: mpsc::Sender<Job>,
    _handle: JoinHandle<()>,
}

impl Actor {
    fn spawn(mut conn: Connection) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let handle = std::thread::spawn(move || {
            while let Ok(job) = rx.recv() {
                job(&mut conn);
            }
        });
        Actor { tx, _handle: handle }
    }

    async fn run<T, F>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&mut Connection) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let (otx, orx) = tokio::sync::oneshot::channel();
        let job: Job = Box::new(move |conn| {
            // A panic inside a job must not tear down the executor thread: catch
            // it here, convert it to a `Conversion` error reply, and let the
            // thread keep serving later jobs. `AssertUnwindSafe` is required
            // because neither `f` nor `&mut Connection` is `UnwindSafe`.
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(conn)));
            let reply = outcome.unwrap_or_else(|payload| {
                let msg = panic_message(payload.as_ref());
                tracing::error!("database job panicked: {msg}");
                Err(DbError::Conversion(format!("panic: {msg}")))
            });
            let _ = otx.send(reply);
        });
        self.tx.send(job).map_err(|_| DbError::Closed)?;
        orx.await.map_err(|_| DbError::Closed)?
    }
}

/// Best-effort extraction of a panic payload's message.
fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

pub struct Db {
    writer: Actor,
    readers: Vec<Actor>,
    next_reader: AtomicUsize,
}

fn apply_runtime_pragmas(conn: &Connection, foreign_keys: bool) -> rusqlite::Result<()> {
    // The connection-level PRAGMAs the app runs against every pool connection
    // (spec §2.6).
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 268435456;
         PRAGMA busy_timeout = 5000;",
    )?;
    // FK enforcement is per-connection and decided by the startup sweep (spec
    // §7): every runtime connection must agree, so the verdict is threaded in.
    conn.pragma_update(None, "foreign_keys", &if foreign_keys { "ON" } else { "OFF" })
}

impl Db {
    pub fn open(path: &Path, read_connections: usize, foreign_keys: bool) -> DbResult<Db> {
        let wconn = Connection::open(path)?;
        apply_runtime_pragmas(&wconn, foreign_keys)?;
        let writer = Actor::spawn(wconn);
        let mut readers = Vec::with_capacity(read_connections);
        for _ in 0..read_connections.max(1) {
            let rconn = Connection::open(path)?;
            apply_runtime_pragmas(&rconn, foreign_keys)?;
            readers.push(Actor::spawn(rconn));
        }
        Ok(Db { writer, readers, next_reader: AtomicUsize::new(0) })
    }

    pub async fn write<T, F>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&mut Connection) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.writer.run(f).await
    }

    pub async fn read<T, F>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(&mut Connection) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let i = self.next_reader.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        self.readers[i].run(f).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (tempfile::TempDir, Db) {
        let dir = tempfile::tempdir().unwrap();
        let db = Db::open(&dir.path().join("t.db"), 2, false).unwrap();
        (dir, db)
    }

    #[tokio::test]
    async fn write_then_read_roundtrip() {
        let (_d, db) = temp_db();
        db.write(|c| {
            c.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (42)")?;
            Ok(())
        })
        .await
        .unwrap();
        let v: i64 = db
            .read(|c| Ok(c.query_row("SELECT v FROM t", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(v, 42);
    }

    #[tokio::test]
    async fn reads_run_concurrently_with_held_write() {
        let (_d, db) = temp_db();
        db.write(|c| Ok(c.execute_batch("CREATE TABLE t (v INTEGER)")?))
            .await
            .unwrap();
        // Hold the writer busy; reads must still complete (WAL).
        let w = db.write(|c| {
            c.execute("INSERT INTO t VALUES (1)", [])?;
            std::thread::sleep(std::time::Duration::from_millis(300));
            Ok(())
        });
        let r = db.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM t", [], |r| r.get::<_, i64>(0))?));
        let (wr, rr) = tokio::join!(w, r);
        wr.unwrap();
        rr.unwrap(); // completing at all (not deadlocking behind the writer) is the assertion
    }

    #[tokio::test]
    async fn wal_and_pragmas_applied() {
        let (_d, db) = temp_db();
        let mode: String = db
            .read(|c| Ok(c.query_row("PRAGMA journal_mode", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(mode.to_lowercase(), "wal");
    }

    #[tokio::test]
    async fn panicking_job_is_contained_and_thread_survives() {
        let (_d, db) = temp_db();
        // A job that panics must come back as a Conversion error, not tear the
        // executor thread down.
        let panicked: DbResult<()> = db.write(|_c| panic!("boom in a job")).await;
        match panicked {
            Err(DbError::Conversion(msg)) => assert!(
                msg.contains("panic") && msg.contains("boom"),
                "panic reply should carry the message, got: {msg}"
            ),
            other => panic!("expected a Conversion error from a panicking job, got: {other:?}"),
        }
        // The same executor must still serve subsequent jobs.
        db.write(|c| Ok(c.execute_batch("CREATE TABLE survived (v INTEGER)")?))
            .await
            .expect("writer thread must survive a panicked job");
        let n: i64 = db
            .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM survived", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn foreign_keys_pragma_reflects_open_flag() {
        let dir = tempfile::tempdir().unwrap();
        let on = Db::open(&dir.path().join("on.db"), 1, true).unwrap();
        let off = Db::open(&dir.path().join("off.db"), 1, false).unwrap();
        let fk_on: i64 = on
            .read(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
            .await
            .unwrap();
        let fk_off: i64 = off
            .read(|c| Ok(c.query_row("PRAGMA foreign_keys", [], |r| r.get(0))?))
            .await
            .unwrap();
        assert_eq!(fk_on, 1, "open(foreign_keys=true) must enable enforcement");
        assert_eq!(fk_off, 0, "open(foreign_keys=false) must leave it off");
    }
}
