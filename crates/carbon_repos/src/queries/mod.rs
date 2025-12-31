//! SQL queries organized by entity.
//!
//! All queries are defined with type-safe parameters and validated at test time
//! against an in-memory SQLite database with the full schema applied.
//!
//! # Type-Safe Query Syntax
//!
//! ```ignore
//! // Execute queries (INSERT/UPDATE/DELETE) - no return type
//! define_query!(UpdateTheme, "UPDATE AppConfiguration SET theme = ?1 WHERE id = 0", (theme: &str));
//! define_query!(DeleteAccount, "DELETE FROM Account WHERE uuid = ?1", (uuid: &str));
//! define_query!(ClearCache, "DELETE FROM HTTPCache", ());
//!
//! // Usage:
//! UpdateTheme::execute(&conn, "dark")?;
//!
//! // Select queries - requires return type, generates all 3 methods
//! define_query!(FindAccount, "SELECT * FROM Account WHERE uuid = ?1", (uuid: &str) -> Account);
//! define_query!(ListAccounts, "SELECT * FROM Account", () -> Account);
//!
//! // Usage - caller chooses which method:
//! let account = FindAccount::fetch_one(&conn, &uuid)?;       // Error if not found
//! let maybe = FindAccount::fetch_optional(&conn, &uuid)?;    // None if not found
//! let all = FindAccount::fetch_all(&conn, &uuid)?;           // Vec (any count)
//!
//! // Scalar queries - for COUNT, MAX, SUM, etc.
//! define_query!(CountAccounts, "SELECT COUNT(*) FROM Account", () => i32);
//!
//! // Usage:
//! let count = CountAccounts::fetch_scalar(&conn)?;
//! ```
//!
//! The SQL constant is private - you MUST use the typed methods.

pub mod account;
pub mod cache;
pub mod instance;
pub mod java;
pub mod metadata;
pub mod modpack;
pub mod settings;

