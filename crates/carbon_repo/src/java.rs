use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct Java {
    pub id: String,
    pub path: String,
    pub major: i64, // Assuming Int maps to i64 in your environment
    pub full_version: String,
    pub type_: String, // Renamed to `type_` because `type` is a reserved keyword in Rust
    pub os: String,
    pub arch: String,
    pub vendor: String,
    pub is_valid: bool,
}

pub struct JavaRepository {
    pool: SqlitePool,
}

impl JavaRepository {
    pub fn new(pool: SqlitePool) -> Self {
        JavaRepository { pool }
    }

    pub async fn add_java(&self, java: Java) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO java (
                id,
                path,
                major,
                full_version,
                type,
                os,
                arch,
                vendor,
                is_valid
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            java.id,
            java.path,
            java.major,
            java.full_version,
            java.type_,
            java.os,
            java.arch,
            java.vendor,
            java.is_valid,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_java(&self, id: String) -> Result<Java, sqlx::Error> {
        let java = sqlx::query_as!(Java, "SELECT id, path, major, full_version, type as type_, os, arch, vendor, is_valid FROM java WHERE id = ?", id)
            .fetch_one(&self.pool)
            .await?;

        Ok(java)
    }
}
