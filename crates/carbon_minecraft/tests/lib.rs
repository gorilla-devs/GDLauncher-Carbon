#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;
    use env_logger::Builder;
    use log::{debug, LevelFilter, trace};

    use carbon_minecraft::{db, instance, try_path_fmt};
    use carbon_minecraft::db::app_configuration::SetParam::SetId;
    use carbon_minecraft::db::app_configuration::WhereParam;
    use carbon_minecraft::db::read_filters::IntFilter;
    use carbon_minecraft::instance::{Instance, InstanceStatus};
    use carbon_minecraft::instance::delete::delete;
    use carbon_minecraft::instance::scan::check_instance_directory_sanity;
    use carbon_minecraft::instance::write::write_at;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_instance_crud() {
        Builder::new().filter_level(LevelFilter::Trace).init();

        let test_assets_base_dir = std::env::current_dir().unwrap()
            .join("test_assets");

        let directory_to_scan = &PathBuf::from(&test_assets_base_dir);
        debug!("scanning directory at {}", try_path_fmt!(directory_to_scan));

        let instance_scan_results = instance::scan::scan_for_instances(directory_to_scan).await.unwrap();

        debug!("Instance scan result {:?}", instance_scan_results);

        let found_instances = instance_scan_results.into_iter().filter(|r|r.is_ok()).collect::<Vec<_>>();

        assert_eq!(1, found_instances.len(), "found 0 good instance in {} we expected 1 !", try_path_fmt!(directory_to_scan));

        let found_instance = Result::unwrap(found_instances.first().unwrap().as_ref());

        debug!("Instance found : \n {}", serde_json::to_string_pretty(&found_instance).unwrap());

        let new_instance = Instance::default();

        assert_eq!(new_instance.persistence_status, InstanceStatus::NotPersisted, "newly constructed instance expected to be not persisted but is persisted !");

        debug!("Instance found : \n {}", serde_json::to_string_pretty(&found_instance).unwrap());

        let tmp_directory = env::temp_dir().join("GDLauncher_test");
        let _ = std::fs::remove_dir_all(&tmp_directory);
        let _ = std::fs::create_dir(&tmp_directory);

        debug!("used tmp for the test : {}", try_path_fmt!(tmp_directory));

        let new_instance = write_at(new_instance, &tmp_directory).await.unwrap();

        assert!(check_instance_directory_sanity(&tmp_directory).await.is_ok());

        debug!("instance correctly wrote at : {}", try_path_fmt!(tmp_directory));

        debug!("trying to delete instance at : {}", try_path_fmt!(tmp_directory));

        let new_instance = delete(new_instance, false).await.unwrap();

        assert!(check_instance_directory_sanity(&tmp_directory).await.is_err());

        assert_eq!(new_instance.persistence_status, InstanceStatus::NotPersisted, "deleted instance expected to be not persisted but remain persisted !" );

        debug!("instance  correctly deleted at : {}", try_path_fmt!(tmp_directory));

    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn persistence_ok() {

        trace!("trying to connect to db ");
        let client = db::new_client().await
            .expect("unable to build app_configuration client using db_url ");
        trace!("connected to db");

        let configuration = client
            .app_configuration()
            .create(vec![SetId(0)])
            .exec()
            .await
            .expect("unable to exec create query for app_configuration");

        trace!("wrote correctly in db : {:#?}",configuration);

        let _serialized_configuration = serde_json::to_string_pretty(&configuration)
            .expect("unable to serialize app_configuration");

        let _count = client.app_configuration()
            .count(vec![WhereParam::Id(IntFilter::Equals(0))])
            .exec().await
            .expect("unable to select app_configuration");

        trace!("read correctly from db ");
    }

    /* #[tokio::test]
     #[tracing_test::traced_test]
     async fn test_versions_meta() {
         // Test latest and download assets
         let meta = McMeta::download_meta().await.unwrap();
         let base_dir = std::env::current_dir().unwrap().join("MC_TEST");
         // Test all versions meta
         let tasks: Vec<_> = meta
             .versions
             .into_iter()
             .map(|version| {
                 let base_dir = base_dir.clone();
                 tokio::spawn(async move { version.get_version_meta(&base_dir).await.unwrap() })
             })
             .collect();

         for task in tasks {
             task.await.unwrap();
         }
     }*/

    #[tokio::test]
    async fn test_download_mc() {

        // let meta = McMeta::download_meta().await.unwrap();

        // let base_dir = std::env::current_dir().unwrap().join("MC_TEST");

        // let version_meta = meta
        //     .versions
        //     .iter()
        //     .find(|version| version.id == "1.12.2")
        //     .unwrap()
        //     .get_version_meta(&base_dir)
        //     .await
        //     .unwrap();

        // let mut downloads = vec![];

        // let asset_index = version_meta
        //     .get_asset_index_meta(&base_dir)
        //     .await
        //     .expect("Failed to get asset index meta");

        // let assets = asset_index
        //     .get_asset_downloads(&base_dir)
        //     .await
        //     .expect("Failed to download assets");
        // downloads.extend(assets);

        // let libs = version_meta
        //     .get_allowed_libraries(&base_dir)
        //     .await
        //     .expect("Failed to get libraries");
        // downloads.extend(libs);

        // let client = version_meta
        //     .get_jar_client(&base_dir)
        //     .await
        //     .expect("Failed to get client download");
        // downloads.push(client);

        // println!("Downloading {downloads:#?}");

        // let total_size = downloads
        //     .iter()
        //     .map(|download| download.size.unwrap_or(0))
        //     .sum::<u64>()
        //     / 1024
        //     / 1024;

        // let (progress, mut progress_handle) = tokio::sync::watch::channel(crate::net::Progress {
        //     current_count: 0,
        //     current_size: 0,
        // });

        // let length = &downloads.len();
        // let handle = tokio::spawn(async move {
        //     crate::net::download_multiple(downloads, progress).await?;
        //     Ok::<(), anyhow::Error>(())
        // });

        // while progress_handle.changed().await.is_ok() {
        //     println!(
        //         "Progress: {} / {} - {} / {} MB",
        //         progress_handle.borrow().current_count,
        //         length - 1,
        //         progress_handle.borrow().current_size,
        //         total_size
        //     );
        // }

        // handle.await.unwrap().unwrap();

        // version_meta.extract_natives(&base_dir).await.unwrap();

        // tokio::fs::remove_dir_all(base_dir)
        //     .await
        //     .expect("Failed to remove");

        // Should test
        // - Cancellation
        // - Do not download files if they are already correct
        // - Progress
        // - Natives
        // - Launch the game
    }
}
