//! SQL queries organized by entity.
//!
//! All queries are defined as const strings and automatically validated at test time
//! against an in-memory SQLite database with the full schema applied.
//!
//! # Typed Query Syntax
//!
//! Queries can be defined with type-safe parameters and return types:
//!
//! ```ignore
//! // Execute (INSERT/UPDATE/DELETE)
//! define_query!(
//!     UpdateTheme,
//!     "UPDATE AppConfiguration SET theme = ?1 WHERE id = 0",
//!     execute(theme: &str)
//! );
//! UpdateTheme::execute(&conn, "dark")?;
//!
//! // Single row query
//! define_query!(
//!     FindAccountByUuid,
//!     "SELECT * FROM Account WHERE uuid = ?1",
//!     query_row(uuid: &str) -> Account
//! );
//! let account = FindAccountByUuid::query_row(&conn, &uuid)?;
//! let maybe = FindAccountByUuid::query_row_optional(&conn, &uuid)?;
//!
//! // Multi-row query
//! define_query!(
//!     ListAccounts,
//!     "SELECT * FROM Account ORDER BY lastUsed DESC",
//!     query() -> Account
//! );
//! let accounts: Vec<Account> = ListAccounts::query_vec(&conn)?;
//! ```

pub mod settings;
pub mod account;
pub mod java;
pub mod instance;
pub mod cache;
pub mod metadata;
pub mod modpack;

/// Macro for defining SQL queries with automatic test-time validation.
///
/// Creates a struct with a `SQL` constant and automatically generates a test
/// that validates the query against the database schema.
///
/// # Syntax Variants
///
/// ## Legacy (untyped)
/// ```ignore
/// define_query!(GetUser, "SELECT * FROM users WHERE id = ?1");
/// conn.query_row(GetUser::SQL, [user_id], |row| User::from_row(row))?;
/// ```
///
/// ## Execute (INSERT/UPDATE/DELETE)
/// ```ignore
/// define_query!(DeleteUser, "DELETE FROM users WHERE id = ?1", execute(id: i32));
/// DeleteUser::execute(&conn, 42)?;
/// ```
///
/// ## Query Row (single result)
/// ```ignore
/// define_query!(FindUser, "SELECT * FROM users WHERE id = ?1", query_row(id: i32) -> User);
/// let user = FindUser::query_row(&conn, 42)?;
/// let maybe_user = FindUser::query_row_optional(&conn, 42)?;
/// ```
///
/// ## Query (multiple results)
/// ```ignore
/// define_query!(ListUsers, "SELECT * FROM users", query() -> User);
/// let users: Vec<User> = ListUsers::query_vec(&conn)?;
/// ```
#[macro_export]
macro_rules! define_query {
    // Legacy syntax - untyped, backward compatible
    ($name:ident, $sql:expr) => {
        #[doc = concat!("Query: `", $sql, "`")]
        pub struct $name;

        impl $name {
            /// The SQL query string.
            pub const SQL: &'static str = $sql;
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare(&format!("EXPLAIN {}", $sql))
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Execute syntax - for INSERT/UPDATE/DELETE
    ($name:ident, $sql:expr, execute($($param:ident: $ptype:ty),* $(,)?)) => {
        #[doc = concat!("Query: `", $sql, "`")]
        $(#[doc = concat!("- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        pub struct $name;

        impl $name {
            /// The SQL query string.
            pub const SQL: &'static str = $sql;

            /// Execute this query (INSERT/UPDATE/DELETE).
            ///
            /// Returns the number of rows affected.
            pub fn execute(
                conn: &rusqlite::Connection,
                $($param: $ptype),*
            ) -> rusqlite::Result<usize> {
                conn.execute(Self::SQL, rusqlite::params![$($param),*])
            }
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare(&format!("EXPLAIN {}", $sql))
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Query row syntax - for single row SELECT
    ($name:ident, $sql:expr, query_row($($param:ident: $ptype:ty),* $(,)?) -> $ret:ty) => {
        #[doc = concat!("Query: `", $sql, "`")]
        $(#[doc = concat!("- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        #[doc = concat!("Returns: `", stringify!($ret), "`")]
        pub struct $name;

        impl $name {
            /// The SQL query string.
            pub const SQL: &'static str = $sql;

            /// Query a single row.
            ///
            /// Returns an error if no rows match or if multiple rows match.
            pub fn query_row(
                conn: &rusqlite::Connection,
                $($param: $ptype),*
            ) -> rusqlite::Result<$ret> {
                conn.query_row(Self::SQL, rusqlite::params![$($param),*], |row| <$ret>::from_row(row))
            }

            /// Query a single row, returning `None` if not found.
            ///
            /// Returns an error only for database errors (not "no rows").
            pub fn query_row_optional(
                conn: &rusqlite::Connection,
                $($param: $ptype),*
            ) -> rusqlite::Result<Option<$ret>> {
                match Self::query_row(conn, $($param),*) {
                    Ok(value) => Ok(Some(value)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            }
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare(&format!("EXPLAIN {}", $sql))
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Query syntax - for multiple row SELECT
    ($name:ident, $sql:expr, query($($param:ident: $ptype:ty),* $(,)?) -> $ret:ty) => {
        #[doc = concat!("Query: `", $sql, "`")]
        $(#[doc = concat!("- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        #[doc = concat!("Returns: `Vec<", stringify!($ret), ">`")]
        pub struct $name;

        impl $name {
            /// The SQL query string.
            pub const SQL: &'static str = $sql;

            /// Query multiple rows into a Vec.
            pub fn query_vec(
                conn: &rusqlite::Connection,
                $($param: $ptype),*
            ) -> rusqlite::Result<Vec<$ret>> {
                let mut stmt = conn.prepare(Self::SQL)?;
                let iter = stmt.query_map(rusqlite::params![$($param),*], |row| <$ret>::from_row(row))?;
                iter.collect()
            }
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare(&format!("EXPLAIN {}", $sql))
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };
}
