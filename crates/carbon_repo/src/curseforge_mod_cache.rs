use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct CurseForgeModCache {
    pub metadata_id: String,
    pub murmur2: i64,
    pub project_id: i64,
    pub file_id: i64,
    pub name: String,
    pub version: String,
    pub urlslug: String,
    pub summary: String,
    pub authors: String,
    pub release_type: i64,    // alpha = 0, beta = 1, stable = 2
    pub update_paths: String, // in the form `<gamever>,<modloader>,<channel>;<gamever>,<modloader>,<channel>` for every available combination
    pub cached_at: NaiveDateTime,
}

pub struct CurseForgeModCacheRepository {
    pool: SqlitePool,
}

impl CurseForgeModCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        CurseForgeModCacheRepository { pool }
    }

    pub async fn add_curseforge_mod_cache(
        &self,
        curseforge_mod_cache: CurseForgeModCache,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO curseforge_mod_cache (
                metadata_id,
                murmur2,
                project_id,
                file_id,
                name,
                version,
                urlslug,
                summary,
                authors,
                release_type,
                update_paths,
                cached_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            curseforge_mod_cache.metadata_id,
            curseforge_mod_cache.murmur2,
            curseforge_mod_cache.project_id,
            curseforge_mod_cache.file_id,
            curseforge_mod_cache.name,
            curseforge_mod_cache.version,
            curseforge_mod_cache.urlslug,
            curseforge_mod_cache.summary,
            curseforge_mod_cache.authors,
            curseforge_mod_cache.release_type,
            curseforge_mod_cache.update_paths,
            curseforge_mod_cache.cached_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_curseforge_mod_cache(
        &self,
        metadata_id: &str,
    ) -> Result<CurseForgeModCache, sqlx::Error> {
        let curseforge_mod_cache = sqlx::query_as!(
            CurseForgeModCache,
            "SELECT * FROM curseforge_mod_cache WHERE metadata_id = ?",
            metadata_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(curseforge_mod_cache)
    }
}
