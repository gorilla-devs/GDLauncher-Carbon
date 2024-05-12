use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModrinthModpackImageCache {
    pub project_id: String,
    pub version_id: String,
    pub url: String,
    pub data: Option<Vec<u8>>,
}

pub struct ModrinthModpackImageCacheRepository {
    pool: SqlitePool,
}

impl ModrinthModpackImageCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModrinthModpackImageCacheRepository { pool }
    }

    pub async fn add_modrinth_modpack_image_cache(
        &self,
        modrinth_modpack_image_cache: ModrinthModpackImageCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO modrinth_modpack_image_cache (
                project_id,
                version_id,
                url,
                data
            ) VALUES (?, ?, ?, ?)",
            modrinth_modpack_image_cache.project_id,
            modrinth_modpack_image_cache.version_id,
            modrinth_modpack_image_cache.url,
            modrinth_modpack_image_cache.data,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_modrinth_modpack_image_cache(
        &self,
        project_id: String,
        version_id: String,
    ) -> Result<ModrinthModpackImageCache, sqlx::Error> {
        let modrinth_modpack_image_cache = sqlx::query_as!(
            ModrinthModpackImageCache,
            "SELECT * FROM modrinth_modpack_image_cache WHERE project_id = ? AND version_id = ?",
            project_id,
            version_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(modrinth_modpack_image_cache)
    }
}
