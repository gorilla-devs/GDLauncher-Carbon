#[cfg(test)]
mod test {
    use std::env;
    use std::path::PathBuf;
    use env_logger::Builder;
    use log::{debug, LevelFilter, trace};
    
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
