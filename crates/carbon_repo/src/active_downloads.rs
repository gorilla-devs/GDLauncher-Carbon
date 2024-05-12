use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct ActiveDownloads {
    pub url: String,
    pub file_id: String,
}

pub struct ActiveDownloadsRepository {
    pool: SqlitePool,
}

impl ActiveDownloadsRepository {
    pub fn new(pool: SqlitePool) -> Self {
        ActiveDownloadsRepository { pool }
    }

    pub async fn add_active_download(&self, download: ActiveDownloads) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO active_downloads (
                url,
                file_id
            ) VALUES (?, ?)",
            download.url,
            download.file_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_active_download(&self, url: String) -> Result<ActiveDownloads, sqlx::Error> {
        let active_download = sqlx::query_as!(
            ActiveDownloads,
            "SELECT * FROM active_downloads WHERE url = ?",
            url
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(active_download)
    }

    pub async fn delete_active_download(&self, url: String) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM active_downloads WHERE url = ?", url)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
