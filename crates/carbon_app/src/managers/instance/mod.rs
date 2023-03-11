use std::{collections::HashMap, io, ops::Deref, path::PathBuf};

use anyhow::anyhow;
use anyhow::bail;
use carbon_domain::instance::InstanceConfiguration;
use futures::future::BoxFuture;
use rspc::Type;
use serde::Serialize;
use serde_json::error::Category as JsonErrorType;
use tokio::sync::{Mutex, MutexGuard, RwLock};

use crate::db::{self, read_filters::IntFilter};
use db::instance::Data as CachedInstance;

use super::ManagerRef;

pub struct InstanceManager {
    instances: RwLock<HashMap<String, InstanceType>>,
    index_lock: Mutex<()>,
}

impl InstanceManager {
    pub fn new() -> Self {
        Self {
            instances: RwLock::new(HashMap::new()),
            index_lock: Mutex::new(()),
        }
    }
}

impl<'s> ManagerRef<'s, InstanceManager> {
    pub async fn scan_instances(self) -> anyhow::Result<()> {
        let instance_cache = self
            .app
            .prisma_client
            .instance()
            .find_many(vec![])
            .exec()
            .await?;

        let instance_path = self
            .app
            .configuration_manager()
            .runtime_path
            .get_instances()
            .to_path();

        let mut stream = tokio::fs::read_dir(instance_path).await?;

        while let Some(dir) = stream.next_entry().await? {
            let path = dir.path();

            let shortpath = path
                .file_name()
                .expect("path given to scan_instance should never have a null filename")
                .to_str()
                .expect("current GDL versions only support UTF8 paths")
                .to_string();

            let cached = instance_cache
                .iter()
                .find(|instance| instance.shortpath == shortpath);

            let Some(_instance) = self.scan_instance(shortpath, path, cached).await? else { continue };

            // todo: cache
        }

        Ok(())
    }

    /// Scan the given path as an instance folder.
    ///
    /// If cached is Some an Instance will always be returned, though it may be missing files.
    /// If cached is None an Instance will only be returned if a config file is present.
    async fn scan_instance(
        self,
        shortpath: String,
        path: PathBuf,
        cached: Option<&CachedInstance>,
    ) -> anyhow::Result<Option<Instance>> {
        use db::instance::{SetParam, UniqueWhereParam};

        let config_path = path.join("config.json");

        let config_text = match tokio::fs::read_to_string(config_path).await {
            Ok(x) => x,
            Err(e) => {
                // if we aren't already tracking this instance just ignore it.
                if let Some(cached) = cached {
                    let invalid_type = match e.kind() {
                        io::ErrorKind::NotFound => InvalidConfiguration::NoFile,
                        _ => InvalidConfiguration::IoError(e.to_string()),
                    };

                    return Ok(Some(Instance {
                        name: cached.name.clone(),
                        shortpath: shortpath.clone(),
                        group: GroupId(cached.group_id),
                        type_: InstanceType::Invalid(invalid_type),
                    }));
                } else {
                    return Ok(None);
                }
            }
        };

        match serde_json::from_str::<InstanceConfiguration>(&config_text) {
            Ok(config) => {
                let group = if let Some(cached) = cached {
                    self.app
                        .prisma_client
                        .instance()
                        .update(
                            UniqueWhereParam::ShortpathEquals(shortpath.clone()),
                            vec![SetParam::SetName(config.instance_name.clone())],
                        )
                        .exec()
                        .await?;

                    GroupId(cached.group_id)
                } else {
                    let group = self.get_default_group().await?;
                    self.add_instance(shortpath.clone(), shortpath.clone(), group)
                        .await?;
                    group
                };

                Ok(Some(Instance {
                    name: config.instance_name.clone(),
                    shortpath: shortpath.clone(),
                    group,
                    type_: InstanceType::Valid(config),
                }))
            }
            Err(e) => {
                let error = InvalidConfiguration::Invalid(ConfigurationParseError {
                    type_: match e.classify() {
                        JsonErrorType::Data => ConfigurationParseErrorType::Data,
                        JsonErrorType::Syntax => ConfigurationParseErrorType::Syntax,
                        JsonErrorType::Eof => ConfigurationParseErrorType::Eof,
                        JsonErrorType::Io => unreachable!(),
                    },
                    line: e.line() as u32, // will panic with more lines but that dosen't really seem like a problem
                    message: e.to_string(),
                    config_text,
                });

                Ok(Some(Instance {
                    name: shortpath.to_string(),
                    shortpath,
                    group: self.get_default_group().await?,
                    type_: InstanceType::Invalid(error),
                }))
            }
        }
    }

