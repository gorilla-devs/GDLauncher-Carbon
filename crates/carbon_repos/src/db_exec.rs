//! Owned sqlite executor: one dedicated writer thread + a small read pool.
//! Closures run entirely on the owning thread, so holding a connection
//! across an .await is impossible by construction.

use crate::db_error::{DbError, DbResult};
use rusqlite::{Connection, OpenFlags};
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

// ---------------------------------------------------------------------------
// Typed connection guards
//
// `Db::read` hands a closure a `ReadGuard`, `Db::write` a `WriteGuard`. The two
// access traits below split the mirrored `Connection` surface so that a
// write-class `_conn` fn (which takes `&impl WriteAccess`) can never be called
// through a read guard: the read-only pool's connection is unreachable from a
// write signature at compile time, not merely rejected at runtime.
//
// Writability is orthogonal to transaction-ness: `ReadGuard::snapshot()` yields
// a `ReadTx` (a WAL read snapshot — `ReadAccess` only), while
// `WriteGuard::transaction()` yields a `WriteTx` (`ReadAccess + WriteAccess`, so
// a write transaction can read but a read snapshot can never write).
// ---------------------------------------------------------------------------

/// Read-only access surface shared by every connection guard. Mirrors just the
/// `rusqlite::Connection` methods the repository layer needs for reads. All four
/// guards (`ReadGuard`, `ReadTx`, `WriteGuard`, `WriteTx`) implement it — write
/// access is a strict superset of read access.
pub trait ReadAccess {
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement<'_>>;
    fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>;
    /// Escape hatch to the underlying connection for repo-internal, runtime-SQL
    /// paths (`DynamicQuery`). The hand-written-SQL census still governs what
    /// SQL runs through it.
    #[doc(hidden)]
    fn raw(&self) -> &Connection;
}

/// Write access surface: everything `ReadAccess` offers plus the mutating
/// `Connection` methods. Only the write guards (`WriteGuard`, `WriteTx`)
/// implement it, so a write-class `_conn` fn — which takes `&impl WriteAccess`
/// — cannot be called through a read guard or a read snapshot.
pub trait WriteAccess: ReadAccess {
    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize>;
    fn execute_batch(&self, sql: &str) -> rusqlite::Result<()>;
    fn last_insert_rowid(&self) -> i64;
}

/// A borrowed handle to a read-pool connection. Grants `ReadAccess` only.
#[derive(Clone, Copy)]
pub struct ReadGuard<'a>(&'a Connection);

/// A borrowed handle to the write-pool connection. Grants `ReadAccess +
/// WriteAccess`. Holds `&mut Connection` so `transaction()` uses rusqlite's
/// checked `Connection::transaction` and the borrow checker forbids touching the
/// connection directly while a transaction is live.
pub struct WriteGuard<'a>(&'a mut Connection);

/// A WAL read snapshot: a `BEGIN DEFERRED` transaction on a read-pool connection
/// pinning one consistent view across several reads. `ReadAccess` only — a
/// snapshot can never write.
pub struct ReadTx<'a>(rusqlite::Transaction<'a>);

/// A write transaction on the write-pool connection. `ReadAccess + WriteAccess`
/// — a write transaction can read.
pub struct WriteTx<'a>(rusqlite::Transaction<'a>);

impl<'a> ReadGuard<'a> {
    /// Wraps a connection reference in a read guard. `#[doc(hidden)]` — the
    /// executor plumbs guards to closures; this constructor also lets tests
    /// exercise `_conn` fns against a raw migrated connection.
    #[doc(hidden)]
    pub fn new(conn: &'a Connection) -> Self {
        ReadGuard(conn)
    }

    /// Opens a `BEGIN DEFERRED` read transaction that pins one WAL snapshot for
    /// the duration of the returned `ReadTx`, so several reads observe one
    /// consistent view.
    pub fn snapshot(&self) -> DbResult<ReadTx<'_>> {
        Ok(ReadTx(self.0.unchecked_transaction()?))
    }
}

impl<'a> WriteGuard<'a> {
    /// Wraps a mutable connection reference in a write guard. `#[doc(hidden)]` —
    /// see [`ReadGuard::new`].
    #[doc(hidden)]
    pub fn new(conn: &'a mut Connection) -> Self {
        WriteGuard(conn)
    }

