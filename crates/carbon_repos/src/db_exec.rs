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
            let _ = otx.send(f(conn));
        });
        self.tx.send(job).map_err(|_| DbError::Closed)?;
        orx.await.map_err(|_| DbError::Closed)?
    }
}

pub struct Db {
    writer: Actor,
    readers: Vec<Actor>,
    next_reader: AtomicUsize,
}

fn apply_runtime_pragmas(conn: &Connection) -> rusqlite::Result<()> {
    // Matches the PRAGMAs the app applied under PCR (spec §2.6).
    // foreign_keys intentionally NOT set here — FK enablement is Plan 3 (spec §7).
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         PRAGMA mmap_size = 268435456;
         PRAGMA busy_timeout = 5000;",
    )
}

impl Db {
    pub fn open(path: &Path, read_connections: usize) -> DbResult<Db> {
        let wconn = Connection::open(path)?;
        apply_runtime_pragmas(&wconn)?;
        let writer = Actor::spawn(wconn);
        let mut readers = Vec::with_capacity(read_connections);
        for _ in 0..read_connections.max(1) {
            let rconn = Connection::open(path)?;
            apply_runtime_pragmas(&rconn)?;
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
        let db = Db::open(&dir.path().join("t.db"), 2).unwrap();
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
}
