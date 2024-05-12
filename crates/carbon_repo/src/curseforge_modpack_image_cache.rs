use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct CurseForgeModpackImageCache {
    pub project_id: i64,
    pub file_id: i64,
    pub url: String,
    pub data: Option<Vec<u8>>,
}

pub struct CurseForgeModpackImageCacheRepository {
    pool: SqlitePool,
}

impl CurseForgeModpackImageCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        CurseForgeModpackImageCacheRepository { pool }
    }

    pub async fn add_curseforge_modpack_image_cache(
        &self,
        curseforge_modpack_image_cache: CurseForgeModpackImageCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO curseforge_modpack_image_cache (
                project_id,
                file_id,
                url,
                data
            ) VALUES (?, ?, ?, ?)",
            curseforge_modpack_image_cache.project_id,
            curseforge_modpack_image_cache.file_id,
            curseforge_modpack_image_cache.url,
            curseforge_modpack_image_cache.data,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_curseforge_modpack_image_cache(
        &self,
        project_id: i64,
        file_id: i64,
    ) -> Result<CurseForgeModpackImageCache, sqlx::Error> {
        let curseforge_modpack_image_cache = sqlx::query_as!(
            CurseForgeModpackImageCache,
            "SELECT * FROM curseforge_modpack_image_cache WHERE project_id = ? AND file_id = ?",
            project_id,
            file_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(curseforge_modpack_image_cache)
    }
}
