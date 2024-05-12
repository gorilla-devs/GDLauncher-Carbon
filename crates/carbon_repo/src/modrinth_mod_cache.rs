use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ModrinthModCache {
    pub metadata_id: String,
    pub sha512: String,
    pub project_id: String,
    pub version_id: String,
    pub title: String,
    pub version: String,
    pub urlslug: String,
    pub description: String,
    pub authors: String,
    pub release_type: i64,    // alpha = 0, beta = 1, stable = 2
    pub update_paths: String, // in the form `<gamever>,<modloader>,<channel>;<gamever>,<modloader>,<channel>` for every available combination
    pub filename: String,
    pub file_url: String,
    pub cached_at: NaiveDateTime,
}

pub struct ModrinthModCacheRepository {
    pool: SqlitePool,
}

impl ModrinthModCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ModrinthModCacheRepository { pool }
    }

    pub async fn add_modrinth_mod_cache(
        &self,
        modrinth_mod_cache: ModrinthModCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO modrinth_mod_cache (
                metadata_id,
                sha512,
                project_id,
                version_id,
                title,
                version,
                urlslug,
                description,
                authors,
                release_type,
                update_paths,
                filename,
                file_url,
                cached_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            modrinth_mod_cache.metadata_id,
            modrinth_mod_cache.sha512,
            modrinth_mod_cache.project_id,
            modrinth_mod_cache.version_id,
            modrinth_mod_cache.title,
            modrinth_mod_cache.version,
            modrinth_mod_cache.urlslug,
            modrinth_mod_cache.description,
            modrinth_mod_cache.authors,
            modrinth_mod_cache.release_type,
            modrinth_mod_cache.update_paths,
            modrinth_mod_cache.filename,
            modrinth_mod_cache.file_url,
            modrinth_mod_cache.cached_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_modrinth_mod_cache(
        &self,
        metadata_id: &str,
    ) -> Result<ModrinthModCache, sqlx::Error> {
        let modrinth_mod_cache = sqlx::query_as!(
            ModrinthModCache,
            "SELECT * FROM modrinth_mod_cache WHERE metadata_id = ?",
            metadata_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(modrinth_mod_cache)
    }
}
