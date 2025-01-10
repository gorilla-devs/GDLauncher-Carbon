#!/usr/bin/env node

const Database = require("better-sqlite3")
const fs = require("fs")

// Rust reserved keywords that need to be escaped
const RUST_KEYWORDS = new Set([
  "as",
  "break",
  "const",
  "continue",
  "crate",
  "else",
  "enum",
  "extern",
  "false",
  "fn",
  "for",
  "if",
  "impl",
  "in",
  "let",
  "loop",
  "match",
  "mod",
  "move",
  "mut",
  "pub",
  "ref",
  "return",
  "self",
  "Self",
  "static",
  "struct",
  "super",
  "trait",
  "true",
  "type",
  "unsafe",
  "use",
  "where",
  "while",
  "abstract",
  "async",
  "become",
  "box",
  "do",
  "final",
  "macro",
  "override",
  "priv",
  "try",
  "typeof",
  "unsized",
  "virtual",
  "yield"
])

function escapeRustKeyword(name) {
  if (RUST_KEYWORDS.has(name)) {
    return `_${name}`
  }
  return name
}

function sqliteTypeToRust(sqliteType, notNull) {
  const type = sqliteType.toUpperCase()
  let rustType

  if (type.includes("INTEGER")) {
    rustType = type.includes("UNSIGNED") ? "u64" : "i64"
  } else if (type.includes("TEXT")) {
    rustType = "String"
  } else if (type.includes("BLOB")) {
    rustType = "Vec<u8>"
  } else if (type.includes("REAL") || type.includes("FLOAT")) {
    rustType = "f64"
  } else if (type.includes("BOOLEAN")) {
    rustType = "bool"
  } else {
    rustType = "String" // Default to String for unknown types
  }

  return notNull ? rustType : `Option<${rustType}>`
}

function snakeToPascalCase(str) {
  return str
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
    .join("")
}

function camelToSnakeCase(str) {
  return str
    .replace(/([A-Z])/g, "_$1")
    .toLowerCase()
    .replace(/^_/, "")
}

function getForeignKeys(db, tableName) {
  return db.prepare(`PRAGMA foreign_key_list(${tableName})`).all()
}

function getIndices(db, tableName) {
  return db.prepare(`PRAGMA index_list(${tableName})`).all()
}

function getTableInfo(db, tableName) {
  const columns = db.prepare(`PRAGMA table_info(${tableName})`).all()
  const foreignKeys = getForeignKeys(db, tableName)
  const indices = getIndices(db, tableName)

  return columns.map((col) => {
    const fk = foreignKeys.find((fk) => fk.from === col.name)
    const index = indices.find((idx) => {
      const indexInfo = db.prepare(`PRAGMA index_info(${idx.name})`).all()
      return indexInfo.some((ii) => ii.name === col.name)
    })

    return {
      name: col.name,
      type: col.type,
      notNull: col.notnull === 1,
      primaryKey: col.pk === 1,
      hasIndex: !!index,
      foreignKey: fk
        ? {
            table: fk.table,
            column: fk.to
          }
        : null
    }
  })
}

