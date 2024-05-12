use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct PartialVersionInfoCache {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub partial_version_info: Vec<u8>,
}

pub struct PartialVersionInfoCacheRepository {
    pool: SqlitePool,
}

impl PartialVersionInfoCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        PartialVersionInfoCacheRepository { pool }
    }

    pub async fn add_partial_version_info_cache(
        &self,
        partial_version_info_cache: PartialVersionInfoCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO partial_version_info_cache (
                id,
                last_updated_at,
                partial_version_info
            ) VALUES (?, ?, ?)",
            partial_version_info_cache.id,
            partial_version_info_cache.last_updated_at,
            partial_version_info_cache.partial_version_info
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_partial_version_info_cache(
        &self,
        id: &str,
    ) -> Result<PartialVersionInfoCache, sqlx::Error> {
        let partial_version_info_cache = sqlx::query_as!(
            PartialVersionInfoCache,
            "SELECT * FROM partial_version_info_cache WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(partial_version_info_cache)
    }
}
