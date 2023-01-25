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
