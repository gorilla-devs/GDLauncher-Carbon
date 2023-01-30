#[cfg(test)]
#[tokio::test]
async fn test_vanilla_modloader() {
    use crate::modloader::{InstallProgress, ModLoaderHandler};
    let vanilla = super::VanillaModLoader::new("1.12.2".to_string(), std::sync::Weak::new());

    let (progress, mut progress_handle) = tokio::sync::watch::channel(InstallProgress::<
        crate::modloader::vanilla::InstallStages,
    > {
        count_progress: None,
        size_progress: None,
        stage: None,
    });

    let handle = tokio::spawn(async move {
        vanilla.install(progress).await?;
        Ok::<_, crate::modloader::vanilla::VanillaError>(())
    });

    while progress_handle.changed().await.is_ok() {
        println!("Progress: {:#?}", progress_handle.borrow());
    }

    handle.await.unwrap().unwrap();
}

#[tokio::test]
#[tracing_test::traced_test]
async fn test_versions_meta() {
    use super::meta::McMeta;

    // Test latest and download assets
    let meta = McMeta::download_manifest_meta().await.unwrap();
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
}
