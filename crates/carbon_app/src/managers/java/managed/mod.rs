use self::azul_zulu::AzulZulu;
use super::java_checker::{JavaChecker, RealJavaChecker};
use crate::{
    api::keys::java::GET_SETUP_MANAGED_JAVA_PROGRESS,
    domain::java::{JavaArch, JavaOs, JavaVendor, JavaVersion},
};
use carbon_repos::db_exec::Db;
use carbon_rt_path::{ManagedJavasPath, TempPath};
use serde::Serialize;
use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
    sync::Arc,
};
use strum::IntoEnumIterator;
use tokio::sync::{
    Mutex,
    watch::{self, Sender},
};

pub mod azul_zulu;

#[derive(Debug, Default, Clone)]
pub enum Step {
    #[default]
    Idle,
    Downloading(u64, u64),
    Extracting(u64, u64),
    Done,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize)]
pub struct ManagedJavaVersion {
    pub id: String,
    pub name: String,
    pub download_url: String,
    pub java_version: JavaVersion,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManagedJavaArchMap(pub HashMap<JavaArch, Vec<ManagedJavaVersion>>);

impl Deref for ManagedJavaArchMap {
    type Target = HashMap<JavaArch, Vec<ManagedJavaVersion>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ManagedJavaArchMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ManagedJavaOsMap(pub HashMap<JavaOs, ManagedJavaArchMap>);

impl Deref for ManagedJavaOsMap {
    type Target = HashMap<JavaOs, ManagedJavaArchMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for ManagedJavaOsMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[async_trait::async_trait]
pub trait Managed {
    async fn setup<G: JavaChecker + Send + Sync>(
        &self,
        version: &ManagedJavaVersion,
        tmp_path: TempPath,
        base_managed_java_path: ManagedJavasPath,
        java_checker: &G,
        db: &Db,
        progress_report: Sender<Step>,
    ) -> anyhow::Result<String>;

    async fn fetch_all_versions(&self) -> anyhow::Result<ManagedJavaOsMap>;
}

#[derive(Debug, Default, Clone)]
pub struct ManagedService {
    azul_zulu: AzulZulu,
    pub setup_progress: Arc<Mutex<Step>>,
}

impl ManagedService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_all_os(&self) -> Vec<JavaOs> {
        JavaOs::iter().collect()
    }

    pub fn get_all_archs(&self) -> Vec<JavaArch> {
        JavaArch::iter().collect()
    }

    pub fn get_all_vendors(&self) -> Vec<JavaVendor> {
        JavaVendor::iter().collect()
    }

    pub async fn get_versions_for_vendor(
        &self,
        vendor: JavaVendor,
    ) -> anyhow::Result<ManagedJavaOsMap> {
        let versions = match vendor {
            JavaVendor::Azul => self.azul_zulu.fetch_all_versions().await?,
        };

        Ok(versions)
    }

    pub async fn setup_managed(
        &self,
        os: JavaOs,
        arch: JavaArch,
        vendor: JavaVendor,
        id: String,
        app: crate::App,
        progress_sender: Option<watch::Sender<Step>>,
    ) -> anyhow::Result<String> {
        let activity = app.runtime_lifecycle.admit_owned(
            "managed Java setup",
            crate::managers::runtime_lifecycle::ActivityPolicy::Installer,
        )?;
        let service = self.clone();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
            Box::pin(async move {
                service
                    .setup_managed_owned(
                        os,
                        arch,
                        vendor,
                        id,
                        app,
                        progress_sender,
                        slot.as_ref().unwrap().cancellation(),
                    )
                    .await
            })
        })
        .await?
    }

