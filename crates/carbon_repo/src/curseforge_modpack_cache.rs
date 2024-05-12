use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct CurseForgeModpackCache {
    pub project_id: i64,
    pub file_id: i64,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: NaiveDateTime,
}

pub struct CurseForgeModpackCacheRepository {
    pool: SqlitePool,
}

impl CurseForgeModpackCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        CurseForgeModpackCacheRepository { pool }
    }

    pub async fn add_curseforge_modpack_cache(
        &self,
        curseforge_modpack_cache: CurseForgeModpackCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO curseforge_modpack_cache (
                project_id,
                file_id,
                modpack_name,
                version_name,
                url_slug,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?)",
            curseforge_modpack_cache.project_id,
            curseforge_modpack_cache.file_id,
            curseforge_modpack_cache.modpack_name,
            curseforge_modpack_cache.version_name,
            curseforge_modpack_cache.url_slug,
            curseforge_modpack_cache.updated_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_curseforge_modpack_cache(
        &self,
        project_id: i32,
        file_id: i32,
    ) -> Result<CurseForgeModpackCache, sqlx::Error> {
        let curseforge_modpack_cache = sqlx::query_as!(
            CurseForgeModpackCache,
            "SELECT * FROM curseforge_modpack_cache WHERE project_id = ? AND file_id = ?",
            project_id,
            file_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(curseforge_modpack_cache)
    }
}
