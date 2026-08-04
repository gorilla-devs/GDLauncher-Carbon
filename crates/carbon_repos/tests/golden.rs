use carbon_repos::dbtypes::from_millis;
use rusqlite::Connection;

// `test-assets/golden_pcr.db` is a frozen artifact: it was written by
// prisma-client-rust (via quaint) while that dependency still existed, capturing
// the exact on-disk encoding the app shipped with. It can no longer be
// regenerated — that is the point. The test below reads it back through the
// rusqlite repository layer to prove the two encodings agree, so it stays as a
// permanent regression guard against the codec drifting away from real data.
const GOLDEN: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/test-assets/golden_pcr.db");
const KNOWN_MS: i64 = 1_784_557_692_104;

#[test]
fn golden_db_reads_back_exact_values_through_new_layer() {
    let conn = Connection::open(GOLDEN).unwrap();

    // storage class is INTEGER and the raw value is exact millis
    let (typeof_, raw): (String, i64) = conn
        .query_row(
            "SELECT typeof(tokenExpires), tokenExpires FROM Account WHERE uuid = 'golden-uuid'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(typeof_, "integer", "PCR must have written INTEGER millis");
    assert_eq!(raw, KNOWN_MS);

    // our codec decodes it to the same instant
    let via_codec: carbon_repos::dbtypes::DbDateTime = conn
        .query_row(
            "SELECT tokenExpires FROM Account WHERE uuid = 'golden-uuid'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(via_codec.0, from_millis(KNOWN_MS).unwrap());

    let major: i32 = conn
        .query_row(
            "SELECT major FROM Java WHERE id = 'golden-java-id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(major, 17);
}
