use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModMetadata {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub murmur2: i64,
    pub sha512: Vec<u8>,
    pub sha1: Vec<u8>,
    pub name: Option<String>,
    pub modid: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Option<String>,
    pub modloaders: String,
}

pub struct ModMetadataRepository {
    pool: SqlitePool,
}

impl ModMetadataRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModMetadataRepository { pool }
    }

    pub async fn add_mod_metadata(&self, mod_metadata: ModMetadata) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mod_metadata (
                id,
                last_updated_at,
                murmur2,
                sha512,
                sha1,
                name,
                modid,
                version,
                description,
                authors,
                modloaders
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            mod_metadata.id,
            mod_metadata.last_updated_at,
            mod_metadata.murmur2,
            mod_metadata.sha512,
            mod_metadata.sha1,
            mod_metadata.name,
            mod_metadata.modid,
            mod_metadata.version,
            mod_metadata.description,
            mod_metadata.authors,
            mod_metadata.modloaders,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_mod_metadata(&self, id: &str) -> Result<ModMetadata, sqlx::Error> {
        let mod_metadata =
            sqlx::query_as!(ModMetadata, "SELECT * FROM mod_metadata WHERE id = ?", id)
                .fetch_one(&self.pool)
                .await?;

        Ok(mod_metadata)
    }
}
