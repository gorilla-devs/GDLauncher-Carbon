use std::io::Cursor;
use std::path::PathBuf;
use std::str::FromStr;

use md5::Digest;
use md5::Md5;
use tokio::sync::mpsc;

use crate::api::translation::Translation;
use crate::db::read_filters::BytesFilter;
use crate::db::read_filters::StringFilter;
use crate::managers::vtask::VisualTask;
use crate::managers::ManagerRef;
use crate::once_send::OnceSend;

pub struct MetaCacheManager {
    cache_channel: mpsc::UnboundedSender<PathBuf>,
    background_channel: OnceSend<mpsc::UnboundedReceiver<PathBuf>>,
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
        let (cache_channel, cache_channel_rx) = mpsc::unbounded_channel();

        Self {
            cache_channel,
            background_channel: OnceSend::new(cache_channel_rx),
        }
    }
}

impl ManagerRef<'_, MetaCacheManager> {
    /// Register a path from the gdl root for mod metadata caching.
    pub fn cache_metadata(self, path: PathBuf) {
        // TODO: log errors
        let _ = self.cache_channel.send(path);
    }

    /// Panics if called more than once
    pub async fn launch_background_task(self) {
        let mut cache_channel = self
            .background_channel
            .take()
            .expect("launch_background_task may only be called once");

        let task = VisualTask::new(Translation::ModCacheTaskUpdate);
        let t_scan_files = task.subtask(Translation::ModCacheTaskUpdateScanFiles).await;
        let t_query_apis = task.subtask(Translation::ModCacheTaskUpdateQueryApis).await;

        let app_scantask = self.app.clone();
        let app_querytask = self.app.clone();

        tokio::spawn(async move {
            use crate::db::{mod_file_cache, mod_metadata};

            let app = app_scantask;
            let basepath = app.settings_manager().runtime_path.get_root().to_path();

            let mut path = basepath.clone();

            while let Some(subpath) = cache_channel.recv().await {
                // the path is reset and pushed instead of push/pop because
                // subpath can span multiple directories.
                path.clear();
                path.push(&basepath);
                path.push(&subpath);

                // TODO: log errors
                let Ok(meta) = tokio::fs::metadata(&path).await else { continue };
                // TODO: log
                if !meta.is_file() {
                    continue;
                }

                let dbstr = subpath.to_string_lossy().into_owned();

                // first check if the mod is already cached
                let cached = app
                    .prisma_client
                    .mod_file_cache()
                    .find_unique(mod_file_cache::UniqueWhereParam::PathEquals(dbstr.clone()))
                    .exec()
                    .await
                    // TODO: log errors
                    .ok()
                    .flatten();

                let md5: [u8; 16] = match cached {
                    Some(cached) if cached.filesize as u64 == meta.len() => {
                        // TODO: log / recache
                        match cached.md_5.try_into() {
                            Ok(md5) => md5,
                            Err(_) => continue,
                        }
                    }
                    _ => {
                        // TODO: log
                        let Ok(mut file) = tokio::fs::read(&path).await else { continue };

                        let r = tokio::task::spawn_blocking(move || {
                            let md5: [u8; 16] = Md5::new_with_prefix(&file).finalize().into();
                            let murmur2 = murmurhash32::murmurhash2(&file);
                            let meta = super::mods::parse_metadata(Cursor::new(&mut file))?;
                            Ok::<_, anyhow::Error>((md5, murmur2, meta))
                        });

                        // TODO: log
                        let Ok(Ok((md5, murmur2, Some(meta)))) = r.await else { continue };

                        let _ = app
                            .prisma_client
                            ._batch((
                                app.prisma_client.mod_file_cache().delete_many(vec![
                                    mod_file_cache::WhereParam::Path(StringFilter::Equals(
                                        dbstr.clone(),
                                    )),
                                ]),
                                app.prisma_client.mod_metadata().delete_many(vec![
                                    mod_metadata::WhereParam::Md5(BytesFilter::Equals(Vec::from(
                                        md5,
                                    ))),
                                ]),
                                app.prisma_client.mod_metadata().create(
                                    Vec::from(md5),
                                    murmur2 as i32,
                                    vec![
                                        mod_metadata::SetParam::SetModid(Some(meta.modid)),
                                        mod_metadata::SetParam::SetName(meta.name),
                                        mod_metadata::SetParam::SetVersion(meta.version),
                                        mod_metadata::SetParam::SetDescription(meta.description),
                                        mod_metadata::SetParam::SetAuthors(meta.authors),
                                    ],
                                ),
                            ))
                            .await;

                        md5
                    }
                };
            }
        });
    }
}
