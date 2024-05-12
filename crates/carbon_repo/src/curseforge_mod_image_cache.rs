use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct CurseForgeModImageCache {
    pub metadata_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
    pub up_to_date: i64,
}

pub struct CurseForgeModImageCacheRepository {
    pool: SqlitePool,
}

impl CurseForgeModImageCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        CurseForgeModImageCacheRepository { pool }
    }

    pub async fn add_curse_forge_mod_image_cache(
        &self,
        curse_forge_mod_image_cache: CurseForgeModImageCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO curseforge_mod_image_cache (
                metadata_id,
                url,
                data,
                up_to_date
            ) VALUES (?, ?, ?, ?)",
            curse_forge_mod_image_cache.metadata_id,
            curse_forge_mod_image_cache.url,
            curse_forge_mod_image_cache.data,
            curse_forge_mod_image_cache.up_to_date,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_curse_forge_mod_image_cache(
        &self,
        metadata_id: &str,
    ) -> Result<CurseForgeModImageCache, sqlx::Error> {
        let curse_forge_mod_image_cache = sqlx::query_as!(
            CurseForgeModImageCache,
            "SELECT * FROM curseforge_mod_image_cache WHERE metadata_id = ?",
            metadata_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(curse_forge_mod_image_cache)
    }
}
