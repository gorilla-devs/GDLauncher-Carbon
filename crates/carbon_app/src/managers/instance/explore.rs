use anyhow::{Context, anyhow};

use crate::{
    domain::instance::{ExploreEntry, ExploreEntryType, InstanceId},
    managers::ManagerRef,
};

use super::{InstanceManager, InvalidInstanceIdError};

impl<'s> ManagerRef<'s, InstanceManager> {
    /// Retrieve a single level of the file tree of an instance
    pub async fn explore_data(
        self,
        instance_id: InstanceId,
        path: Vec<String>,
    ) -> anyhow::Result<Vec<ExploreEntry>> {
        let instances = self.instances.read().await;
        let instance = instances
            .get(&instance_id)
            .ok_or(InvalidInstanceIdError(instance_id))?;

        let shortpath = instance.shortpath.clone();
        drop(instances);

        let instance_data_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_data_path();

        // Refuse anything that could climb out of the instance dir.
        for segment in &path {
            if segment.is_empty()
                || segment == "."
                || segment == ".."
                || segment.contains('/')
                || segment.contains('\\')
                || segment.contains('\0')
            {
                return Err(anyhow!(
                    "Invalid path segment in explore_data: {:?}",
                    segment
                ));
            }
        }

        let mut data_path = instance_data_path.clone();
        for segment in path {
            data_path.push(segment);
        }

        // Defensive symlink check: if both paths resolve cleanly, assert the
        // canonical target sits under the canonical instance dir. We only run
        // the check when BOTH canonicalize calls succeed — partial
        // canonicalization can mismatch on Windows (case differences) and
        // would falsely reject legitimate not-yet-existing subfolders, which
        // `read_dir` will surface as a normal "not found" error below.
        if let (Ok(canonical_data), Ok(canonical_target)) = tokio::join!(
            tokio::fs::canonicalize(&instance_data_path),
            tokio::fs::canonicalize(&data_path),
        ) {
            if !canonical_target.starts_with(&canonical_data) {
                return Err(anyhow!(
                    "Resolved path {:?} is not within instance dir {:?}",
                    canonical_target,
                    canonical_data
                ));
            }
        }

        let mut dir = tokio::fs::read_dir(&data_path)
            .await
            .context(format!("Reading instance data path: {data_path:?}"))?;

        let mut entries = Vec::<ExploreEntry>::new();
        while let Some(entry) = dir.next_entry().await? {
            let meta = entry.metadata().await?;
            entries.push(ExploreEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                type_: match meta.is_dir() {
                    true => ExploreEntryType::Directory,
                    false => ExploreEntryType::File {
                        size: meta.len() as u32,
                    },
                },
            })
        }

        Ok(entries)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashSet;

    use crate::{
        domain::instance::{ExploreEntry, ExploreEntryType, info},
        managers::instance::InstanceVersionSource,
    };

    #[tokio::test]
    async fn read_data() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let default_group_id = app.instance_manager().get_default_group().await?;
        let instance_id = app
            .instance_manager()
            .create_instance(
                default_group_id,
                String::from("test"),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: String::from("1.7.10"),
                        modloaders: HashSet::new(),
                    },
                )),
                String::new(),
            )
            .await?;

        let dir = app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path("test")
            .get_data_path();

        let folder = dir.join("folder");
        tokio::fs::create_dir(&folder).await?;
        tokio::fs::write(folder.join("file"), []).await?;
        tokio::fs::create_dir(folder.join("subfolder")).await?;

        let mut data = app
            .instance_manager()
            .explore_data(instance_id, vec![String::from("folder")])
            .await?;

        let mut expected = vec![
            ExploreEntry {
                name: String::from("file"),
                type_: ExploreEntryType::File { size: 0 },
            },
            ExploreEntry {
                name: String::from("subfolder"),
                type_: ExploreEntryType::Directory,
            },
        ];

        expected.sort_by_key(|e| e.name.clone());
        data.sort_by_key(|e| e.name.clone());

        assert_eq!(data, expected);

        Ok(())
    }
}