    /// Move the given group to the index directly before `before`.
    /// If `before` is None, move to the end of the list.
    async fn move_group(self, group: GroupId, before: Option<GroupId>) -> anyhow::Result<()> {
        use db::instance_group::{SetParam, UniqueWhereParam, WhereParam};

        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        let start_idx = self
            .app
            .prisma_client
            .instance_group()
            .find_unique(UniqueWhereParam::IdEquals(*group))
            .exec()
            .await?
            .ok_or_else(|| anyhow!("GroupId is not in database, this should never happen"))?
            .group_index;

        let target_idx = match before {
            Some(target) => {
                self.app
                    .prisma_client
                    .instance_group()
                    .find_unique(UniqueWhereParam::IdEquals(*target))
                    .exec()
                    .await?
                    .ok_or_else(|| anyhow!("GroupId is not in database, this should never happen"))?
                    .group_index
            }
            None => {
                self.app
                    .prisma_client
                    .instance_group()
                    .count(vec![])
                    .exec()
                    .await? as i32
            }
        };

        let (reamining_query, target_idx) = match (start_idx, target_idx) {
            (start, target) if start < target => (
                self.app.prisma_client.instance_group().update_many(
                    vec![
                        WhereParam::GroupIndex(IntFilter::Gt(start)),
                        WhereParam::GroupIndex(IntFilter::Lt(target)),
                    ],
                    vec![SetParam::DecrementGroupIndex(1)],
                ),
                target - 1,
            ),
            (start, target) if start > target => (
                self.app.prisma_client.instance_group().update_many(
                    vec![
                        WhereParam::GroupIndex(IntFilter::Gte(target)),
                        WhereParam::GroupIndex(IntFilter::Lt(start)),
                    ],
                    vec![SetParam::IncrementGroupIndex(1)],
                ),
                target,
            ),
            _ => return Ok(()),
        };

        self.app
            .prisma_client
            ._batch((
                reamining_query,
                self.app.prisma_client.instance_group().update(
                    UniqueWhereParam::IdEquals(*group),
                    vec![SetParam::SetGroupIndex(target_idx)],
                ),
            ))
            .await?;

        Ok(())
    }

