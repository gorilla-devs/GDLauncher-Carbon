//! Compile-time query registry.
//!
//! The `queries!` macro emits, per entry, a sync `{name}_conn(conn, …)` fn plus
//! an async `{name}(db, …)` pool-routing wrapper, and a `QUERIES` const
//! describing every query (name, SQL, param names, and — for row-returning
//! queries — the expected columns), which the schema checker consumes.

use crate::from_row::ColumnSpec;

#[derive(Debug, Clone, Copy)]
pub struct QueryCheck {
    pub name: &'static str,
    pub sql: &'static str,
    pub params: &'static [&'static str],
    pub columns: Option<&'static [ColumnSpec]>,
}

/// Escape hatch for runtime-assembled SQL. Exempt from the static checker;
/// every construction site must have a dedicated execution test.
pub struct DynamicQuery {
    pub sql: String,
    pub params: Vec<(&'static str, Box<dyn rusqlite::types::ToSql + Send>)>,
}

impl DynamicQuery {
    pub fn execute(&self, conn: &rusqlite::Connection) -> Result<usize, rusqlite::Error> {
        let mut st = conn.prepare(&self.sql)?;
        let bound: Vec<(&str, &dyn rusqlite::types::ToSql)> = self
            .params
            .iter()
            .map(|(n, v)| (*n, v.as_ref() as &dyn rusqlite::types::ToSql))
            .collect();
        st.execute(&bound[..])
    }

