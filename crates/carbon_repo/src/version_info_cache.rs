use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct VersionInfoCache {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub version_info: Vec<u8>,
}

pub struct VersionInfoCacheRepository {
    pool: SqlitePool,
}

impl VersionInfoCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        VersionInfoCacheRepository { pool }
    }

    pub async fn add_version_info_cache(
        &self,
        version_info_cache: VersionInfoCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO version_info_cache (
                id,
                last_updated_at,
                version_info
            ) VALUES (?, ?, ?)",
            version_info_cache.id,
            version_info_cache.last_updated_at,
            version_info_cache.version_info
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_version_info_cache(&self, id: &str) -> Result<VersionInfoCache, sqlx::Error> {
        let version_info_cache = sqlx::query_as!(
            VersionInfoCache,
            "SELECT * FROM version_info_cache WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(version_info_cache)
    }
}
