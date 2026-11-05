//! Resolve exact constraints to a stable reachable graph before artifact IO.
use super::dependency::{ConflictingPin, DependencyGap, ReplacedPin, SkippedDependency};
use super::*;
use crate::domain::vtask::DependencyPlatform;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

type SharedResource = Arc<Mutex<BoxedResourceInstaller>>;
pub(super) type Plan = HashMap<String, Vec<SharedResource>>;

pub(super) struct Resolution {
    pub plan: Plan,
    /// Required dependencies left out because they cannot be resolved, and on
    /// a fresh install the ones the instance only has disabled. The selected
    /// addon still installs; the user is told which ones are missing or off.
    pub skipped: Vec<SkippedDependency>,
    /// Dependencies installed at another release than their dependant pins,
    /// because the pinned one is not for this game and loader.
    pub replaced: Vec<ReplacedPin>,
    /// Dependencies that dependants pin at different versions, installed at
    /// one of them for all.
    pub conflicting: Vec<ConflictingPin>,
}
fn project(id: &str) -> String {
    id.rsplit_once(':')
        .map_or(id, |(project, _)| project)
        .to_owned()
}

/// The platform and project a planner key (`platform:project`) names.
fn named(key: &str) -> (DependencyPlatform, String) {
    match key.split_once(':') {
        Some(("curseforge", project)) => (DependencyPlatform::Curseforge, project.to_owned()),
        Some((_, project)) => (DependencyPlatform::Modrinth, project.to_owned()),
        None => (DependencyPlatform::Modrinth, key.to_owned()),
    }
}

