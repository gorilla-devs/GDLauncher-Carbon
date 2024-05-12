use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct JavaProfile {
    pub name: String,
    pub is_system_profile: bool,
    pub java_id: Option<String>, // This corresponds to the optional Java relation
}

pub struct JavaProfileRepository {
    pool: SqlitePool,
}

impl JavaProfileRepository {
    pub fn new(pool: SqlitePool) -> Self {
        JavaProfileRepository { pool }
    }

    pub async fn add_java_profile(&self, java_profile: JavaProfile) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO java_profile (
                name,
                is_system_profile,
                java_id
            ) VALUES (?, ?, ?)",
            java_profile.name,
            java_profile.is_system_profile,
            java_profile.java_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_java_profile(&self, name: String) -> Result<JavaProfile, sqlx::Error> {
        let java_profile = sqlx::query_as!(
            JavaProfile,
            "SELECT * FROM java_profile WHERE name = ?",
            name
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(java_profile)
    }
}