    /// Begins a write transaction. Borrows the guard mutably, so the underlying
    /// connection cannot be used directly until the transaction is committed or
    /// dropped.
    pub fn transaction(&mut self) -> DbResult<WriteTx<'_>> {
        Ok(WriteTx(self.0.transaction()?))
    }
}

impl WriteTx<'_> {
    /// Commits the write transaction.
    pub fn commit(self) -> DbResult<()> {
        self.0.commit()?;
        Ok(())
    }
}

impl ReadTx<'_> {
    /// Ends the read snapshot. A read snapshot performs no writes, so committing
    /// and dropping are equivalent; provided for symmetry with `WriteTx`.
    pub fn commit(self) -> DbResult<()> {
        self.0.commit()?;
        Ok(())
    }
}

impl ReadAccess for ReadGuard<'_> {
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement<'_>> {
        self.0.prepare_cached(sql)
    }
    fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.0.query_row(sql, params, f)
    }
    fn raw(&self) -> &Connection {
        self.0
    }
}

impl ReadAccess for ReadTx<'_> {
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement<'_>> {
        self.0.prepare_cached(sql)
    }
    fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.0.query_row(sql, params, f)
    }
    fn raw(&self) -> &Connection {
        &self.0
    }
}

impl ReadAccess for WriteGuard<'_> {
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement<'_>> {
        self.0.prepare_cached(sql)
    }
    fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.0.query_row(sql, params, f)
    }
    fn raw(&self) -> &Connection {
        self.0
    }
}

impl WriteAccess for WriteGuard<'_> {
    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.0.execute(sql, params)
    }
    fn execute_batch(&self, sql: &str) -> rusqlite::Result<()> {
        self.0.execute_batch(sql)
    }
    fn last_insert_rowid(&self) -> i64 {
        self.0.last_insert_rowid()
    }
}

impl ReadAccess for WriteTx<'_> {
    fn prepare_cached(&self, sql: &str) -> rusqlite::Result<rusqlite::CachedStatement<'_>> {
        self.0.prepare_cached(sql)
    }
    fn query_row<T, P, F>(&self, sql: &str, params: P, f: F) -> rusqlite::Result<T>
    where
        P: rusqlite::Params,
        F: FnOnce(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
    {
        self.0.query_row(sql, params, f)
    }
    fn raw(&self) -> &Connection {
        &self.0
    }
}

impl WriteAccess for WriteTx<'_> {
    fn execute<P: rusqlite::Params>(&self, sql: &str, params: P) -> rusqlite::Result<usize> {
        self.0.execute(sql, params)
    }
    fn execute_batch(&self, sql: &str) -> rusqlite::Result<()> {
        self.0.execute_batch(sql)
    }
    fn last_insert_rowid(&self) -> i64 {
        self.0.last_insert_rowid()
    }
}

pub struct Db {
    writer: Actor,
    readers: Vec<Actor>,
    next_reader: AtomicUsize,
}

