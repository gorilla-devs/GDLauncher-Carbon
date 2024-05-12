use sqlx::sqlite::SqlitePool;
use sqlx::FromRow;

#[derive(FromRow, Debug)]
pub struct InstanceGroup {
    pub id: i64,
    pub name: String,
    pub group_index: i64,
}

pub struct InstanceGroupRepository {
    pool: SqlitePool,
}

impl InstanceGroupRepository {
    pub fn new(pool: SqlitePool) -> Self {
        InstanceGroupRepository { pool }
    }

    pub async fn add_instance_group(
        &self,
        instance_group: InstanceGroup,
    ) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "INSERT INTO instance_group (
                name,
                group_index
            ) VALUES (?, ?)",
            instance_group.name,
            instance_group.group_index
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_instance_group(&self, id: i32) -> Result<InstanceGroup, sqlx::Error> {
        let instance_group = sqlx::query_as!(
            InstanceGroup,
            "SELECT * FROM instance_group WHERE id = ?",
            id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(instance_group)
    }
}
