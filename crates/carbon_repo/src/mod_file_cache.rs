use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModFileCache {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub instance_id: i64,
    pub filename: String,
    pub filesize: i64,
    pub enabled: bool,
    pub metadata_id: String,
}

pub struct ModFileCacheRepository {
    pool: SqlitePool,
}

impl ModFileCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModFileCacheRepository { pool }
    }

    pub async fn add_mod_file_cache(
        &self,
        mod_file_cache: ModFileCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO mod_file_cache (
                id,
                last_updated_at,
                instance_id,
                filename,
                filesize,
                enabled,
                metadata_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            mod_file_cache.id,
            mod_file_cache.last_updated_at,
            mod_file_cache.instance_id,
            mod_file_cache.filename,
            mod_file_cache.filesize,
            mod_file_cache.enabled,
            mod_file_cache.metadata_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_mod_file_cache(&self, id: &str) -> Result<ModFileCache, sqlx::Error> {
        let mod_file_cache = sqlx::query_as!(
            ModFileCache,
            "SELECT * FROM mod_file_cache WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(mod_file_cache)
    }
}
