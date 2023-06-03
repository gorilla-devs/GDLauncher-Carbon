use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::VecDeque;
use std::io::Cursor;
use std::path::PathBuf;
use std::str::FromStr;

use md5::Digest;
use md5::Md5;
use sentry::types::ParseDsnError;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use crate::api::translation::Translation;
use crate::db::read_filters::BytesFilter;
use crate::db::read_filters::IntFilter;
use crate::db::read_filters::StringFilter;
use crate::domain::instance::InstanceId;
use crate::domain::runtime_path::InstancesPath;
use crate::managers::vtask::VisualTask;
use crate::managers::ManagerRef;
use crate::once_send::OnceSend;

pub struct MetaCacheManager {
    waiting_instances: RwLock<HashSet<InstanceId>>,
    priority_instance: Mutex<Option<InstanceId>>,
    remote_request_queue: RwLock<VecDeque<([u8; 16], InstanceId)>>,
    waiting_notify: watch::Sender<()>,
    remote_nofity: watch::Sender<()>,
    // local cache notify, remote cache notify
    background_watches: OnceSend<(watch::Receiver<()>, watch::Receiver<()>)>,
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
struct ModId(pub [u8; 16]);

impl ToString for ModId {
    fn to_string(&self) -> String {
        hex::encode(self.0)
    }
}

impl FromStr for ModId {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut slice = [0u8; 16];
        hex::decode_to_slice(s, &mut slice)?;
        Ok(Self(slice))
    }
}

impl MetaCacheManager {
    pub fn new() -> Self {
        let (local_tx, local_rx) = watch::channel(());
        let (remote_tx, remote_rx) = watch::channel(());

        Self {
            waiting_instances: RwLock::new(HashSet::new()),
            priority_instance: Mutex::new(None),
            remote_request_queue: RwLock::new(VecDeque::new()),
            waiting_notify: local_tx,
            remote_nofity: remote_tx,
            background_watches: OnceSend::new((local_rx, remote_rx)),
        }
    }
}

impl ManagerRef<'_, MetaCacheManager> {
    /// Panics if called more than once
    pub async fn launch_background_tasks(self) {
        let (mut local_notify, mut cf_notify) = self
            .background_watches
            .take()
            .expect("launch_background_tasks may only be called once");

        let app_local = self.app.clone();
        let app_cf = self.app.clone();

        tokio::spawn(async move {
            use crate::db::{mod_file_cache as fcdb, mod_metadata as metadb};
            let app = app_local;
            let instance_manager = app.instance_manager();
            let basepath = app.settings_manager().runtime_path.get_root().to_path();
            let mut pathbuf = PathBuf::new();

            while local_notify.changed().await.is_ok() {
                loop {
                    let instance_id = match app
                        .meta_cache_manager()
                        .priority_instance
                        .lock()
                        .await
                        .take()
                    {
                        Some(priority) => Some(priority),
                        None => app
                            .meta_cache_manager()
                            .waiting_instances
                            .read()
                            .await
                            .iter()
                            .next()
                            .copied(),
                    };

                    let Some(instance_id) = instance_id else { break };

                    let instances = instance_manager.instances.read().await;
                    let Some(instance) = instances.get(&instance_id) else { continue };

                    let cache_app = app.clone();
                    let cached_entries = tokio::spawn(async move {
                        cache_app
                            .prisma_client
                            .mod_file_cache()
                            .find_many(vec![fcdb::WhereParam::InstanceId(IntFilter::Equals(
                                *instance_id as i32,
                            ))])
                            .exec()
                            .await
                    });

                    let subpath = InstancesPath::subpath()
                        .get_instance_path(&instance.shortpath)
                        .get_mods_path();

                    pathbuf.clear();
                    pathbuf.push(&basepath);
                    pathbuf.push(&subpath);

                    let mut modpaths = HashMap::<String, u64>::new();
                    let Ok(mut entries) = tokio::fs::read_dir(&pathbuf).await else { continue };

                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let file_name = entry.file_name();
                        let Some(utf8_name) = file_name.to_str() else { continue };

                        let is_jar = utf8_name.ends_with(".jar");
                        let is_jar_disabled = utf8_name.ends_with(".jar.disabled");

                        if !is_jar && !is_jar_disabled {
                            continue;
                        }
                        let Ok(metadata) = entry.metadata().await else { continue };
                        // file || symlink
                        if !metadata.is_dir() {
                            continue;
                        }

                        modpaths.insert(utf8_name.to_string(), metadata.len());
                    }

                    let mut dirty_cache = Vec::<fcdb::UniqueWhereParam>::new();

                    if let Ok(Ok(cached_entries)) = cached_entries.await {
                        for entry in cached_entries {
                            if let Some(real_size) = modpaths.get(&entry.path) {
                                if *real_size == entry.filesize as u64 {
                                    modpaths.remove(&entry.path);
                                    continue;
                                }
                            }

                            dirty_cache.push(fcdb::UniqueWhereParam::InstanceIdPathEquals(
                                *instance_id,
                                entry.path,
                            ));
                        }
                    }

                    let entry_futures = modpaths.into_iter().map(|(subpath, filesize)| {
                        let pathbuf = &pathbuf;
                        async move {
                            let content = tokio::fs::read(pathbuf.join(&subpath)).await?;
                            let (md5, murmur2, meta) = tokio::task::spawn_blocking(|| {
                                (
                                    <[u8; 16] as From<_>>::from(
                                        Md5::new_with_prefix(&content).finalize(),
                                    ),
                                    murmurhash32::murmurhash2(&content),
                                    super::mods::parse_metadata(Cursor::new(content)),
                                )
                            })
                            .await?;

                            let meta = meta?;

                            Ok::<_, anyhow::Error>(Some((subpath, filesize, md5, murmur2, meta)))
                        }
                    });

                    let (new_fc_entries, meta_entries) = futures::future::join_all(entry_futures)
                        .await
                        .into_iter()
                        .map(|m| m.unwrap_or(None))
                        .filter_map(|m| m)
                        .map(|(subpath, filesize, md5, murmur2, meta)| {
                            (
                                (
                                    *instance_id as i32,
                                    subpath,
                                    filesize as i32,
                                    Vec::from(md5),
                                    Vec::new(),
                                ),
                                (
                                    Vec::from(md5),
                                    murmur2 as i32,
                                    match meta {
                                        Some(meta) => vec![
                                            metadb::SetParam::SetName(meta.name),
                                            metadb::SetParam::SetModid(Some(meta.modid)),
                                            metadb::SetParam::SetVersion(meta.version),
                                            metadb::SetParam::SetDescription(meta.description),
                                            metadb::SetParam::SetAuthors(meta.authors),
                                        ],
                                        None => Vec::new(),
                                    },
                                ),
                            )
                        })
                        .unzip();

                    // TODO: FE background error endpoint
                    let _ = app
                        .prisma_client
                        ._batch((
                            dirty_cache
                                .into_iter()
                                .map(|id| app.prisma_client.mod_file_cache().delete(id))
                                .collect::<Vec<_>>(),
                            app.prisma_client
                                .mod_file_cache()
                                .create_many(new_fc_entries),
                            {
                                let mut q =
                                    app.prisma_client.mod_metadata().create_many(meta_entries);

                                q.skip_duplicates = true;
                                q
                            },
                        ))
                        .await;
                }
            }
        });

        /*
        tokio::spawn(async move {
            let app = app_cf;

            while cf_notify.changed().await.is_ok() {}
        });
        */
    }
}