function generateRustStruct(tableName, columns) {
  const structName = snakeToPascalCase(tableName)
  const lines = ["#[derive(Debug, Clone)]", `pub struct ${structName} {`]

  // Regular fields
  columns.forEach(
    ({ name, type, notNull, primaryKey, foreignKey, hasIndex }) => {
      let fieldName = camelToSnakeCase(name)
      fieldName = escapeRustKeyword(fieldName)
      let rustType = sqliteTypeToRust(type, notNull)

      // Add field documentation
      const docs = []
      if (primaryKey) docs.push("Primary key")
      if (foreignKey)
        docs.push(`Foreign key to ${foreignKey.table}.${foreignKey.column}`)
      if (hasIndex) docs.push("Indexed")

      if (docs.length > 0) {
        lines.push(`    /// ${docs.join(". ")}`)
      }

      lines.push(`    pub ${fieldName}: ${rustType},`)
    }
  )

  lines.push("}")

  // Add column names as constants
  lines.push("")
  lines.push(`impl ${structName} {`)

  // Add table name constant
  lines.push(`    pub const TABLE: &'static str = "${tableName}";`)

  // Add column constants
  columns.forEach(({ name }) => {
    const constName = name.toUpperCase()
    lines.push(`    pub const ${constName}: &'static str = "${name}";`)
  })

  // Add constructor
  lines.push("")
  lines.push("    #[allow(clippy::too_many_arguments)]")
  lines.push("    pub fn new(")
  columns.forEach(({ name, type, notNull }) => {
    let fieldName = camelToSnakeCase(name)
    fieldName = escapeRustKeyword(fieldName)
    let rustType = sqliteTypeToRust(type, notNull)
    lines.push(`        ${fieldName}: ${rustType},`)
  })
  lines.push(`    ) -> ${structName} {`)
  lines.push(`        ${structName} {`)
  columns.forEach(({ name }) => {
    let fieldName = camelToSnakeCase(name)
    fieldName = escapeRustKeyword(fieldName)
    lines.push(`            ${fieldName},`)
  })
  lines.push("        }")
  lines.push("    }")

  // Add from_row implementation that uses column names
  lines.push("")
  lines.push(
    "    pub fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {"
  )
  lines.push("        Ok(Self {")
  columns.forEach(({ name }) => {
    let fieldName = camelToSnakeCase(name)
    fieldName = escapeRustKeyword(fieldName)
    lines.push(
      `            ${fieldName}: row.get(Self::${name.toUpperCase()})?,`
    )
  })
  lines.push("        })")
  lines.push("    }")

  // Add column list method
  lines.push("")
  lines.push("    pub fn columns() -> &'static [&'static str] {")
  lines.push("        &[")
  columns.forEach(({ name }) => {
    lines.push(`            Self::${name.toUpperCase()},`)
  })
  lines.push("        ]")
  lines.push("    }")

  // Add get_by_id method using column names
  const primaryKey = columns.find((col) => col.primaryKey)
  if (primaryKey) {
    const pkConst = primaryKey.name.toUpperCase()
    const pkType = sqliteTypeToRust(primaryKey.type, primaryKey.notNull)

    lines.push("")
    lines.push(
      `    pub fn get_by_id(conn: &rusqlite::Connection, id: ${pkType}) -> rusqlite::Result<Option<Self>> {`
    )
    lines.push(`        let columns = Self::columns().join(", ");`)
    lines.push(
      `        let sql = format!("SELECT {} FROM {} WHERE {} = ?", columns, Self::TABLE, Self::${pkConst});`
    )
    lines.push("        let mut stmt = conn.prepare(&sql)?;")
    lines.push("        let mut rows = stmt.query([id])?;")
    lines.push("")
    lines.push("        if let Some(row) = rows.next()? {")
    lines.push("            Ok(Some(Self::from_row(row)?))")
    lines.push("        } else {")
    lines.push("            Ok(None)")
    lines.push("        }")
    lines.push("    }")
  }

  // Add get_all method using column names
  lines.push("")
  lines.push(
    "    pub fn get_all(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<Self>> {"
  )
  lines.push(`        let columns = Self::columns().join(", ");`)
  lines.push(
    `        let sql = format!("SELECT {} FROM {}", columns, Self::TABLE);`
  )
  lines.push("        let mut stmt = conn.prepare(&sql)?;")
  lines.push("        let rows = stmt.query_map([], Self::from_row)?;")
  lines.push("        rows.collect()")
  lines.push("    }")

  // Generate methods for indexed columns
  columns.forEach(({ name, type, notNull, hasIndex }) => {
    if (hasIndex && !primaryKey) {
      const methodName = `get_by_${camelToSnakeCase(name)}`
      const paramType = sqliteTypeToRust(type, notNull)
      const constName = name.toUpperCase()

      lines.push("")
      lines.push(
        `    pub fn ${methodName}(conn: &rusqlite::Connection, value: ${paramType}) -> rusqlite::Result<Vec<Self>> {`
      )
      lines.push(`        let columns = Self::columns().join(", ");`)
      lines.push(
        `        let sql = format!("SELECT {} FROM {} WHERE {} = ?", columns, Self::TABLE, Self::${constName});`
      )
      lines.push("        let mut stmt = conn.prepare(&sql)?;")
      lines.push("        let rows = stmt.query_map([value], Self::from_row)?;")
      lines.push("        rows.collect()")
      lines.push("    }")
    }
  })

  lines.push("}")

  return lines.join("\n")
}

function generateModuleStructure(rustCode) {
  return [
    "//! Generated Rust types from SQLite schema",
    "//! Do not edit manually",
    "",
    "use rusqlite;",
    "",
    rustCode
  ].join("\n")
}

function generateRustTypes(dbPath) {
  try {
    const db = new Database(dbPath)

    // Get all table names
    const tables = db
      .prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
      )
      .all()

    const rustCode = []

    // Generate main structs
    tables.forEach(({ name: tableName }) => {
      const columns = getTableInfo(db, tableName)
      rustCode.push(generateRustStruct(tableName, columns))
      rustCode.push("") // Add blank line between structs
    })

    db.close()
    return generateModuleStructure(rustCode.join("\n"))
  } catch (error) {
    console.error("Error:", error.message)
    process.exit(1)
  }
}

// CLI handling
if (require.main === module) {
  if (process.argv.length !== 3) {
    console.log("Usage: node sqlite_to_rust.js <path_to_sqlite_db>")
    process.exit(1)
  }

  const dbPath = process.argv[2]

  if (!fs.existsSync(dbPath)) {
    console.error(`Error: Database file '${dbPath}' does not exist`)
    process.exit(1)
  }

  const rustCode = generateRustTypes(dbPath)
  console.log(rustCode)
}

module.exports = {
  generateRustTypes,
  sqliteTypeToRust,
  generateRustStruct
}