    /// Move the given instance to the index directly before `target` in the target instance group.
    /// If `target` is None, move to the end of the instance group.
    async fn move_instance(
        self,
        instance: InstanceId,
        target: InstanceMoveTarget,
    ) -> anyhow::Result<()> {
        use db::instance::{SetParam, UniqueWhereParam, WhereParam};

        // lock indexes while we're changing them
        let _index_lock = self.index_lock.lock().await;

        let (start_group, start_idx) = {
            let instance = self
                .app
                .prisma_client
                .instance()
                .find_unique(UniqueWhereParam::IdEquals(*instance))
                .exec()
                .await?
                .ok_or_else(|| {
                    anyhow!("InstanceId is not in database, this should never happen")
                })?;

            (GroupId(instance.group_id), instance.index)
        };

        let (target_group, target_idx) = match target {
            InstanceMoveTarget::Before(target) => {
                let instance = self
                    .app
                    .prisma_client
                    .instance()
                    .find_unique(UniqueWhereParam::IdEquals(*target))
                    .exec()
                    .await?
                    .ok_or_else(|| {
                        anyhow!("InstanceId is not in database, this should never happen")
                    })?;

                (GroupId(instance.group_id), instance.index)
            }
            InstanceMoveTarget::EndOfGroup(group) => {
                let target_idx = self
                    .app
                    .prisma_client
                    .instance()
                    .count(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
                    .exec()
                    .await? as i32;

                (group, target_idx)
            }
        };

        let index_shifts = if start_group == target_group {
            vec![match (start_idx, target_idx) {
                (start, target) if start < target => self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gt(start)),
                        WhereParam::Index(IntFilter::Lte(target)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                (start, target) if start > target => self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gte(target)),
                        WhereParam::Index(IntFilter::Lt(start)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
                _ => return Ok(()),
            }]
        } else {
            vec![
                self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*start_group)),
                        WhereParam::Index(IntFilter::Gt(start_idx)),
                    ],
                    vec![SetParam::DecrementIndex(1)],
                ),
                self.app.prisma_client.instance().update_many(
                    vec![
                        WhereParam::GroupId(IntFilter::Equals(*target_group)),
                        WhereParam::Index(IntFilter::Gte(target_idx)),
                    ],
                    vec![SetParam::IncrementIndex(1)],
                ),
            ]
        };

        self.app
            .prisma_client
            ._batch((
                index_shifts,
                self.app.prisma_client.instance().update(
                    UniqueWhereParam::IdEquals(*instance),
                    vec![
                        SetParam::SetGroupId(*target_group),
                        SetParam::SetIndex(target_idx),
                    ],
                ),
            ))
            .await?;

        Ok(())
    }

    fn get_default_group(self) -> BoxFuture<'s, anyhow::Result<GroupId>> {
        Box::pin(async move {
            use db::instance_group::WhereParam;

            static DEFAULT_MUTEX: Mutex<()> = Mutex::const_new(());

            let groupid = self
                .app
                .configuration_manager()
                .configuration()
                .get()
                .await?
                .default_instance_group;

            match groupid {
                Some(groupid) => {
                    let group = self
                        .app
                        .prisma_client
                        .instance_group()
                        .find_first(vec![WhereParam::Id(IntFilter::Equals(groupid))])
                        .exec()
                        .await?;

                    match group {
                        Some(x) => Ok(GroupId(x.id)),
                        None => bail!("invalid database state: default group specified in configuration, but missing from groups"),
                    }
                }
                None => {
                    match DEFAULT_MUTEX.try_lock() {
                        Ok(_lock) => {
                            let index = self.next_group_index().await?;

                            self.app
                                .prisma_client
                                ._transaction()
                                .run(|prisma| async move {
                                    let group = prisma
                                        .instance_group()
                                        .create(
                                            String::from("localize➽default"),
                                            index.value,
                                            vec![],
                                        )
                                        .exec()
                                        .await?;

                                    use db::app_configuration::{SetParam, UniqueWhereParam};

                                    prisma
                                        .app_configuration()
                                        .update(
                                            UniqueWhereParam::IdEquals(0),
                                            vec![SetParam::SetDefaultInstanceGroup(Some(group.id))],
                                        )
                                        .exec()
                                        .await?;

                                    Ok(GroupId(group.id))
                                })
                                .await
                        }
                        Err(_) => {
                            // Wait for the lock to finish, some other thread probably
                            // wrote the group to the DB at this point, so just retry getting it from the db.
                            let _ = DEFAULT_MUTEX.lock().await;
                            self.get_default_group().await
                        }
                    }
                }
            }
        })
    }

    async fn create_group(self, name: String) -> anyhow::Result<GroupId> {
        let index = self.next_group_index().await?;

        let group = self
            .app
            .prisma_client
            .instance_group()
            .create(name, index.value, vec![])
            .exec()
            .await?;

        Ok(GroupId(group.id))
    }

    /// Add an instance to the database without checking if it exists.
    async fn add_instance(
        self,
        name: String,
        shortpath: String,
        group: GroupId,
    ) -> anyhow::Result<InstanceId> {
        use db::instance_group::UniqueWhereParam;
        let index = self.next_instance_index(group).await?;

        let instance = self
            .app
            .prisma_client
            .instance()
            .create(
                name,
                shortpath,
                index.value,
                UniqueWhereParam::IdEquals(*group),
                vec![],
            )
            .exec()
            .await?;

        Ok(InstanceId(instance.id))
    }

    /// Delete an instance group and move all contained instances into the default group.
    // TODO: handle deleting the default group while it has instances.
    async fn delete_group(self, group: GroupId) -> anyhow::Result<()> {
        use db::{instance, instance_group};

        // lock indexes before checking for instances to make sure none can be moved or created.
        let _index_lock = self.index_lock.lock().await;

        let any_instances = self
            .app
            .prisma_client
            .instance()
            .count(vec![instance::WhereParam::GroupId(IntFilter::Equals(
                *group,
            ))])
            .exec()
            .await?
            != 0;

        // a default group will be created if get_default_group is called, so
        // we check if any instances exist before creating it to avoid making an
        // empty group every time a group is deleted.
        if any_instances {
            let default_group = self.get_default_group().await?;

            // next_instance_index can't be used due to _index_lock, and dropping it
            // first would be a race condition.
            let base_index = self
                .app
                .prisma_client
                .instance()
                .count(vec![instance::WhereParam::GroupId(IntFilter::Equals(
                    *group,
                ))])
                .exec()
                .await?;

            self.app
                .prisma_client
                ._batch((
                    self.app.prisma_client.instance().update_many(
                        vec![instance::WhereParam::GroupId(IntFilter::Equals(*group))],
                        vec![
                            instance::SetParam::SetGroupId(*default_group),
                            instance::SetParam::IncrementIndex(base_index as i32),
                        ],
                    ),
                    self.app
                        .prisma_client
                        .instance_group()
                        .delete(instance_group::UniqueWhereParam::IdEquals(*group)),
                ))
                .await?;
        } else {
            self.app
                .prisma_client
                .instance_group()
                .delete(instance_group::UniqueWhereParam::IdEquals(*group))
                .exec()
                .await?;
        }

        Ok(())
    }

    async fn next_group_index(self) -> anyhow::Result<IdLock<'s, i32>> {
        let guard = self.manager.index_lock.lock().await;

        let count = self
            .app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        Ok(IdLock {
            value: count as i32,
            guard,
        })
    }

    async fn next_instance_index(self, group: GroupId) -> anyhow::Result<IdLock<'s, i32>> {
        use db::instance::WhereParam;

        let guard = self.manager.index_lock.lock().await;

        let count = self
            .app
            .prisma_client
            .instance()
            .count(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
            .exec()
            .await?;

        Ok(IdLock {
            value: count as i32,
            guard,
        })
    }
}

