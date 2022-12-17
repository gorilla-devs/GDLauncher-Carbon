mod assets;
mod meta;
mod version;

#[cfg(test)]
mod test {
    use super::meta::McMeta;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_versions_meta() {
        // Test latest and download assets
        let meta = McMeta::download_meta().await.unwrap();

        // Test all versions meta
        let tasks: Vec<_> = meta
            .versions
            .into_iter()
            .map(|version| tokio::spawn(async move { version.get_version_meta().await.unwrap() }))
            .collect();

        for task in tasks {
            task.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_download_mc() {
        let meta = McMeta::download_meta().await.unwrap();

        let version = meta.latest.version_for_release(&meta);

        let version_meta = version.get_version_meta().await.unwrap();

        let asset_index = version_meta
            .retrieve_asset_index_meta()
            .await
            .expect("Failed to get asset index meta");

        asset_index
            .download_assets(std::env::current_dir().unwrap().join("assets"))
            .await
            .expect("Failed to download assets");

        version_meta
            .download_allowed_libraries()
            .await
            .expect("Failed to get libraries");

        tokio::fs::remove_dir_all(std::env::current_dir().unwrap().join("assets"))
            .await
            .expect("Failed to remove assets");
        tokio::fs::remove_dir_all(std::env::current_dir().unwrap().join("libraries"))
            .await
            .expect("Failed to remove libraries");

        // Should test
        // - Cancellation
        // - Do not download files if they are already correct
        // - Progress
        // - Natives
        // - Launch the game
    }
}
