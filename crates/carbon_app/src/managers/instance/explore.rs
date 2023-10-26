use std::path::PathBuf;

use anyhow::Context;

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

        let mut data_path = self
            .app
            .settings_manager()
            .runtime_path
            .get_instances()
            .get_instance_path(&shortpath)
            .get_data_path();

        data_path.extend(path);

        let mut dir = tokio::fs::read_dir(&data_path)
            .await
            .context(format!("Reading instance data path: {data_path:?}"))?;

        let mut entries = Vec::<ExploreEntry>::new();
        while let Some(entry) = dir.next_entry().await? {
            entries.push(ExploreEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                type_: match entry.metadata().await?.is_dir() {
                    true => ExploreEntryType::Directory,
                    false => ExploreEntryType::File,
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
        domain::instance::{info, ExploreEntry, ExploreEntryType},
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
                        modloaders: HashSet::from([info::ModLoader {
                            type_: info::ModLoaderType::Forge,
                            version: String::from("10.13.4.1614"),
                        }]),
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

        let data = app
            .instance_manager()
            .explore_data(instance_id, vec![String::from("folder")])
            .await?;

        let expected = vec![
            ExploreEntry {
                name: String::from("file"),
                type_: ExploreEntryType::File,
            },
            ExploreEntry {
                name: String::from("subfolder"),
                type_: ExploreEntryType::Directory,
            },
        ];

        assert_eq!(data, expected);

        Ok(())
    }
}
