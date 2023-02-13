#[cfg(test)]
mod test {
    use crate::managers::instance::InstanceManager;
    use crate::managers::representation::CreateInstanceDto;
    use crate::managers::{instance, ManagersInner};
    use env_logger::Builder;
    use log::{debug, LevelFilter};
    use std::collections::BTreeMap;
    use std::env;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_instance_crud() {
        Builder::new().filter_level(LevelFilter::Trace).init();

        let test_assets_base_dir = env::temp_dir().join(format!(
            "gd_launcher_test_folder_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        ));
        let (invalidation_sender, _) = tokio::sync::broadcast::channel(200);
        let app = ManagersInner::new_with_invalidation_channel(invalidation_sender).await;
        let instance_manager = &app.instance_manager;

        let create_instance_dto = CreateInstanceDto {
            name: "test_written_instance".to_string(),
            minecraft_version: "1.15.179".to_string(),
            path_to_save_at: Some(test_assets_base_dir.clone()),
        };

        let added_written_instance = instance_manager
            .add_instance(create_instance_dto)
            .await
            .unwrap();

        let create_instance_dto = CreateInstanceDto {
            name: "test_in_memory_instance".to_string(),
            minecraft_version: "1.15.179".to_string(),
            path_to_save_at: None,
        };

        let added_in_memory_instance = instance_manager
            .add_instance(create_instance_dto)
            .await
            .unwrap();

        let in_memory_instance_id = added_in_memory_instance
            .uuid
            .parse()
            .expect("unable to parse id to u128");
        let written_instance_id = added_written_instance
            .uuid
            .parse()
            .expect("unable to parse id to u128");

        let read_written_instance = instance_manager
            .get_instance_by_id(written_instance_id)
            .await
            .unwrap();

        let read_in_memory_instance = instance_manager
            .get_instance_by_id(in_memory_instance_id)
            .await
            .unwrap();

        assert_eq!(added_written_instance, read_written_instance);
        assert_eq!(added_in_memory_instance, read_in_memory_instance);

        let notes = "i'm a in-memory instance".to_string();
        let mut new_props = BTreeMap::new();
        new_props.insert("notes".to_string(), notes.clone().into());

        let patched_in_memory_instance = instance_manager
            .patch_instance_by_id(in_memory_instance_id, new_props)
            .await
            .unwrap();

        let in_memory_instance = instance_manager
            .get_instance_by_id(in_memory_instance_id)
            .await
            .unwrap();
        assert_eq!(in_memory_instance, patched_in_memory_instance);
        assert_eq!(in_memory_instance.notes, notes.clone());

        let notes = "i'm a written instance".to_string();
        let mut new_props = BTreeMap::new();
        new_props.insert("notes".to_string(), notes.clone().into());

        let patched_written_instance = instance_manager
            .patch_instance_by_id(written_instance_id, new_props)
            .await
            .unwrap();

        let written_instance = instance_manager
            .get_instance_by_id(written_instance_id)
            .await
            .unwrap();
        assert_eq!(written_instance, patched_written_instance);
        assert_eq!(written_instance.notes, notes.clone());

        instance_manager
            .delete_instance_by_id(written_instance_id, true)
            .await
            .unwrap();

        instance_manager
            .get_instance_by_id(written_instance_id)
            .await
            .unwrap_err();

        instance_manager
            .delete_instance_by_id(in_memory_instance_id, true)
            .await
            .unwrap();

        instance_manager
            .get_instance_by_id(written_instance_id)
            .await
            .expect_err("expected to not find written_instance after delete!");

        instance_manager
            .get_instance_by_id(in_memory_instance_id)
            .await
            .expect_err("expected to not find in_memory_instance after delete!");
    }
}
