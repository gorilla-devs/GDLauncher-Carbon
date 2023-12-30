use std::path::PathBuf;

use crate::domain::runtime_path::RuntimePath;

mod finder;

#[async_trait::async_trait]
pub trait Discovery {
    fn new(runtime_path: RuntimePath) -> Self
    where
        Self: Sized;
    async fn find_java_paths(&self) -> Vec<PathBuf>;
    async fn find_managed_java_paths(&self) -> Vec<PathBuf>;
}

pub struct RealDiscovery {
    runtime_path: RuntimePath,
}

#[async_trait::async_trait]
impl Discovery for RealDiscovery {
    fn new(runtime_path: RuntimePath) -> Self {
        Self { runtime_path }
    }

    async fn find_java_paths(&self) -> Vec<PathBuf> {
        finder::find_java_paths().await
    }

    async fn find_managed_java_paths(&self) -> Vec<PathBuf> {
        finder::scan_managed_java_paths(self.runtime_path.get_managed_javas().to_path()).await
    }
}

pub struct MockDiscovery;

#[async_trait::async_trait]
impl Discovery for MockDiscovery {
    fn new(_runtime_path: RuntimePath) -> Self {
        Self
    }

    async fn find_java_paths(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/java1"),
            PathBuf::from("/java2"),
            PathBuf::from("/java3"),
        ]
    }

    async fn find_managed_java_paths(&self) -> Vec<PathBuf> {
        vec![
            PathBuf::from("/managed/java1"),
            PathBuf::from("/managed/java2"),
            PathBuf::from("/managed/java3"),
        ]
    }
}