    /// Reads a single scalar column-0 value, mirroring `queries!`'s `i64`
    /// return arm (no `FromRow` needed for a bare scalar).
    pub fn query_scalar_i64(&self, conn: &rusqlite::Connection) -> Result<i64, rusqlite::Error> {
        let mut st = conn.prepare(&self.sql)?;
        let bound: Vec<(&str, &dyn rusqlite::types::ToSql)> = self
            .params
            .iter()
            .map(|(n, v)| (*n, v.as_ref() as &dyn rusqlite::types::ToSql))
            .collect();
        st.query_row(&bound[..], |r| r.get(0))
    }
}

/// Read/write classification of a statement, derived from its first significant
/// SQL keyword. `SELECT`/`WITH` route to the read pool; everything else
/// (`INSERT`/`UPDATE`/`DELETE`/`REPLACE`/…) routes to the writer. A write
/// misclassified as a read fails loudly on the read-only read pool, so the
/// conservative default (write) can never silently corrupt.
pub fn is_write_sql(sql: &str) -> bool {
    let word: String = sql
        .trim_start()
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    !(word.eq_ignore_ascii_case("SELECT") || word.eq_ignore_ascii_case("WITH"))
}

/// Converts a borrowed query argument (the shape the sync `{name}_conn` fn
/// accepts) into an owned, `Send + 'static` value that can be moved into the
/// executor closure across an `.await`. This is the owned-mapping rule the
/// `queries!` wrapper relies on, expressed once as a trait rather than
/// per-argument in the macro (macro_rules cannot inspect a `:ty` fragment to
/// branch on it): `&str → String`, `&[u8] → Vec<u8>`, `Option<&T> →
/// Option<Owned>`, and every `Copy` scalar maps to itself.
pub trait IntoOwnedArg {
    type Owned: OwnedArg + Send + 'static;
    fn into_owned_arg(self) -> Self::Owned;
}

/// The owned side of [`IntoOwnedArg`]: reborrows the owned value back into the
/// exact shape the sync `{name}_conn` fn expects, so the wrapper can forward it
/// unchanged (`String → &str`, `Vec<u8> → &[u8]`, `Option<String> →
/// Option<&str>`, `Copy` scalars by value).
pub trait OwnedArg {
    type Borrowed<'a>
    where
        Self: 'a;
    fn borrow_arg(&self) -> Self::Borrowed<'_>;
}

impl IntoOwnedArg for &str {
    type Owned = String;
    fn into_owned_arg(self) -> String {
        self.to_owned()
    }
}
impl OwnedArg for String {
    type Borrowed<'a> = &'a str;
    fn borrow_arg(&self) -> &str {
        self.as_str()
    }
}

impl IntoOwnedArg for &[u8] {
    type Owned = Vec<u8>;
    fn into_owned_arg(self) -> Vec<u8> {
        self.to_vec()
    }
}
impl OwnedArg for Vec<u8> {
    type Borrowed<'a> = &'a [u8];
    fn borrow_arg(&self) -> &[u8] {
        self.as_slice()
    }
}

impl IntoOwnedArg for Option<&str> {
    type Owned = Option<String>;
    fn into_owned_arg(self) -> Option<String> {
        self.map(|s| s.to_owned())
    }
}
impl OwnedArg for Option<String> {
    type Borrowed<'a> = Option<&'a str>;
    fn borrow_arg(&self) -> Option<&str> {
        self.as_deref()
    }
}

impl IntoOwnedArg for Option<&[u8]> {
    type Owned = Option<Vec<u8>>;
    fn into_owned_arg(self) -> Option<Vec<u8>> {
        self.map(|s| s.to_vec())
    }
}
impl OwnedArg for Option<Vec<u8>> {
    type Borrowed<'a> = Option<&'a [u8]>;
    fn borrow_arg(&self) -> Option<&[u8]> {
        self.as_deref()
    }
}

/// Owned-mapping for `Copy` scalars: owned form is the value itself and the
/// reborrow is a copy. `$t` must be `Copy + Send + 'static`.
macro_rules! copy_owned_arg {
    ($($t:ty),* $(,)?) => { $(
        impl IntoOwnedArg for $t {
            type Owned = $t;
            fn into_owned_arg(self) -> $t { self }
        }
        impl OwnedArg for $t {
            type Borrowed<'a> = $t;
            fn borrow_arg(&self) -> $t { *self }
        }
    )* };
}
copy_owned_arg!(
    i32,
    i64,
    bool,
    crate::dbtypes::DbDateTime,
    Option<i32>,
    Option<i64>,
    Option<bool>,
    Option<crate::dbtypes::DbDateTime>,
);

/// Emits, for each query, a sync `{name}_conn(conn, …)` fn and an async
/// `{name}(db, …)` pool-routing wrapper, plus a `QUERIES` const covering them
/// all.
///
/// Return-type arms select the execution strategy for `{name}_conn`: `Option<R>`
/// → `query_row(...).optional()`; `Vec<R>` → `query_map` + collect; `usize` →
/// `execute`; `i64` → `query_row` reading column 0 directly (no `FromRow` needed
/// for a bare scalar); a bare `R` → `query_row` via `FromRow`. Params bind
/// through a `&[(&str, &dyn ToSql)]` slice, with each arg name stringified and
/// `:`-prefixed as its key.
///
/// The async `{name}` wrapper takes the SAME argument shapes as `{name}_conn`
/// (so call sites pass `&str`/scalars directly, never `.to_string()`), converts
/// each to an owned `Send + 'static` value via [`IntoOwnedArg`], moves them into
/// a `'static` closure, reborrows via [`OwnedArg`] to call `{name}_conn`, and
/// routes the closure to `db.read` or `db.write` per [`is_write_sql`] applied to
/// the entry's SQL. Identifier concatenation (`{name}` + `_conn`) is done with
/// `paste!`, reached through the `$crate::paste` re-export.
///
/// Dispatch runs as a token-tree muncher: each query is matched on its concrete
/// return shape and consumed one at a time, threading the accumulated
/// `QueryCheck` entries (as raw tokens) through the `@munch` arms. Matching the
/// shape directly (rather than capturing the return type as a `:ty` fragment and
/// re-matching it later) is required — a captured `:ty` fragment is opaque and
/// cannot be structurally re-matched.
#[macro_export]
macro_rules! queries {
    // Muncher complete: every query consumed, emit the collected registry.
    (@munch [ $($acc:tt)* ]) => {
        pub const QUERIES: &[$crate::registry::QueryCheck] = &[ $($acc)* ];
    };

    // Option<Row> → query_row(...).optional()
    (@munch [ $($acc:tt)* ]
        $(#[$doc:meta])* fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) -> Option<$row:ty> = $sql:literal ;
        $($rest:tt)*
    ) => {
        $crate::paste::paste! {
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub fn [<$name _conn>](conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<Option<$row>, rusqlite::Error> {
                use rusqlite::OptionalExtension;
                let mut st = conn.prepare_cached($sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             <$row as $crate::from_row::FromRow>::from_row)
                  .optional()
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<Option<$row>> {
                $crate::__queries_route!(db, $sql, Option<$row>, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
        }, ] $($rest)*);
    };

    // Vec<Row> → query_map + collect
    (@munch [ $($acc:tt)* ]
        $(#[$doc:meta])* fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) -> Vec<$row:ty> = $sql:literal ;
        $($rest:tt)*
    ) => {
        $crate::paste::paste! {
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub fn [<$name _conn>](conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<Vec<$row>, rusqlite::Error> {
                let mut st = conn.prepare_cached($sql)?;
                let rows = st.query_map(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                                        <$row as $crate::from_row::FromRow>::from_row)?;
                rows.collect()
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<Vec<$row>> {
                $crate::__queries_route!(db, $sql, Vec<$row>, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
        }, ] $($rest)*);
    };

    // usize → execute
    (@munch [ $($acc:tt)* ]
        $(#[$doc:meta])* fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) -> usize = $sql:literal ;
        $($rest:tt)*
    ) => {
        $crate::paste::paste! {
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub fn [<$name _conn>](conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<usize, rusqlite::Error> {
                let mut st = conn.prepare_cached($sql)?;
                st.execute(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)])
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<usize> {
                $crate::__queries_route!(db, $sql, usize, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: None,
        }, ] $($rest)*);
    };

    // i64 → query_row scalar (no FromRow needed — matched before the generic
    // bare-Row arm so a plain `i64` return never falls into `$row:ty` and
    // wrongly demands `FromRow` on `i64`).
    (@munch [ $($acc:tt)* ]
        $(#[$doc:meta])* fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) -> i64 = $sql:literal ;
        $($rest:tt)*
    ) => {
        $crate::paste::paste! {
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub fn [<$name _conn>](conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<i64, rusqlite::Error> {
                let mut st = conn.prepare_cached($sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             |r| r.get(0))
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<i64> {
                $crate::__queries_route!(db, $sql, i64, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: None,
        }, ] $($rest)*);
    };

    // Bare Row → query_row
    (@munch [ $($acc:tt)* ]
        $(#[$doc:meta])* fn $name:ident( $($arg:ident : $aty:ty),* $(,)? ) -> $row:ty = $sql:literal ;
        $($rest:tt)*
    ) => {
        $crate::paste::paste! {
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub fn [<$name _conn>](conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<$row, rusqlite::Error> {
                let mut st = conn.prepare_cached($sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             <$row as $crate::from_row::FromRow>::from_row)
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<$row> {
                $crate::__queries_route!(db, $sql, $row, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
        }, ] $($rest)*);
    };

    // Public entry: kick off the muncher with an empty accumulator.
    ( $($body:tt)+ ) => {
        $crate::queries!(@munch [] $($body)+);
    };
}

/// Wrapper body shared by every `queries!` async arm: own each arg, move them
/// into a `'static` executor closure that reborrows and calls `$conn_fn`, and
/// route the closure to the read or write pool per the SQL class. Kept as a
/// separate `#[macro_export]` rule (not inlined) so the identical body isn't
/// duplicated across the five return-shape arms. `$conn_fn` is already the
/// `paste!`-concatenated `{name}_conn` identifier at the call site.
#[macro_export]
#[doc(hidden)]
macro_rules! __queries_route {
    ($db:ident, $sql:literal, $ret:ty, $conn_fn:ident, $($arg:ident),* $(,)?) => {{
        $( let $arg = $crate::registry::IntoOwnedArg::into_owned_arg($arg); )*
        let __run = move |__conn: &mut rusqlite::Connection| -> $crate::db_error::DbResult<$ret> {
            ::std::result::Result::Ok(
                $conn_fn(&*__conn, $( $crate::registry::OwnedArg::borrow_arg(&$arg) ),*)?
            )
        };
        if $crate::registry::is_write_sql($sql) {
            $db.write(__run).await
        } else {
            $db.read(__run).await
        }
    }};
}
