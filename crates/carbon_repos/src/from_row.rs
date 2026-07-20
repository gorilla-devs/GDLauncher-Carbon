//! Name-based row mapping for repository row structs.
//!
//! `#[derive(FromRow)]` (in `carbon_macro`) generates a `from_row` that reads
//! each column by name (never by positional index) and a `COLUMNS` constant
//! describing the expected schema, which the schema checker consumes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeClass {
    Integer,
    Real,
    Text,
    Blob,
    DateTime,
    Bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnSpec {
    pub name: &'static str,
    pub ty: TypeClass,
    pub nullable: bool,
}

pub trait FromRow: Sized {
    const COLUMNS: &'static [ColumnSpec];
    fn from_row(row: &rusqlite::Row<'_>) -> Result<Self, rusqlite::Error>;
}