/// `replacing`: the root replaces an installed version of itself (an update).
/// A dependency the instance only has disabled counts as present either way
/// and stays disabled. A fresh install names it in a warning; an update says
/// nothing, since it only adds dependencies that are missing entirely.
///
/// Two dependants may pin different versions of one missing dependency. Only
/// one copy can be installed, so the newest pinned version is, for both, and
/// the user is told (the root's own version always stays what the user
/// picked). Server installs do the same.
pub(super) async fn resolve(
    root: SharedResource,
    app: &Arc<AppInner>,
    instance_id: InstanceId,
    replacing: bool,
    cancel: &CancellationToken,
) -> anyhow::Result<Resolution> {
    let root_id = root.lock().await.id();
    let root_key = project(&root_id);
    let mut hints = HashMap::from([(root_key.clone(), root_id.clone())]);
    let mut previous = HashSet::new();
    // Projects pinned at different versions, and the version installed for
    // every dependant. Decided once, then kept by every later pass.
    let mut settled = HashMap::from([(root_key.clone(), (root_id.clone(), root.clone()))]);
    for _ in 0..128 {
        let mut selected = HashMap::<String, SharedResource>::new();
        let mut identities = HashMap::<String, String>::new();
        let mut edges = HashMap::<String, Vec<String>>::new();
        let mut exact = BTreeMap::<String, BTreeSet<String>>::new();
        let mut skipped = Vec::<SkippedDependency>::new();
        let mut replaced = Vec::<ReplacedPin>::new();
        // Every pinned version, to settle a conflict between them.
        let mut pins = HashMap::from([(root_id.clone(), root.clone())]);
        // Projects a dependant pins at another version than the settled one.
        let mut yielded = BTreeSet::<String>::new();
        exact
            .entry(root_key.clone())
            .or_default()
            .insert(root_id.clone());
        let mut queue = vec![(root.clone(), true)];
        while let Some((resource, pinned)) = queue.pop() {
            if cancel.is_cancelled() {
                bail!("Addon installation cancelled during dependency planning");
            }
            let identity = resource.lock().await.id();
            let key = project(&identity);
            if pinned {
                exact
                    .entry(key.clone())
                    .or_default()
                    .insert(identity.clone());
            }
            if identities.contains_key(&key) {
                continue;
            }
            if selected.len() >= 512 {
                bail!("Required dependency graph exceeds 512 unique projects");
            }
            identities.insert(key.clone(), identity.clone());
            selected.insert(key.clone(), resource.clone());
            let dependencies = {
                let resource = resource.lock().await;
                let manager = app.instance_manager();
                let instances = manager.instances.read().await;
                let data = instances
                    .get(&instance_id)
                    .ok_or(InvalidInstanceIdError(instance_id))?
                    .data()?;
                resource
                    .dependencies(app, instance_id, data, ModChannel::Stable)
                    .collect::<Vec<_>>()
            };
            let mut required = Vec::new();
            for getter in dependencies {
                let mut choices = identities.clone();
                choices.extend(hints.clone());
                let resolved = tokio::select! {
                    result = getter(choices) => result,
                    _ = cancel.cancelled() => bail!("Addon installation cancelled during dependency planning"),
                };
                let Some(resource) = resolved else {
                    continue;
                };
                let resource = match resource {
                    Ok(resource) => resource,
                    Err(error) => match error.downcast_ref::<DependencyGap>() {
                        Some(DependencyGap::DisabledOnly {
                            dependency,
                            filenames,
                        }) => {
                            if replacing {
                                tracing::info!(
                                    dependency = %dependency.label(),
                                    filenames = ?filenames,
                                    "Updating next to a required dependency that is only installed disabled"
                                );
                            } else {
                                tracing::info!(
                                    dependency = %dependency.label(),
                                    filenames = ?filenames,
                                    "Installing next to a required dependency that is only installed disabled"
                                );
                                if !skipped.iter().any(|known| known.key == dependency.key) {
                                    skipped.push(dependency.clone());
                                }
                            }
                            continue;
                        }
                        Some(DependencyGap::Unresolvable(skip)) => {
                            if !skipped.iter().any(|known| known.key == skip.key) {
                                skipped.push(skip.clone());
                            }
                            continue;
                        }
                        _ => return Err(error),
                    },
                };
                if let Some(pin) = resource.replaced_pin() {
                    if !replaced.contains(&pin) {
                        replaced.push(pin);
                    }
                }
                let dependency_id = resource.id();
                let dependency_key = project(&dependency_id);
                let mut pinned = resource.exact_dependency();
                let mut resource = Arc::new(Mutex::new(resource));
                if pinned {
                    if let Some((chosen, installed)) = settled.get(&dependency_key) {
                        if *chosen != dependency_id {
                            tracing::info!(
                                pinned = %dependency_id,
                                installed = %chosen,
                                "A dependant pins another version of a required dependency than the one installed for all"
                            );
                            yielded.insert(dependency_key.clone());
                            pinned = false;
                            resource = installed.clone();
                        }
                    }
                }
                if pinned {
                    exact
                        .entry(dependency_key.clone())
                        .or_default()
                        .insert(dependency_id.clone());
                    pins.entry(dependency_id)
                        .or_insert_with(|| resource.clone());
                }
                required.push(dependency_key);
                queue.push((resource, pinned));
            }
            edges.insert(key, required);
        }
        // Settle each project pinned at several versions on the newest of them.
        let mut settling = false;
        for (key, ids) in &exact {
            if ids.len() < 2 || settled.contains_key(key) {
                continue;
            }
            let mut newest: Option<(&String, &SharedResource, _)> = None;
            for id in ids {
                let Some(resource) = pins.get(id) else {
                    continue;
                };
                let published = resource.lock().await.published();
                if newest.as_ref().is_none_or(|(_, _, best)| published > *best) {
                    newest = Some((id, resource, published));
                }
            }
            if let Some((id, resource, _)) = newest {
                tracing::info!(
                    dependency = %key,
                    pinned = ?ids,
                    installed = %id,
                    "Dependants pin different versions of a required dependency; installing the newest for all"
                );
                settled.insert(key.clone(), (id.clone(), resource.clone()));
                settling = true;
            }
        }
        let consistent = exact.values().all(|pins| pins.len() == 1);
        let next: HashMap<_, _> = exact
            .iter()
            .map(|(key, pins)| {
                let chosen = settled
                    .get(key)
                    .map(|(chosen, _)| chosen.clone())
                    .unwrap_or_else(|| pins.first().unwrap().clone());
                (key.clone(), chosen)
            })
            .collect();
        let satisfies = consistent
            && next
                .iter()
                .all(|(key, identity)| identities.get(key) == Some(identity));
        if satisfies && next == hints {
            // A provisional choice may have used a graph hint to avoid treating
            // the explicitly replaced root as an installed dependency. Before
            // accepting the closure, check every other final choice against
            // installed dependants; a hint grants no replacement authority.
            for (key, resource) in &selected {
                if key != &root_key {
                    validate_required_resource(&**resource.lock().await, app, instance_id).await?;
                }
            }
            let mut plan = HashMap::new();
            for (key, children) in edges {
                let identity = identities
                    .get(&key)
                    .context("Missing planned resource identity")?
                    .clone();
                let children = children
                    .into_iter()
                    .map(|child| {
                        selected
                            .get(&child)
                            .cloned()
                            .context("Required dependency missing from accepted closure")
                    })
                    .collect::<anyhow::Result<Vec<_>>>()?;
                plan.insert(identity, children);
            }
            // Another dependant may have resolved a project one getter skipped.
            skipped.retain(|skip| !identities.contains_key(&skip.key));
            // Only what is actually installed.
            replaced.retain(|pin| identities.get(&pin.key) == Some(&pin.identity));
            let mut conflicting = Vec::new();
            for key in yielded {
                let Some(resource) = selected.get(&key) else {
                    continue;
                };
                let resource = resource.lock().await;
                let (platform, project_id) = named(&key);
                conflicting.push(ConflictingPin {
                    name: resource.project_name(),
                    platform,
                    project_id,
                    installed: resource.version_number(),
                });
            }
            return Ok(Resolution {
                plan,
                skipped,
                replaced,
                conflicting,
            });
        }
        // Discard all resources/edges from the old provisional pass. Exact
        // requirements belonging only to now-unreachable metadata disappear.
        let signature = format!(
            "{:?}:{:?}",
            identities.iter().collect::<BTreeMap<_, _>>(),
            exact
        );
        if !previous.insert(signature) && !settling {
            bail!(
                "Required dependency exact pins are inconsistent or cannot resolve to a stable compatible graph"
            );
        }
        hints = next;
    }
    bail!("Required dependency planning exceeded 128 bounded refinements")
}

