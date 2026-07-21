//! Compile-time query registry.
//!
//! The `queries!` macro emits a typed wrapper `fn` per entry plus a `QUERIES`
//! const describing every query (name, SQL, param names, and — for row-returning
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

/// Emits a typed wrapper `fn` for each query and a `QUERIES` const covering them
/// all.
///
/// Return-type arms select the execution strategy: `Option<R>` →
/// `query_row(...).optional()`; `Vec<R>` → `query_map` + collect; `usize` →
/// `execute`; `i64` → `query_row` reading column 0 directly (no `FromRow`
/// needed for a bare scalar); a bare `R` → `query_row` via `FromRow`. Params
/// bind through a `&[(&str, &dyn ToSql)]` slice, with each arg name
/// stringified and `:`-prefixed as its key.
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
        $(#[$doc])*
        pub fn $name(conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<Option<$row>, rusqlite::Error> {
            use rusqlite::OptionalExtension;
            let mut st = conn.prepare_cached($sql)?;
            st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                         <$row as $crate::from_row::FromRow>::from_row)
              .optional()
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
        $(#[$doc])*
        pub fn $name(conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<Vec<$row>, rusqlite::Error> {
            let mut st = conn.prepare_cached($sql)?;
            let rows = st.query_map(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                                    <$row as $crate::from_row::FromRow>::from_row)?;
            rows.collect()
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
        $(#[$doc])*
        pub fn $name(conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<usize, rusqlite::Error> {
            let mut st = conn.prepare_cached($sql)?;
            st.execute(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)])
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
        $(#[$doc])*
        pub fn $name(conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<i64, rusqlite::Error> {
            let mut st = conn.prepare_cached($sql)?;
            st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                         |r| r.get(0))
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
        $(#[$doc])*
        pub fn $name(conn: &rusqlite::Connection, $($arg : $aty),*) -> Result<$row, rusqlite::Error> {
            let mut st = conn.prepare_cached($sql)?;
            st.query_row(&[ $( (concat!(":", stringify!($arg)), &$arg as &dyn rusqlite::ToSql) ),* ] as &[(&str, &dyn rusqlite::ToSql)],
                         <$row as $crate::from_row::FromRow>::from_row)
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
