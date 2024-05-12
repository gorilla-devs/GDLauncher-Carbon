use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct Instance {
    pub id: i64,
    pub name: String,
    pub shortpath: String,
    pub favorite: bool,
    pub has_pack_update: bool,
    pub index: i64,
    pub group_id: i64,
}

pub struct InstanceRepository {
    pool: SqlitePool,
}

impl InstanceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        InstanceRepository { pool }
    }

    pub async fn add_instance(&self, instance: Instance) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO instance (
                id,
                name,
                shortpath,
                favorite,
                has_pack_update,
                `index`,
                group_id
            ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            instance.id,
            instance.name,
            instance.shortpath,
            instance.favorite,
            instance.has_pack_update,
            instance.index,
            instance.group_id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_instance(&self, id: i64) -> Result<Instance, sqlx::Error> {
        let instance = sqlx::query_as!(Instance, "SELECT * FROM instance WHERE id = ?", id)
            .fetch_one(&self.pool)
            .await?;

        Ok(instance)
    }

    pub async fn update_instance(&self, instance: Instance) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "UPDATE instance SET
            name = ?,
            shortpath = ?,
            favorite = ?,
            has_pack_update = ?,
            `index` = ?,
            group_id = ?
            WHERE id = ?",
            instance.name,
            instance.shortpath,
            instance.favorite,
            instance.has_pack_update,
            instance.index,
            instance.group_id,
            instance.id
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_instance(&self, id: i64) -> Result<(), sqlx::Error> {
        sqlx::query!("DELETE FROM instance WHERE id = ?", id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
