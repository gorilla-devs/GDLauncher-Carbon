use rusqlite::Connection;

mod q {
    use carbon_repos::from_row::FromRow;
    use carbon_repos::queries;

    #[derive(carbon_macro::FromRow, Debug, PartialEq)]
    pub struct Row {
        pub id: String,
        pub major: i32,
    }

    queries! {
        fn get_by_id(id: &str) -> Option<Row> = "SELECT id, major FROM Java WHERE id = :id";
        fn get_all() -> Vec<Row> = "SELECT id, major FROM Java";
        fn set_major(id: &str, major: i32) -> usize = "UPDATE Java SET major = :major WHERE id = :id";
    }
}

#[test]
fn typed_wrappers_and_registry() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE Java (id TEXT PRIMARY KEY, major INTEGER);
         INSERT INTO Java VALUES ('a', 17), ('b', 21);",
    )
    .unwrap();

    assert_eq!(q::get_by_id(&conn, "a").unwrap().unwrap().major, 17);
    assert_eq!(q::get_all(&conn).unwrap().len(), 2);
    assert_eq!(q::set_major(&conn, "a", 22).unwrap(), 1);
    assert_eq!(q::get_by_id(&conn, "a").unwrap().unwrap().major, 22);
    assert_eq!(q::get_by_id(&conn, "zz").unwrap(), None);

    // registry captures every query with its param names and row metadata
    assert_eq!(q::QUERIES.len(), 3);
    let set = q::QUERIES.iter().find(|c| c.name == "set_major").unwrap();
    assert_eq!(set.params, &[":id", ":major"]);
}
