use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModrinthModpackCache {
    pub project_id: String,
    pub version_id: String,
    pub modpack_name: String,
    pub version_name: String,
    pub url_slug: String,
    pub updated_at: chrono::NaiveDateTime,
}

pub struct ModrinthModpackCacheRepository {
    pool: SqlitePool,
}

impl ModrinthModpackCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModrinthModpackCacheRepository { pool }
    }

    pub async fn add_modrinth_modpack_cache(
        &self,
        modrinth_modpack_cache: ModrinthModpackCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO modrinth_modpack_cache (
                project_id,
                version_id,
                modpack_name,
                version_name,
                url_slug,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?)",
            modrinth_modpack_cache.project_id,
            modrinth_modpack_cache.version_id,
            modrinth_modpack_cache.modpack_name,
            modrinth_modpack_cache.version_name,
            modrinth_modpack_cache.url_slug,
            modrinth_modpack_cache.updated_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_modrinth_modpack_cache(
        &self,
        project_id: String,
        version_id: String,
    ) -> Result<ModrinthModpackCache, sqlx::Error> {
        let modrinth_modpack_cache = sqlx::query_as!(
            ModrinthModpackCache,
            "SELECT * FROM modrinth_modpack_cache WHERE project_id = ? AND version_id = ?",
            project_id,
            version_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(modrinth_modpack_cache)
    }
}
