use chrono::NaiveDateTime;
use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct HTTPCache {
    pub url: String,
    pub status_code: i64,
    pub data: Vec<u8>, // Using Vec<u8> to represent binary data (Bytes)
    pub expires_at: Option<NaiveDateTime>,
    pub last_modified: Option<String>,
    pub etag: Option<String>,
}

pub struct HTTPCacheRepository {
    pool: SqlitePool,
}

impl HTTPCacheRepository {
    pub fn new(pool: SqlitePool) -> Self {
        HTTPCacheRepository { pool }
    }

    pub async fn upsert_cache(&self, cache: HTTPCache) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query!("DELETE FROM http_cache WHERE url = ?", cache.url)
            .execute(&mut *tx)
            .await?;

        sqlx::query!(
            "INSERT INTO http_cache (url, status_code, data, expires_at, last_modified, etag) VALUES (?, ?, ?, ?, ?, ?)",
            cache.url,
            cache.status_code,
            cache.data,
            cache.expires_at,
            cache.last_modified,
            cache.etag
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn get_entry(&self, url: String) -> Result<HTTPCache, sqlx::Error> {
        let http_cache = sqlx::query_as!(
            HTTPCache,
            "SELECT
                url,
                status_code,
                data,
                expires_at,
                last_modified,
                etag
            FROM http_cache WHERE url = ?",
            url
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(http_cache)
    }
}
