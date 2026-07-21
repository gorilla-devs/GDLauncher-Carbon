//! Pins the WAL snapshot-isolation semantics `ReadGuard::snapshot()` provides:
//! reads inside one snapshot observe a single database world even while the
//! writer commits concurrently, and reads WITHOUT a snapshot observe the
//! writer's commit between statements. The channel choreography makes the
//! interleaving deterministic: the read closure blocks its reader-actor thread
//! while the writer (a separate thread) commits.

use std::sync::mpsc;

use carbon_repos::db_exec::{Db, ReadAccess, WriteAccess};

fn temp_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("iso.db"), 2, false).unwrap();
    (dir, db)
}

async fn interleaved_reads(db: &Db, use_snapshot: bool) -> (i64, i64) {
    let (to_writer, at_first_read) = mpsc::channel::<()>();
    let (to_reader, write_committed) = mpsc::channel::<()>();

    let read_side = db.read(move |conn| {
        let q = "SELECT COUNT(*) FROM t";
        let (v1, v2);
        if use_snapshot {
            let snap = conn.snapshot()?;
            v1 = snap.query_row(q, [], |r| r.get::<_, i64>(0))?;
            to_writer.send(()).unwrap();
            write_committed.recv().unwrap();
            v2 = snap.query_row(q, [], |r| r.get::<_, i64>(0))?;
        } else {
            v1 = conn.query_row(q, [], |r| r.get::<_, i64>(0))?;
            to_writer.send(()).unwrap();
            write_committed.recv().unwrap();
            v2 = conn.query_row(q, [], |r| r.get::<_, i64>(0))?;
        }
        Ok((v1, v2))
    });

    let write_side = async {
        at_first_read.recv().unwrap();
        db.write(|conn| Ok(conn.execute("INSERT INTO t VALUES (2)", [])?))
            .await
            .unwrap();
        to_reader.send(()).unwrap();
    };

    let (read_res, ()) = tokio::join!(read_side, write_side);
    read_res.unwrap()
}

#[tokio::test]
async fn snapshot_pins_one_world_across_a_concurrent_commit() {
    let (_d, db) = temp_db();
    db.write(|c| Ok(c.execute_batch("CREATE TABLE t (v INTEGER); INSERT INTO t VALUES (1)")?))
        .await
        .unwrap();

    let (v1, v2) = interleaved_reads(&db, true).await;
    assert_eq!((v1, v2), (1, 1), "snapshot reads must agree despite the concurrent commit");

    let (v1, v2) = interleaved_reads(&db, false).await;
    assert_eq!(v1, 2, "left over from the snapshot run's insert");
    assert_eq!(v2, 3, "bare reads autocommit per statement and must observe the new commit");
}