struct Group {
    id: GroupId,
    name: String,
    // index intentionally omitted
}

/// Lock used to prevent race conditions when modifying group or instance indexes
struct IdLock<'a, V: Copy + Clone> {
    value: V,
    guard: MutexGuard<'a, ()>,
}

// Typed group id to avoid dealing with a raw integer ids.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct GroupId(i32);

// Typed instance id to avoid dealing with a raw integer ids.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct InstanceId(i32);

impl Deref for GroupId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Deref for InstanceId {
    type Target = i32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

enum InstanceMoveTarget {
    Before(InstanceId),
    EndOfGroup(GroupId),
}

struct Instance {
    name: String,
    shortpath: String,
    group: GroupId,
    // todo: icon
    type_: InstanceType,
}

enum InstanceType {
    Valid(InstanceConfiguration),
    Invalid(InvalidConfiguration),
}

#[derive(Type, Serialize)]
enum InvalidConfiguration {
    NoFile,
    Invalid(ConfigurationParseError),
    IoError(String),
}

#[derive(Type, Serialize)]
struct ConfigurationParseError {
    type_: ConfigurationParseErrorType,
    message: String,
    line: u32,
    config_text: String,
}

#[derive(Type, Serialize)]
enum ConfigurationParseErrorType {
    Syntax,
    Data,
    Eof,
}

#[cfg(test)]
mod test {
    use prisma_client_rust::Direction;

    use crate::{
        db::{self, read_filters::IntFilter, PrismaClient},
        managers::instance::{GroupId, InstanceId, InstanceMoveTarget},
    };

    #[tokio::test]
    async fn move_groups() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_groups(prisma_client: &PrismaClient) -> anyhow::Result<Vec<GroupId>> {
            use crate::db::instance_group::OrderByParam;

            Ok(prisma_client
                .instance_group()
                .find_many(vec![])
                .order_by(OrderByParam::GroupIndex(Direction::Asc))
                .exec()
                .await?
                .into_iter()
                .map(|group| GroupId(group.id))
                .collect())
        }

        let mut groups = [
            app.instance_manager()
                .create_group(String::from("move0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move1"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move2"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move3"))
                .await?,
            app.instance_manager()
                .create_group(String::from("move4"))
                .await?,
        ];

