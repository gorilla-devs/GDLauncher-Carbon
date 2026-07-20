//! Compile-time query registry.
//!
//! The `queries!` macro emits a typed wrapper `fn` per entry plus a `QUERIES`
//! const describing every query (name, SQL, param names, and — for row-returning
//! queries — the expected columns), which the schema checker consumes.

use crate::from_row::ColumnSpec;

pub struct QueryCheck {
    pub name: &'static str,
    pub sql: &'static str,
    pub params: &'static [&'static str],
    pub columns: Option<&'static [ColumnSpec]>,
}

/// Emits a typed wrapper `fn` for each query and a `QUERIES` const covering them
/// all.
///
/// Return-type arms select the execution strategy: `Option<R>` →
/// `query_row(...).optional()`; `Vec<R>` → `query_map` + collect; `usize` →
/// `execute`; a bare `R` → `query_row`. Params bind through a
/// `&[(&str, &dyn ToSql)]` slice, with each arg name stringified and
/// `:`-prefixed as its key.
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
