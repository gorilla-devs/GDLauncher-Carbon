//! Pins the all-or-nothing semantics multi-statement write groups rely on: a
//! `WriteTx` that errors (or is dropped) before `commit` leaves no trace, and
//! a committed one persists every statement.

use carbon_repos::db_error::DbError;
use carbon_repos::db_exec::{Db, ReadAccess, WriteAccess};

fn temp_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(&dir.path().join("tx.db"), 1, false).unwrap();
    (dir, db)
}

#[tokio::test]
async fn write_tx_rolls_back_on_error_and_persists_on_commit() {
    let (_d, db) = temp_db();
    db.write(|c| Ok(c.execute_batch("CREATE TABLE t (v INTEGER)")?))
        .await
        .unwrap();

    // Error before commit: the completed first statement must NOT persist.
    let r = db
        .write(|mut conn| {
            let tx = conn.transaction()?;
            tx.execute("INSERT INTO t VALUES (1)", [])?;
            Err::<(), _>(DbError::Conversion("injected failure after first insert".into()))
        })
        .await;
    assert!(r.is_err());
    let n: i64 = db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(n, 0, "uncommitted transaction must leave no trace");

    // Commit: both statements persist together.
    db.write(|mut conn| {
        let tx = conn.transaction()?;
        tx.execute("INSERT INTO t VALUES (1)", [])?;
        tx.execute("INSERT INTO t VALUES (2)", [])?;
        tx.commit()?;
        Ok(())
    })
    .await
    .unwrap();
    let n: i64 = db
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(n, 2, "committed transaction must persist every statement");
}