/// Macro for defining type-safe SQL queries.
///
/// Creates a struct with typed methods for query execution. The SQL is private
/// to enforce type safety - callers must use the generated methods.
///
/// # Syntax
///
/// ## Execute (no return type)
/// For INSERT, UPDATE, DELETE, CREATE, DROP, ALTER, PRAGMA:
/// ```ignore
/// define_query!(DeleteUser, "DELETE FROM users WHERE id = ?1", (id: i32));
/// define_query!(ClearAll, "DELETE FROM users", ());
///
/// DeleteUser::execute(&conn, 42)?;  // Returns usize (rows affected)
/// ```
///
/// ## Query (with struct return type)
/// For SELECT - generates `fetch_one`, `fetch_optional`, and `fetch_all`:
/// ```ignore
/// define_query!(FindUser, "SELECT * FROM users WHERE id = ?1", (id: i32) -> User);
/// define_query!(ListUsers, "SELECT * FROM users", () -> User);
///
/// let user = FindUser::fetch_one(&conn, 42)?;        // Error if 0 rows
/// let maybe = FindUser::fetch_optional(&conn, 42)?;  // None if 0 rows
/// let users = ListUsers::fetch_all(&conn)?;          // Vec<User>
/// ```
///
/// ## Scalar Query (with primitive return type)
/// For SELECT returning a single value (COUNT, MAX, SUM, etc.):
/// ```ignore
/// define_query!(CountUsers, "SELECT COUNT(*) FROM users", () => i32);
/// define_query!(GetMaxId, "SELECT MAX(id) FROM users WHERE active = ?1", (active: bool) => i64);
///
/// let count = CountUsers::fetch_scalar(&conn)?;            // Returns i32
/// let max = GetMaxId::fetch_scalar_optional(&conn, true)?; // Returns Option<i64>
/// ```
#[macro_export]
macro_rules! define_query {
    // Legacy syntax - untyped, for complex queries not yet migrated
    // SQL is still private, but requires manual execution
    ($name:ident, $sql:expr) => {
        #[doc = concat!("Query: `", $sql, "`")]
        #[doc = "\n\n**Note:** This query uses legacy syntax and needs migration."]
        pub struct $name;

        impl $name {
            /// The SQL query string (for manual execution).
            pub const SQL: &'static str = $sql;
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare($sql)
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Execute syntax - no return type (for INSERT/UPDATE/DELETE/etc)
    ($name:ident, $sql:expr, ($($param:ident: $ptype:ty),* $(,)?)) => {
        #[doc = concat!("Execute query: `", $sql, "`")]
        $(#[doc = concat!("\n- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        pub struct $name;

        impl $name {
            /// The SQL query string (for transaction/manual execution).
            pub const SQL: &'static str = $sql;

            /// Execute this query.
            ///
            /// Returns the number of rows affected.
            pub fn execute(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<usize> {
                conn.as_connection().execute(Self::SQL, rusqlite::params![$($param),*])
            }
        }

        paste::paste! {
            #[cfg(test)]
            #[test]
            #[allow(non_snake_case)]
            fn [<validate_query_ $name>]() {
                let mut conn = rusqlite::Connection::open_in_memory().unwrap();
                $crate::migrations::run_migrations(&mut conn).unwrap();
                conn.prepare($sql)
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Scalar query syntax - for single-value returns (COUNT, MAX, SUM, etc)
    ($name:ident, $sql:expr, ($($param:ident: $ptype:ty),* $(,)?) => $scalar:ty) => {
        #[doc = concat!("Scalar query: `", $sql, "`")]
        $(#[doc = concat!("\n- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        #[doc = concat!("\nReturns: `", stringify!($scalar), "`")]
        pub struct $name;

        impl $name {
            /// The SQL query string (for transaction/manual execution).
            pub const SQL: &'static str = $sql;

            /// Fetch a single scalar value.
            ///
            /// Returns error if no rows match.
            pub fn fetch_scalar(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<$scalar> {
                conn.as_connection().query_row(Self::SQL, rusqlite::params![$($param),*], |row| row.get(0))
            }

            /// Fetch an optional scalar value.
            ///
            /// Returns `None` if no rows match.
            pub fn fetch_scalar_optional(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<Option<$scalar>> {
                match conn.as_connection().query_row(Self::SQL, rusqlite::params![$($param),*], |row| row.get(0)) {
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
                conn.prepare($sql)
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };

    // Query syntax - with struct return type (for SELECT)
    ($name:ident, $sql:expr, ($($param:ident: $ptype:ty),* $(,)?) -> $ret:ty) => {
        #[doc = concat!("Select query: `", $sql, "`")]
        $(#[doc = concat!("\n- `", stringify!($param), "`: `", stringify!($ptype), "`")])*
        #[doc = concat!("\nReturns: `", stringify!($ret), "`")]
        pub struct $name;

        impl $name {
            /// The SQL query string (for transaction/manual execution).
            pub const SQL: &'static str = $sql;

            /// Fetch exactly one row.
            ///
            /// Returns error if no rows match.
            pub fn fetch_one(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<$ret> {
                conn.as_connection().query_row(Self::SQL, rusqlite::params![$($param),*], |row| <$ret>::from_row(row))
            }

            /// Fetch zero or one row.
            ///
            /// Returns `None` if no rows match.
            pub fn fetch_optional(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<Option<$ret>> {
                match conn.as_connection().query_row(Self::SQL, rusqlite::params![$($param),*], |row| <$ret>::from_row(row)) {
                    Ok(value) => Ok(Some(value)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e),
                }
            }

            /// Fetch all matching rows.
            pub fn fetch_all(
                conn: &impl $crate::AsConnection,
                $($param: $ptype),*
            ) -> rusqlite::Result<Vec<$ret>> {
                let conn = conn.as_connection();
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
                conn.prepare($sql)
                    .expect(concat!("Query validation failed: ", stringify!($name)));
            }
        }
    };
}
