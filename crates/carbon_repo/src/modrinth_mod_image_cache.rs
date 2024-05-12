use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModrinthModImageCache {
    pub metadata_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
    pub up_to_date: i64,
}

pub struct ModrinthModImageCacheRepository {
    pool: SqlitePool,
}

impl ModrinthModImageCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModrinthModImageCacheRepository { pool }
    }

    pub async fn add_modrinth_mod_image_cache(
        &self,
        modrinth_mod_image_cache: ModrinthModImageCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO modrinth_mod_image_cache (
                metadata_id,
                url,
                data,
                up_to_date
            ) VALUES (?, ?, ?, ?)",
            modrinth_mod_image_cache.metadata_id,
            modrinth_mod_image_cache.url,
            modrinth_mod_image_cache.data,
            modrinth_mod_image_cache.up_to_date,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_modrinth_mod_image_cache(
        &self,
        metadata_id: &str,
    ) -> Result<ModrinthModImageCache, sqlx::Error> {
        let modrinth_mod_image_cache = sqlx::query_as!(
            ModrinthModImageCache,
            "SELECT * FROM modrinth_mod_image_cache WHERE metadata_id = ?",
            metadata_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(modrinth_mod_image_cache)
    }
}