/// Applies the connection-level PRAGMAs the app runs against every pool
/// connection (spec §2.6).
///
/// `read_only` gates `journal_mode`: it is a database-header setting, so
/// *setting* it needs write access to the file. The writer always opens
/// first and puts the file into WAL mode before any reader connects, so
/// readers just need to observe that mode, not set it — attempting to set
/// journal_mode on a read-only connection would either error or (per SQLite)
/// silently return the current mode as a query result instead of executing,
/// which `execute_batch` can't express. Skip it here and let the writer own
/// journal-mode exclusively.
fn apply_runtime_pragmas(conn: &Connection, foreign_keys: bool, read_only: bool) -> rusqlite::Result<()> {
    if !read_only {
        conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    }
    conn.execute_batch(
        "PRAGMA synchronous = NORMAL;
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
        // Writer opens (and WAL-initializes) first: readers below open
        // read-only and must find the file already in WAL mode.
        let wconn = Connection::open(path)?;
        apply_runtime_pragmas(&wconn, foreign_keys, false)?;
        let writer = Actor::spawn(wconn);
        let mut readers = Vec::with_capacity(read_connections);
        // SQLITE_OPEN_READ_ONLY makes any write attempt through the read pool
        // fail loudly (SQLITE_READONLY) instead of silently succeeding.
        let read_flags = OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_NO_MUTEX;
        for _ in 0..read_connections.max(1) {
            let rconn = Connection::open_with_flags(path, read_flags)?;
            apply_runtime_pragmas(&rconn, foreign_keys, true)?;
            readers.push(Actor::spawn(rconn));
        }
        Ok(Db { writer, readers, next_reader: AtomicUsize::new(0) })
    }

    /// Runs `f` on the write-pool connection, handing it a [`WriteGuard`]
    /// (`ReadAccess + WriteAccess`).
    pub async fn write<T, F>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(WriteGuard) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        self.writer.run(move |conn| f(WriteGuard(conn))).await
    }

    /// Runs `f` on a read-pool connection, handing it a [`ReadGuard`]
    /// (`ReadAccess` only).
    ///
    /// The read/write split is enforced by the type system. A write-class
    /// `_conn` fn takes `&impl WriteAccess`, which a `ReadGuard` does not
    /// satisfy:
    ///
    /// ```compile_fail
    /// use carbon_repos::db_exec::ReadGuard;
    /// let conn = rusqlite::Connection::open_in_memory().unwrap();
    /// let guard = ReadGuard::new(&conn);
    /// // `set_instance_index_conn` is write-class (`&impl WriteAccess`); a
    /// // `ReadGuard` grants `ReadAccess` only, so this does not compile.
    /// carbon_repos::repos::instance::set_instance_index_conn(&guard, 1, 2).unwrap();
    /// ```
    ///
    /// A read snapshot (`ReadTx`) likewise grants `ReadAccess` only, so a write
    /// through it is a compile error, not a runtime read-only failure:
    ///
    /// ```compile_fail
    /// use carbon_repos::db_exec::ReadGuard;
    /// let conn = rusqlite::Connection::open_in_memory().unwrap();
    /// let guard = ReadGuard::new(&conn);
    /// let snap = guard.snapshot().unwrap();
    /// carbon_repos::repos::instance::set_instance_index_conn(&snap, 1, 2).unwrap();
    /// ```
    ///
    /// Writability is orthogonal to transaction-ness: a `WriteTx` grants
    /// `ReadAccess + WriteAccess`, so a write transaction can call a read-class
    /// `_conn` fn. This compiles:
    ///
    /// ```
    /// use carbon_repos::db_exec::WriteGuard;
    /// let mut conn = rusqlite::Connection::open_in_memory().unwrap();
    /// let mut guard = WriteGuard::new(&mut conn);
    /// let tx = guard.transaction().unwrap();
    /// // `get_instance_conn` is read-class (`&impl ReadAccess`); a `WriteTx`
    /// // satisfies that bound. (The call errors at runtime on the empty schema;
    /// // we only assert it type-checks.)
    /// let _ = carbon_repos::repos::instance::get_instance_conn(&tx, 1);
    /// ```
    pub async fn read<T, F>(&self, f: F) -> DbResult<T>
    where
        F: FnOnce(ReadGuard) -> DbResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let i = self.next_reader.fetch_add(1, Ordering::Relaxed) % self.readers.len();
        self.readers[i].run(move |conn| f(ReadGuard(conn))).await
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
    async fn write_through_read_pool_fails_loudly() {
        let (_d, db) = temp_db();
        db.write(|c| Ok(c.execute_batch("CREATE TABLE t (v INTEGER)")?))
            .await
            .unwrap();
        // A write attempted through the read pool must fail: the connection
        // is opened SQLITE_OPEN_READ_ONLY, so this cannot silently succeed. A
        // `ReadGuard` cannot even express a write, so this reaches for the raw
        // connection to prove the OS-level read-only flag still refuses it.
        let err = db.read(|c| Ok(c.raw().execute_batch("CREATE TABLE nope (x)")?)).await;
        assert!(err.is_err(), "write through the read pool must fail loudly, got {err:?}");
        // The same statement through the write pool succeeds.
        db.write(|c| Ok(c.execute_batch("CREATE TABLE nope (x)")?))
            .await
            .expect("write through the write pool must succeed");
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

/// Guard constructors for integration tests. Tests exercise `_conn` fns
/// against raw migrated connections and need real guards to satisfy the
/// access-trait bounds; production code receives guards from `Db::read`/
/// `Db::write` and never constructs them directly.
#[doc(hidden)]
pub mod test_support {
    use super::{Connection, ReadGuard, WriteGuard};

    pub fn wg(c: &mut Connection) -> WriteGuard<'_> {
        WriteGuard::new(c)
    }

    pub fn rg(c: &Connection) -> ReadGuard<'_> {
        ReadGuard::new(c)
    }
}
