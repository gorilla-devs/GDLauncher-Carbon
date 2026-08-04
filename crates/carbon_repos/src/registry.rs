//! Compile-time query registry.
//!
//! The `queries!` macro emits, per entry, a sync `{name}_conn(conn, …)` fn plus
//! an async `{name}(db, …)` pool-routing wrapper, and a `QUERIES` const
//! describing every query (name, SQL, param names, and — for row-returning
//! queries — the expected columns), which the schema checker consumes.

use crate::from_row::ColumnSpec;

/// Read/write classification of a registered statement, derived at compile time
/// from its leading SQL verb (see [`class_of`]). Drives which pool the async
/// wrapper routes to and lets the checker prove a `Read`-classified query never
/// writes (the manifest-lock rule).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryClass {
    /// `SELECT`/`WITH` — routed to the read-only pool.
    Read,
    /// Everything else (`INSERT`/`UPDATE`/`DELETE`/`REPLACE`/…) — routed to the
    /// writer.
    Write,
}

#[derive(Debug, Clone, Copy)]
pub struct QueryCheck {
    pub name: &'static str,
    pub sql: &'static str,
    pub params: &'static [&'static str],
    pub columns: Option<&'static [ColumnSpec]>,
    /// Read/write class, derived from `sql`'s leading verb via [`class_of`].
    pub class: QueryClass,
}

/// Escape hatch for runtime-assembled SQL. Exempt from the static checker;
/// every construction site must have a dedicated execution test.
pub struct DynamicQuery {
    pub sql: String,
    pub params: Vec<(&'static str, Box<dyn rusqlite::types::ToSql + Send>)>,
}

impl DynamicQuery {
    /// Executes the runtime-assembled write. Takes `&impl WriteAccess` so it can
    /// only run through a write guard; the runtime SQL is prepared through the
    /// guard's `raw()` escape (this is the `DynamicQuery` exemption the
    /// hand-written-SQL census carves out).
    pub fn execute(
        &self,
        conn: &impl crate::db_exec::WriteAccess,
    ) -> Result<usize, rusqlite::Error> {
        let mut st = conn.raw().prepare(&self.sql)?;
        let bound: Vec<(&str, &dyn rusqlite::types::ToSql)> = self
            .params
            .iter()
            .map(|(n, v)| (*n, v.as_ref() as &dyn rusqlite::types::ToSql))
            .collect();
        st.execute(&bound[..])
    }

    /// Reads a single scalar column-0 value, mirroring `queries!`'s `i64`
    /// return arm (no `FromRow` needed for a bare scalar). Takes `&impl
    /// ReadAccess` — the read path never needs write access.
    pub fn query_scalar_i64(
        &self,
        conn: &impl crate::db_exec::ReadAccess,
    ) -> Result<i64, rusqlite::Error> {
        let mut st = conn.raw().prepare(&self.sql)?;
        let bound: Vec<(&str, &dyn rusqlite::types::ToSql)> = self
            .params
            .iter()
            .map(|(n, v)| (*n, v.as_ref() as &dyn rusqlite::types::ToSql))
            .collect();
        st.query_row(&bound[..], |r| r.get(0))
    }
}

/// True when `sql`'s leading verb is not `SELECT`/`WITH` — i.e. it writes.
/// `SELECT`/`WITH` route to the read pool; everything else
/// (`INSERT`/`UPDATE`/`DELETE`/`REPLACE`/…) routes to the writer. A write
/// misclassified as a read fails loudly on the read-only read pool, so the
/// conservative default (write) can never silently corrupt.
///
/// `const` so [`class_of`] can fill [`QueryCheck::class`] at compile time.
pub const fn is_write_sql(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0;
    // Skip leading ASCII whitespace.
    while i < bytes.len() {
        let c = bytes[i];
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
            i += 1;
        } else {
            break;
        }
    }
    let start = i;
    // Read the leading ASCII-alphabetic word.
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() {
            i += 1;
        } else {
            break;
        }
    }
    let len = i - start;
    !(region_eq_ignore_case(bytes, start, len, b"SELECT")
        || region_eq_ignore_case(bytes, start, len, b"WITH"))
}

/// Case-insensitive ASCII comparison of `bytes[start..start+len]` against `kw`,
/// usable in `const` context (avoids const-unstable slice ops).
const fn region_eq_ignore_case(bytes: &[u8], start: usize, len: usize, kw: &[u8]) -> bool {
    if len != kw.len() {
        return false;
    }
    let mut j = 0;
    while j < len {
        if bytes[start + j].to_ascii_uppercase() != kw[j].to_ascii_uppercase() {
            return false;
        }
        j += 1;
    }
    true
}