pub(super) async fn getters(
    resources: Vec<SharedResource>,
    path: &InstancePath,
) -> anyhow::Result<DependencyIterator<'static>> {
    let mut getters: Vec<ResourceInstallerGetter> = Vec::new();
    for resource in resources {
        let (id, name, kind, filename) = {
            let lock = resource.lock().await;
            (
                lock.id(),
                lock.display_name(),
                lock.addon_type(),
                publication::filename(&**lock, path)?,
            )
        };
        getters.push(Box::new(move |_| {
            Box::pin(async move {
                Some(Ok(Box::new(PlannedResource {
                    resource,
                    id,
                    name,
                    kind,
                    filename,
                }) as BoxedResourceInstaller))
            })
        }));
    }
    Ok(DependencyIterator::new(getters.into_iter()))
}
struct PlannedResource {
    resource: SharedResource,
    id: String,
    name: String,
    kind: instance::AddonType,
    filename: String,
}
#[async_trait::async_trait]
impl ResourceInstaller for PlannedResource {
    fn id(&self) -> String {
        self.id.clone()
    }
    fn display_name(&self) -> String {
        self.name.clone()
    }
    fn addon_type(&self) -> instance::AddonType {
        self.kind
    }
    fn get_install_path(&self, path: &InstancePath) -> PathBuf {
        self.kind.get_folder_path(path).join(&self.filename)
    }
    async fn downloadable(&self, path: &InstancePath) -> Option<Downloadable> {
        self.resource.lock().await.downloadable(path).await
    }
    async fn post_process(
        &self,
        path: &InstancePath,
        staged: &std::path::Path,
    ) -> anyhow::Result<()> {
        self.resource.lock().await.post_process(path, staged).await
    }
    fn dependencies(
        &self,
        _: &Arc<AppInner>,
        _: InstanceId,
        _: &InstanceData,
        _: ModChannel,
    ) -> DependencyIterator {
        DependencyIterator::new(std::iter::empty())
    }
    async fn is_already_installed(
        &self,
        app: &Arc<AppInner>,
        id: InstanceId,
    ) -> anyhow::Result<bool> {
        self.resource
            .lock()
            .await
            .is_already_installed(app, id)
            .await
    }
    async fn rollback(&self, data: &mut InstanceData) -> anyhow::Result<()> {
        self.resource.lock().await.rollback(data).await
    }
    async fn record_identity(&self, app: &Arc<AppInner>, metadata_id: &str) -> anyhow::Result<()> {
        self.resource
            .lock()
            .await
            .record_identity(app, metadata_id)
            .await
    }
}
