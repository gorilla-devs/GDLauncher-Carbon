pub mod assets;
pub mod manifest;
pub mod minecraft;
pub mod modded;
pub mod version;

#[cfg(test)]
mod test {
    use crate::domain::minecraft::manifest::MinecraftManifest;

    // #[tokio::test]
    // async fn test_live_manifest_versions_format() {
    //     // Test latest and download assets
    //     let meta = MinecraftManifest::fetch().await.unwrap();

    //     // Test all versions meta
    //     let tasks: Vec<_> = meta
    //         .versions
    //         .into_iter()
    //         .map(|version| {
    //             tokio::spawn(async move {
    //                 version.fetch().await.unwrap();
    //             })
    //         })
    //         .collect();

    //     for task in tasks {
    //         task.await.unwrap();
    //     }
    // }
}
