use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct AssetsMetaCache {
    pub id: String,
    pub last_updated_at: NaiveDateTime,
    pub assets_index: Vec<u8>,
}

pub struct AssetsMetaCacheRepository {
    pool: SqlitePool,
}

impl AssetsMetaCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        AssetsMetaCacheRepository { pool }
    }

    pub async fn add_assets_meta_cache(
        &self,
        assets_meta_cache: AssetsMetaCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO assets_meta_cache (
                id,
                last_updated_at,
                assets_index
            ) VALUES (?, ?, ?)",
            assets_meta_cache.id,
            assets_meta_cache.last_updated_at,
            assets_meta_cache.assets_index
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_assets_meta_cache(&self, id: &str) -> Result<AssetsMetaCache, sqlx::Error> {
        let assets_meta_cache = sqlx::query_as!(
            AssetsMetaCache,
            "SELECT * FROM assets_meta_cache WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(assets_meta_cache)
    }
}