/// Classifies a statement by its leading SQL verb. `const` so the `queries!`
/// macro and hand-written `QueryCheck`s can fill [`QueryCheck::class`] at
/// compile time.
pub const fn class_of(sql: &str) -> QueryClass {
    if is_write_sql(sql) {
        QueryClass::Write
    } else {
        QueryClass::Read
    }
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
/// routes the closure to `db.read` (read-shaped arms) or `db.write` (the
/// `usize`/`execute` arm) — the pool matching the entry's class, since the
/// return shape coincides with the SQL verb. Identifier concatenation (`{name}`
/// + `_conn`) is done with `paste!`, reached through the `$crate::paste`
/// re-export.
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
            pub fn [<$name _conn>](conn: &impl $crate::db_exec::ReadAccess, $($arg : $aty),*) -> Result<Option<$row>, rusqlite::Error> {
                use rusqlite::OptionalExtension;
                let mut st = $crate::db_exec::ReadAccess::prepare_cached(conn, $sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             <$row as $crate::from_row::FromRow>::from_row)
                  .optional()
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<Option<$row>> {
                $crate::__queries_route!(read, db, Option<$row>, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
            class: $crate::registry::class_of($sql),
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
            pub fn [<$name _conn>](conn: &impl $crate::db_exec::ReadAccess, $($arg : $aty),*) -> Result<Vec<$row>, rusqlite::Error> {
                let mut st = $crate::db_exec::ReadAccess::prepare_cached(conn, $sql)?;
                let rows = st.query_map(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                                        <$row as $crate::from_row::FromRow>::from_row)?;
                rows.collect()
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<Vec<$row>> {
                $crate::__queries_route!(read, db, Vec<$row>, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
            class: $crate::registry::class_of($sql),
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
            pub fn [<$name _conn>](conn: &impl $crate::db_exec::WriteAccess, $($arg : $aty),*) -> Result<usize, rusqlite::Error> {
                let mut st = $crate::db_exec::ReadAccess::prepare_cached(conn, $sql)?;
                st.execute(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)])
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<usize> {
                $crate::__queries_route!(write, db, usize, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: None,
            class: $crate::registry::class_of($sql),
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
            pub fn [<$name _conn>](conn: &impl $crate::db_exec::ReadAccess, $($arg : $aty),*) -> Result<i64, rusqlite::Error> {
                let mut st = $crate::db_exec::ReadAccess::prepare_cached(conn, $sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             |r| r.get(0))
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<i64> {
                $crate::__queries_route!(read, db, i64, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: None,
            class: $crate::registry::class_of($sql),
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
            pub fn [<$name _conn>](conn: &impl $crate::db_exec::ReadAccess, $($arg : $aty),*) -> Result<$row, rusqlite::Error> {
                let mut st = $crate::db_exec::ReadAccess::prepare_cached(conn, $sql)?;
                st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                             <$row as $crate::from_row::FromRow>::from_row)
            }
            $(#[$doc])*
            #[allow(clippy::too_many_arguments)]
            pub async fn $name(db: &$crate::db_exec::Db, $($arg : $aty),*) -> $crate::db_error::DbResult<$row> {
                $crate::__queries_route!(read, db, $row, [<$name _conn>], $($arg),*)
            }
        }
        $crate::queries!(@munch [ $($acc)* $crate::registry::QueryCheck {
            name: stringify!($name),
            sql: $sql,
            params: &[ $( concat!(":", stringify!($arg)) ),* ],
            columns: Some(<$row as $crate::from_row::FromRow>::COLUMNS),
            class: $crate::registry::class_of($sql),
        }, ] $($rest)*);
    };

    // Public entry: kick off the muncher with an empty accumulator.
    ( $($body:tt)+ ) => {
        $crate::queries!(@munch [] $($body)+);
    };
}

/// Wrapper body shared by every `queries!` async arm: own each arg, move them
/// into a `'static` executor closure that reborrows and calls `$conn_fn`, and
/// route the closure to the read or write pool. The pool is chosen by the
/// leading `read`/`write` selector — the return-shape arm that expands this
/// already knows the class (read arms select `read`; the `usize`/`execute` arm
/// selects `write`), so it matches the SQL-verb classification without a runtime
/// check. Read closures receive a `ReadGuard`, write closures a `WriteGuard`;
/// `$conn_fn` takes `&impl ReadAccess`/`&impl WriteAccess` accordingly. Kept as a
/// separate `#[macro_export]` rule (not inlined) so the identical body isn't
/// duplicated across the five return-shape arms. `$conn_fn` is already the
/// `paste!`-concatenated `{name}_conn` identifier at the call site.
#[macro_export]
#[doc(hidden)]
macro_rules! __queries_route {
    (read, $db:ident, $ret:ty, $conn_fn:ident, $($arg:ident),* $(,)?) => {{
        $( let $arg = $crate::registry::IntoOwnedArg::into_owned_arg($arg); )*
        $db.read(move |__guard| -> $crate::db_error::DbResult<$ret> {
            ::std::result::Result::Ok(
                $conn_fn(&__guard, $( $crate::registry::OwnedArg::borrow_arg(&$arg) ),*)?
            )
        }).await
    }};
    (write, $db:ident, $ret:ty, $conn_fn:ident, $($arg:ident),* $(,)?) => {{
        $( let $arg = $crate::registry::IntoOwnedArg::into_owned_arg($arg); )*
        $db.write(move |__guard| -> $crate::db_error::DbResult<$ret> {
            ::std::result::Result::Ok(
                $conn_fn(&__guard, $( $crate::registry::OwnedArg::borrow_arg(&$arg) ),*)?
            )
        }).await
    }};
}