    async fn setup_managed_owned(
        &self,
        os: JavaOs,
        arch: JavaArch,
        vendor: JavaVendor,
        id: String,
        app: crate::App,
        progress_sender: Option<watch::Sender<Step>>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<String> {
        anyhow::ensure!(!cancel.is_cancelled(), "Managed Java setup cancelled");
        let invalidation_app = app.clone();
        complete_managed_setup(
            async {
                let java_id = match vendor {
                    JavaVendor::Azul => {
                        let versions = self.azul_zulu.fetch_all_versions().await?;
                        let version = versions
                            .get(&os)
                            .ok_or_else(|| anyhow::anyhow!("No versions for os: {:?}", os))?
                            .get(&arch)
                            .ok_or_else(|| anyhow::anyhow!("No versions for arch: {:?}", arch))?
                            .iter()
                            .find(|v| v.id == id)
                            .ok_or_else(|| anyhow::anyhow!("No version for id: {}", id))?;

                        let tmp_path = app.settings_manager().runtime_path.get_temp();
                        let base_managed_java_path =
                            app.settings_manager().runtime_path.get_managed_javas();
                        let db = app.db.clone();

                        let (sender, mut recv) = tokio::sync::watch::channel(Step::Idle);

                        let progress_ref = Arc::clone(&self.setup_progress);

                        let progress_app = app.clone();
                        let progress_task = tokio::spawn(async move {
                            let app = progress_app;

                            while recv.changed().await.is_ok() {
                                let mut progress_ref = progress_ref.lock().await;
                                *progress_ref = recv.borrow().clone();
                                if let Some(progress_sender) = &progress_sender {
                                    let _ = progress_sender.send(recv.borrow().clone());
                                }
                                app.invalidate(GET_SETUP_MANAGED_JAVA_PROGRESS, None);
                                drop(progress_ref);
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                        });

                        let setup_result = self
                            .azul_zulu
                            .setup(
                                version,
                                tmp_path,
                                base_managed_java_path,
                                &RealJavaChecker::with_cancel(
                                    app.runtime_lifecycle.clone(),
                                    cancel.clone(),
                                ),
                                &db,
                                sender,
                            )
                            .await;
                        progress_task.await?;
                        setup_result?
                    }
                };

                Ok(java_id)
            },
            || invalidation_app.invalidate(crate::api::keys::java::GET_AVAILABLE_JAVAS, None),
        )
        .await
    }
}

async fn complete_managed_setup<F: std::future::Future<Output = anyhow::Result<String>>>(
    setup: F,
    invalidate: impl FnOnce(),
) -> anyhow::Result<String> {
    let java_id = setup.await?;
    invalidate();
    Ok(java_id)
}

#[cfg(test)]
mod task18_tests {
    use super::*;
    #[tokio::test]
    async fn task18_managed_invalidation_follows_persistence_and_not_failure() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("managed.db");
        let mut conn = rusqlite::Connection::open(&path).unwrap();
        carbon_repos::get_migrations()
            .0
            .to_latest(&mut conn)
            .unwrap();
        drop(conn);
        let db = Db::open(&path, 2, true).unwrap();
        let notified = std::sync::atomic::AtomicUsize::new(0);
        let result = complete_managed_setup(async {
            assert_eq!(notified.load(std::sync::atomic::Ordering::SeqCst), 0);
            crate::managers::java::scan_and_sync::upsert_java_component_to_db(&db, crate::domain::java::JavaComponent {
                path: "validated-fixture-runtime".into(), version: JavaVersion::from_major(21),
                _type: crate::domain::java::JavaComponentType::Managed, os: JavaOs::Linux, arch: JavaArch::X86_64, vendor: "fixture".into(),
            }).await
        }, || {
            let connection = rusqlite::Connection::open(&path).unwrap();
            let count: i64 = connection.query_row("SELECT count(*) FROM Java WHERE path='validated-fixture-runtime' AND isValid=1", [], |row| row.get(0)).unwrap();
            assert_eq!(count, 1, "query invalidation must follow visible persisted Java");
            notified.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }).await.unwrap();
        assert!(
            carbon_repos::repos::java::get_java_by_id(&db, &result)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(notified.load(std::sync::atomic::Ordering::SeqCst), 1);
        assert!(
            complete_managed_setup(
                async { anyhow::bail!("controlled probe/persistence failure") },
                || {
                    panic!("failure must not publish success");
                }
            )
            .await
            .is_err()
        );
    }
}