        // move 1 to 3 as if dragged
        app.instance_manager()
            .move_group(groups[1], Some(groups[3]))
            .await?;
        groups = [groups[0], groups[2], groups[1], groups[3], groups[4]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 3 back to 1
        app.instance_manager()
            .move_group(groups[3], Some(groups[1]))
            .await?;
        groups = [groups[0], groups[3], groups[1], groups[2], groups[4]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 1 to end of list
        app.instance_manager().move_group(groups[1], None).await?;
        groups = [groups[0], groups[2], groups[3], groups[4], groups[1]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        // move 4 to beginning of list
        app.instance_manager()
            .move_group(groups[4], Some(groups[0]))
            .await?;
        groups = [groups[4], groups[0], groups[1], groups[2], groups[3]];
        assert_eq!(
            groups[..],
            get_ordered_groups(&app.prisma_client).await?[..]
        );

        Ok(())
    }

    #[tokio::test]
    async fn move_instances() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        async fn get_ordered_instances(
            prisma_client: &PrismaClient,
            group: GroupId,
        ) -> anyhow::Result<Vec<InstanceId>> {
            use crate::db::instance::{OrderByParam, WhereParam};

            Ok(prisma_client
                .instance()
                .find_many(vec![WhereParam::GroupId(IntFilter::Equals(*group))])
                .order_by(OrderByParam::Index(Direction::Asc))
                .exec()
                .await?
                .into_iter()
                .map(|instance| InstanceId(instance.id))
                .collect())
        }

        let [group0, group1] = [
            app.instance_manager()
                .create_group(String::from("group0"))
                .await?,
            app.instance_manager()
                .create_group(String::from("group1"))
                .await?,
        ];

        let mk_instance = |shortpath: &'static str, group| {
            let app = &app;
            async move {
                let id = app
                    .instance_manager()
                    .add_instance(shortpath.to_string(), shortpath.to_string(), group)
                    .await?;

                Ok::<_, anyhow::Error>(id)
            }
        };

        let mut group0_instances = [
            mk_instance("g0i0", group0.clone()).await?,
            mk_instance("g0i1", group0.clone()).await?,
            mk_instance("g0i2", group0.clone()).await?,
        ];

        let group1_instances = [
            mk_instance("g1i0", group1.clone()).await?,
            mk_instance("g1i1", group1.clone()).await?,
        ];

        // move 1 to end of list
        app.instance_manager()
            .move_instance(group0_instances[1], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[0],
            group0_instances[2],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 0 to end of list
        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group0))
            .await?;

        group0_instances = [
            group0_instances[1],
            group0_instances[2],
            group0_instances[0],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 2 back to 0
        app.instance_manager()
            .move_instance(
                group0_instances[2],
                InstanceMoveTarget::Before(group0_instances[0]),
            )
            .await?;

        group0_instances = [
            group0_instances[2],
            group0_instances[0],
            group0_instances[1],
        ];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        // move 0:1 to 1:1
        app.instance_manager()
            .move_instance(
                group0_instances[1],
                InstanceMoveTarget::Before(group1_instances[1]),
            )
            .await?;

        let group1_instances = [
            group1_instances[0],
            group0_instances[1],
            group1_instances[1],
        ];

        let group0_instances = [group0_instances[0], group0_instances[2]];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.prisma_client, group1).await?[..],
        );

        // move 0:0 to end of group 1
        app.instance_manager()
            .move_instance(group0_instances[0], InstanceMoveTarget::EndOfGroup(group1))
            .await?;

        let group1_instances = [
            group1_instances[0],
            group1_instances[1],
            group1_instances[2],
            group0_instances[0],
        ];

        let group0_instances = [group0_instances[1]];

        assert_eq!(
            group0_instances[..],
            get_ordered_instances(&app.prisma_client, group0).await?[..],
        );

        assert_eq!(
            group1_instances[..],
            get_ordered_instances(&app.prisma_client, group1).await?[..],
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group() -> anyhow::Result<()> {
        use db::instance::UniqueWhereParam::ShortpathEquals;

        let app = crate::setup_managers_for_test().await;

        let default_group = app.instance_manager().get_default_group().await?;
        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;
        app.instance_manager()
            .add_instance(String::from("baz"), String::from("baz"), default_group)
            .await?;
        app.instance_manager()
            .add_instance(String::from("bar"), String::from("bar"), group)
            .await?;

        let instance = app
            .prisma_client
            .instance()
            .find_unique(ShortpathEquals(String::from("bar")))
            .exec()
            .await?
            .unwrap();

        assert_eq!(instance.index, 0);
        assert_eq!(instance.group_id, *group);

        app.instance_manager().delete_group(group).await?;

        let instance = app
            .prisma_client
            .instance()
            .find_unique(ShortpathEquals(String::from("bar")))
            .exec()
            .await?
            .unwrap();

        // index should be `1` due to instance already present in default group.
        assert_eq!(instance.index, 1);
        assert_eq!(
            instance.group_id,
            *app.instance_manager().get_default_group().await?
        );

        Ok(())
    }

    #[tokio::test]
    async fn delete_group_empty() -> anyhow::Result<()> {
        let app = crate::setup_managers_for_test().await;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert no default group exists
        assert_eq!(group_count, 0);

        let group = app
            .instance_manager()
            .create_group(String::from("foo"))
            .await?;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert only the created group exists
        assert_eq!(group_count, 1);

        app.instance_manager().delete_group(group).await?;

        let group_count = app
            .prisma_client
            .instance_group()
            .count(vec![])
            .exec()
            .await?;

        // assert the default group was not created while deleting the new group
        assert_eq!(group_count, 0);

        Ok(())
    }
}
