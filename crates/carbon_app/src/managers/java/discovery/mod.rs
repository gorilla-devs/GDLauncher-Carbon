use std::path::PathBuf;

mod finder;

#[async_trait::async_trait]
pub trait Discovery {
    async fn find_java_paths(&self) -> Vec<PathBuf>;
}

pub struct RealDiscovery;

#[async_trait::async_trait]
impl Discovery for RealDiscovery {
    async fn find_java_paths(&self) -> Vec<PathBuf> {
        finder::find_java_paths().await
    }
}

pub struct MockDiscovery;

#[async_trait::async_trait]
impl Discovery for MockDiscovery {
    async fn find_java_paths(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/java1"),
            PathBuf::from("/java2"),
            PathBuf::from("/java3"),
        ]
    }
}
