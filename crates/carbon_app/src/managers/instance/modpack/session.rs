//! The sole durable setup authority. Admission is serialized by the existing
//! instance gate; this record does not replace that gate or RuntimeLifecycle.
use super::{RepairMarkerFile, packinfo};
use crate::domain::instance::{InstanceId, info};
use crate::managers::instance::schema;
use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const RECORD: &str = "setup-session.json";

/// Staged sources [`Session::verify_sources`] hashes at once, as the disk
/// scans do.
const VERIFY_CONCURRENCY: usize = 8;

/// What [`Session::verify_sources`] found, for the apply that runs next.
#[derive(Debug, Default)]
pub struct SourceCheck {
    /// Live files (bare or `.disabled` twin) that an interrupted earlier
    /// attempt already published from their staged source: as a second name
    /// of it, or, on a volume without hard links, as the staged file itself,
    /// moved there. That attempt may have stopped before it synced their
    /// folders, and committing deletes the staged names, so the apply syncs
    /// those folders too.
    pub published: Vec<PathBuf>,
    /// Keys whose staged source no longer holds the target bytes. Only a
    /// resumed apply reports these; the apply never publishes from them.
    pub changed: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Owner {
    pub instance_id: i32,
    pub operation: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Producer {
    Change,
    Repair,
    Create,
    Import,
    Settings,
    Reconstructed,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Kind {
    Pack,
    VersionOnly,
    ImportedComplete,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Reserved,
    Resolved,
    Applying,
    Applied,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub packinfo: String,
    pub digest: String,
    pub options_digest: String,
    /// Exact original staged spelling for each staged member. Download skips
    /// are recorded separately: a vanished original is never a skip or Delete.
    pub sources: BTreeMap<String, String>,
    pub skipped: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Archive {
    pub name: String,
    pub sha512: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptedPreparation {
    pub generation: String,
    pub task_id: i32,
    pub instance_id: i32,
    pub operation: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Session {
    pub schema_version: u32,
    pub operation: String,
    /// None is private pre-ID provenance, never admitted live authority.
    pub instance_id: Option<i32>,
    pub producer: Producer,
    pub kind: Kind,
    pub phase: Phase,
    pub installed_config: String,
    pub pending_config: Option<String>,
    pub requested_pack: Option<String>,
    pub original_options: Option<RepairMarkerFile>,
    pub target: Option<Target>,
    pub archive: Option<Archive>,
    #[serde(default)]
    pub copied_to: Option<i32>,
    #[serde(default)]
    pub adopted_from: Option<Owner>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_preparation: Option<AcceptedPreparation>,
    /// Set on a Repair that replaced a setup which may have changed installed
    /// content (see [`Session::interrupted`]): the replaced setup's retired
    /// label. The live files may be part-way to this repair's target, so the
    /// repair is never cancelled back to the configuration it started from,
    /// its target is never previewed against the installed record, and a
    /// Repair that replaces it in turn installs the same target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replaces_interrupted: Option<String>,
    /// What the user should hear once this setup succeeds (see
    /// [`Request::warnings`]). Kept here because the task that finishes a
    /// setup need not be the one that requested it: a failed or interrupted
    /// attempt is finished by Play or Resume, and a failed task's warnings are
    /// not reported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<crate::domain::vtask::TaskWarning>,
}

pub fn binding(
    target: &packinfo::PackInfo,
    options: Option<&RepairMarkerFile>,
) -> anyhow::Result<(String, String)> {
    let ordered: BTreeMap<_, _> = target
        .files
        .iter()
        .map(|(path, hashes)| (path, (hex::encode(hashes.sha512), hex::encode(hashes.md5))))
        .collect();
    Ok((
        hex::encode(Sha512::digest(serde_json::to_vec(&ordered)?)),
        hex::encode(Sha512::digest(serde_json::to_vec(&options)?)),
    ))
}

/// A folder the setup made inside the instance folder (`.setup`, its staging
/// and replacements, `.repair-recovery`): a link there is not the setup's.
pub fn plain_directory(path: &Path) -> anyhow::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("setup requires a plain directory: {}", path.display());
    }
    Ok(())
}

/// The instance folder itself may be a link or junction (a pack moved to
/// another drive and linked back), which 2.0.40 updated and repaired through,
/// so it is followed. The folders the setup makes inside it stay plain (see
/// [`plain_directory`]).
pub fn instance_directory(path: &Path) -> anyhow::Result<()> {
    match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => bail!("the instance folder is not a folder: {}", path.display()),
        Err(error) => Err(error)
            .with_context(|| format!("could not open the instance folder {}", path.display())),
    }
}

pub async fn sync_dir(path: &Path) -> anyhow::Result<()> {
    // A volume that does not sync folders (a VirtualBox shared folder, some
    // network mounts) leaves the entries to the system: 2.0.40 changed a game
    // version in Settings there without a folder sync.
    crate::util::durable::sync_dir_async(path).await?;
    Ok(())
}

pub async fn atomic_json<T: Serialize>(path: &Path, value: &T, new: bool) -> anyhow::Result<()> {
    atomic_bytes(path, serde_json::to_vec_pretty(value)?, new).await
}

/// Publishes `bytes` as `path`, in a folder the setup made (see
/// [`plain_directory`]).
pub async fn atomic_bytes(path: &Path, bytes: Vec<u8>, new: bool) -> anyhow::Result<()> {
    let parent = path.parent().context("session path has no parent")?;
    plain_directory(parent)?;
    publish_bytes(path, bytes, new).await
}

/// [`atomic_bytes`] for a record directly in the instance folder, such as
/// `tmp-packinfo.json` (see [`instance_directory`]).
pub async fn atomic_bytes_in_root(path: &Path, bytes: Vec<u8>, new: bool) -> anyhow::Result<()> {
    let parent = path.parent().context("session path has no parent")?;
    instance_directory(parent)?;
    publish_bytes(path, bytes, new).await
}

async fn publish_bytes(path: &Path, bytes: Vec<u8>, new: bool) -> anyhow::Result<()> {
    let path = path.to_owned();
    let parent = path
        .parent()
        .context("session path has no parent")?
        .to_owned();
    let sync_parent = parent.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        use std::io::Write;
        let mut temporary = crate::util::publish::temp_in(&parent)?;
        temporary.write_all(&bytes)?;
        crate::util::durable::sync_file(temporary.as_file())?;
        #[cfg(test)]
        let publication_pause = publication_fixture::wait(&path);
        if new {
            crate::util::publish::persist_noclobber(temporary, &path)?;
        } else {
            crate::util::publish::persist(temporary, &path)?;
        }
        #[cfg(test)]
        if let Some(pause) = publication_pause {
            pause.finished.notify_one();
        }
        Ok(())
    })
    .await??;
    sync_dir(&sync_parent).await
}

impl Session {
    pub fn new(
        config: &info::Instance,
        producer: Producer,
        kind: Kind,
        options: Option<RepairMarkerFile>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            schema_version: 1,
            operation: uuid::Uuid::new_v4().to_string(),
            instance_id: None,
            producer,
            kind,
            phase: Phase::Reserved,
            installed_config: schema::make_instance_config(config.clone())?,
            pending_config: None,
            requested_pack: None,
            original_options: options,
            target: None,
            archive: None,
            copied_to: None,
            adopted_from: None,
            accepted_preparation: None,
            replaces_interrupted: None,
            warnings: Vec::new(),
        })
    }
    pub fn bind_preparation(
        &mut self,
        generation: &str,
        task: crate::domain::vtask::VisualTaskId,
        current: &info::Instance,
    ) -> anyhow::Result<()> {
        let owner = self.owner()?;
        self.accepted_preparation = Some(AcceptedPreparation {
            generation: generation.to_owned(),
            task_id: task.0,
            instance_id: owner.instance_id,
            operation: owner.operation,
            version: version_key(&current.game_configuration.version),
        });
        Ok(())
    }

    /// A retained task and its exact current-core Session certify recovery
    /// authority, never successful preparation. The resumed task must succeed.
    pub fn validate_shader_recovery(
        &self,
        current: &info::Instance,
        instance: InstanceId,
        expected_task: crate::domain::vtask::VisualTaskId,
        expected_mc: &str,
        generation: &str,
        failed_task: Option<crate::domain::vtask::VisualTaskId>,
        outcome: &crate::managers::vtask::TaskOutcome,
    ) -> anyhow::Result<()> {
        use crate::managers::vtask::TaskOutcome;
        if failed_task != Some(expected_task)
            || (!matches!(outcome, TaskOutcome::Failed(task) if task.id == expected_task)
                && !matches!(outcome, TaskOutcome::Dismissed))
        {
            bail!(
                "The original setup is not a settled failed attempt. Check its status before retrying."
            );
        }
        let owner = self.owner()?;
        let accepted = self.accepted_preparation.as_ref().context(
            "This setup has no current-core preparation binding. Recover it from the instance.",
        )?;
        if owner.instance_id != instance.0
            || accepted.instance_id != instance.0
            || accepted.operation != owner.operation
            || accepted.generation != generation
            || accepted.task_id != expected_task.0
            || self.copied_to.is_some()
            || self.adopted_from.is_some()
            || self.producer != Producer::Settings
            || self.kind != Kind::VersionOnly
        {
            bail!("The setup belongs to another operation. Recover it from the instance.");
        }
        let original = schema::parse_instance_config(&self.installed_config)?;
        let Some(info::GameVersion::Standard(version)) = &original.game_configuration.version
        else {
            bail!("The original setup has no supported game target.");
        };
        if version.release != expected_mc
            || version.modloaders.len() != 1
            || !version
                .modloaders
                .iter()
                .any(|loader| loader.type_ == info::ModLoaderType::Fabric)
        {
            bail!("The original Fabric target changed. Recover it from the instance.");
        }
        let target = version_key(&original.game_configuration.version);
        if accepted.version != target
            || version_key(&current.game_configuration.version) != target
            || current.modpack.as_ref().map(|pack| &pack.modpack)
                != original.modpack.as_ref().map(|pack| &pack.modpack)
        {
            bail!("The game or loader changed independently. Recover it from the instance.");
        }
        if let Some(pending) = &self.pending_config {
            let pending = schema::parse_instance_config(pending)?;
            if version_key(&pending.game_configuration.version) != target
                || pending.modpack.as_ref().map(|pack| &pack.modpack)
                    != original.modpack.as_ref().map(|pack| &pack.modpack)
            {
                bail!("The pending target changed. Recover it from the instance.");
            }
        }
        Ok(())
    }

    pub fn owner(&self) -> anyhow::Result<Owner> {
        Ok(Owner {
            instance_id: self.instance_id.context(
                "setup has private, unbound provenance; preserve and reconstruct original inputs",
            )?,
            operation: self.operation.clone(),
        })
    }
    pub fn check_owner(&self, owner: &Owner) -> anyhow::Result<()> {
        if self.owner()? != *owner {
            bail!("setup belongs to a different instance/operation; preserved for recovery");
        }
        Ok(())
    }
    pub async fn load(root: &Path) -> anyhow::Result<Option<Self>> {
        #[cfg(test)]
        load_count::record(root);
        let setup = root.join(".setup");
        if !setup.try_exists()? {
            return Ok(None);
        }
        plain_directory(&setup)?;
        let path = setup.join(RECORD);
        let metadata = match tokio::fs::symlink_metadata(&path).await {
            Ok(metadata) => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            bail!("setup session is not a plain file");
        }
        let session: Self = serde_json::from_slice(&tokio::fs::read(&path).await?)
            .context("unsupported or malformed setup session; preserved for recovery")?;
        if session.schema_version != 1 || uuid::Uuid::parse_str(&session.operation).is_err() {
            bail!("unsupported setup session; preserved for recovery");
        }
        schema::parse_instance_config(&session.installed_config)?;
        if let Some(config) = &session.pending_config {
            schema::parse_instance_config(config)?;
        }
        if let Some(target) = &session.target {
            let info = packinfo::parse_packinfo(&target.packinfo)?;
            let expected = binding(&info, session.original_options.as_ref())?;
            if expected != (target.digest.clone(), target.options_digest.clone()) {
                bail!("setup target binding is invalid");
            }
            for (key, spelling) in &target.sources {
                if !info.files.contains_key(key)
                    || super::normalize_pack_path(spelling).as_ref() != Some(spelling)
                    || spelling.strip_suffix(".disabled").unwrap_or(spelling) != key
                {
                    bail!("invalid original source binding");
                }
            }
        }
        Ok(Some(session))
    }
    pub async fn require(root: &Path, owner: &Owner) -> anyhow::Result<Self> {
        let session = Self::load(root)
            .await?
            .context("owned setup session disappeared; preserved for recovery")?;
        session.check_owner(owner)?;
        Ok(session)
    }
    /// Private recovery keeps the exact authority captured before settings
    /// awaits. A new valid Session for the same instance is still foreign work.
    pub async fn require_preparation(root: &Path, expected: &Self) -> anyhow::Result<Self> {
        let session = Self::require(root, &expected.owner()?).await?;
        if serde_json::to_vec(&session)? != serde_json::to_vec(expected)? {
            bail!("The original setup changed. Recover it from the instance.");
        }
        Ok(session)
    }
    pub async fn create(&self, root: &Path) -> anyhow::Result<()> {
        atomic_json(&root.join(".setup").join(RECORD), self, true).await?;
        sync_dir(root).await
    }
    pub async fn save(&self, root: &Path, owner: &Owner) -> anyhow::Result<()> {
        Self::require(root, owner).await?;
        self.check_owner(owner)?;
        atomic_json(&root.join(".setup").join(RECORD), self, false).await
    }
    pub async fn bind_private(mut self, root: &Path, id: InstanceId) -> anyhow::Result<Self> {
        let saved = Self::load(root)
            .await?
            .context("private setup provenance disappeared")?;
        if saved.instance_id.is_some() || saved.operation != self.operation {
            bail!("private setup provenance changed");
        }
        self.instance_id = Some(id.0);
        atomic_json(&root.join(".setup").join(RECORD), &self, false).await?;
        Ok(self)
    }
    pub async fn bind_target(
        &mut self,
        root: &Path,
        staged: &Path,
        target: &packinfo::PackInfo,
        skipped: Vec<String>,
    ) -> anyhow::Result<()> {
        let mut sources = BTreeMap::new();
        let mut walker = crate::util::NormalizedWalkdir::new(&staged.join("instance"))?;
        while let Some(entry) = walker.next()? {
            if entry.is_dir {
                continue;
            }
            let spelling = entry.relative_path.to_owned();
            let key = spelling
                .strip_suffix(".disabled")
                .unwrap_or(&spelling)
                .to_owned();
            if !target.files.contains_key(&key) {
                bail!("original staged source {spelling} is not part of the complete target");
            }
            // Preparation keeps only the bare spelling of a file a pack ships
            // both ways (see `drop_shadowed_disabled_twins`), so a second one
            // here appeared after the scan.
            if let Some(other) = sources.insert(key, spelling.clone()) {
                bail!(
                    "The modpack files {} and {} were both prepared for the same file, so the modpack can't be installed as prepared. Press Play to prepare it again.",
                    other.trim_start_matches('/'),
                    spelling.trim_start_matches('/')
                );
            }
        }
        for key in target.files.keys() {
            if !sources.contains_key(key) && !skipped.contains(key) {
                bail!("complete target has no original input for {key}");
            }
        }
        let (digest, options_digest) = binding(target, self.original_options.as_ref())?;
        self.target = Some(Target {
            packinfo: packinfo::make_packinfo(target.clone())?,
            digest,
            options_digest,
            sources,
            skipped,
        });
        self.phase = Phase::Resolved;
        // `target` is the scan of these very files, hashed moments ago. The
        // sources are hashed again once, right before the apply publishes
        // them (`verify_sources`).
        let _ = root;
        Ok(())
    }

    /// Hashes every original staged source against the complete target. The
    /// apply runs this once, under the instance gate, before it publishes
    /// anything.
    ///
    /// The apply publishes a staged file as a hard link, so after an
    /// interrupted apply some sources already are their live file (or its
    /// `.disabled` twin) in `live`. Those hold the live file's bytes now,
    /// including any edit made in place since, and the resumed planner judges
    /// the live file itself, so they are not held to the target hash here.
    /// For the same reason a resumed apply reports a source that no longer
    /// matches instead of failing: it may have been edited in place through
    /// its live name, which was then deleted or replaced.
    ///
    /// On a volume without hard links the apply moves the staged file into
    /// place instead, as 2.0.40 did, so a resumed apply also meets sources
    /// that are gone from staging while their live file (or twin) is there.
    /// Those count as published the same way: the planner judges the live
    /// file and never needs their staged copy, which it no longer finds.
    /// Every other source must still exist.
    pub async fn verify_sources(&self, staged: &Path, live: &Path) -> anyhow::Result<SourceCheck> {
        let bound = self
            .target
            .as_ref()
            .context("pack setup lacks complete original target")?;
        let target = packinfo::parse_packinfo(&bound.packinfo)?;
        // Play fails the same way every time, so name the way out.
        let way_out = self.staged_way_out();
        let mut check = SourceCheck::default();
        // Only an apply that has started (Applying is saved before the first
        // publication) can have published a source.
        let resumed = self.phase == Phase::Applying;
        let published = if resumed {
            let sources: Vec<_> = bound
                .sources
                .iter()
                .map(|(key, spelling)| {
                    let bare = live.join(key.trim_start_matches('/'));
                    let mut twin = bare.clone().into_os_string();
                    twin.push(".disabled");
                    (
                        spelling.clone(),
                        staged
                            .join("instance")
                            .join(spelling.trim_start_matches('/')),
                        bare,
                        PathBuf::from(twin),
                    )
                })
                .collect();
            let published = tokio::task::spawn_blocking(move || {
                sources
                    .into_iter()
                    .filter_map(|(spelling, source, bare, twin)| {
                        let names = [bare, twin];
                        let linked = names
                            .iter()
                            .find(|name| crate::util::publish::same_file(name, &source));
                        let moved = || {
                            let gone = matches!(
                                std::fs::symlink_metadata(&source),
                                Err(error) if error.kind() == std::io::ErrorKind::NotFound
                            );
                            gone.then(|| names.iter().find(|name| name.symlink_metadata().is_ok()))
                                .flatten()
                        };
                        linked.or_else(moved).map(|name| (spelling, name.clone()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .await?;
            check.published = published.values().cloned().collect();
            published
        } else {
            BTreeMap::new()
        };
        // Hashed a few at a time, like the scans, then judged in key order so
        // the same damage always reports the same file first. The inputs are
        // owned: borrowing futures here would keep the launch task from
        // being Send.
        let pending: Vec<(String, String, PathBuf)> = bound
            .sources
            .iter()
            .filter(|(_, spelling)| !published.contains_key(*spelling))
            .map(|(key, spelling)| {
                let path = staged
                    .join("instance")
                    .join(spelling.trim_start_matches('/'));
                (key.clone(), spelling.clone(), path)
            })
            .collect();
        let mut hashed: Vec<(String, String, anyhow::Result<[u8; 64]>)> = {
            use futures::StreamExt;
            futures::stream::iter(pending)
                .map(|(key, spelling, path)| async move {
                    let hash = super::origin_check::hash_file_sha512(&path).await;
                    (key, spelling, hash)
                })
                .buffer_unordered(VERIFY_CONCURRENCY)
                .collect()
                .await
        };
        hashed.sort_by(|a, b| a.0.cmp(&b.0));
        for (key, spelling, hash) in hashed {
            let relative = spelling.trim_start_matches('/');
            let hash = hash.with_context(|| format!("A file downloaded for this modpack ({relative}) went missing or became unreadable before it could be installed, often because antivirus software removed it. {way_out}"))?;
            if hash != target.files[&key].sha512 {
                if !resumed {
                    bail!(
                        "A file downloaded for this modpack ({relative}) changed before it could be installed, often because antivirus software modified it. {way_out}"
                    );
                }
                tracing::warn!(
                    "Staged source {spelling} changed after its setup started applying; it will not be installed"
                );
                check.changed.insert(key);
            }
        }
        Ok(check)
    }
    /// The way out of a staged file that went missing or changed: Repair
    /// downloads a pack linked to a platform again. A pack imported from a
    /// file that no platform publishes can't be repaired (Repair needs the
    /// instance linked to a modpack, which the installed configuration stays
    /// until the setup commits), so it is imported again instead.
    pub(crate) fn staged_way_out(&self) -> &'static str {
        if schema::parse_instance_config(&self.installed_config)
            .is_ok_and(|config| config.modpack.is_some())
        {
            "Use Repair on this instance to download the modpack again; your worlds are kept."
        } else {
            "This instance isn't linked to a modpack that Repair can download again, so import the modpack file again as a new instance, and delete this one once the new one works."
        }
    }
    /// Returns a Resolved pack setup to Reserved, so the next Play prepares
    /// it again: its bound target was judged against files that changed
    /// since (a file the preparation skipped because it was already in place
    /// went missing or changed before the apply). The apply never started
    /// (it saves Applying before it publishes anything), so nothing installed
    /// changed and nothing staged was published.
    ///
    /// The root `tmp-packinfo.json`, which records the released target, is
    /// kept beside the setup as `prior-record-*`, as a replaced preparation
    /// record is. The staged files become an earlier attempt's downloads,
    /// which the next preparation reuses once their hashes match. Callers
    /// hold the instance gate.
    pub async fn release_target(&mut self, root: &Path, owner: &Owner) -> anyhow::Result<()> {
        if self.kind != Kind::Pack || self.phase != Phase::Resolved {
            bail!("only a pack setup that hasn't started applying can be prepared again");
        }
        let setup = root.join(".setup");
        let temporary = root.join("tmp-packinfo.json");
        if entry_metadata(&temporary)?.is_some() {
            tokio::fs::rename(
                &temporary,
                setup.join(format!(
                    "prior-record-tmp-packinfo.json-{}",
                    uuid::Uuid::new_v4()
                )),
            )
            .await?;
        }
        self.target = None;
        self.phase = Phase::Reserved;
        self.save(root, owner).await?;
        // After the save: a staging beside a Resolved record is what the
        // apply reads. Beside a Reserved one it is only linked from.
        let staging = setup.join("staging");
        if entry_metadata(&staging)?.is_some_and(|metadata| is_plain_dir(&metadata)) {
            let earlier = setup.join(format!("preparing-{}", uuid::Uuid::new_v4()));
            if let Err(error) = crate::util::publish::rename_dir(&staging, &earlier).await {
                tracing::warn!(?error, "released staging kept in place");
            }
        }
        sync_dir(&setup).await?;
        sync_dir(root).await
    }

    /// The record for a sessionless `.setup` that 2.0.40 marked
    /// modpack-complete: an import whose files were copied in, or a
    /// version-only install whose pack step had finished. Only the game
    /// version install remains, so nothing is staged or applied for it.
    pub fn legacy_complete(
        config: &info::Instance,
        instance: InstanceId,
        requested_pack: Option<String>,
    ) -> anyhow::Result<Self> {
        let mut session = Self::new(
            config,
            Producer::Reconstructed,
            Kind::ImportedComplete,
            None,
        )?;
        session.instance_id = Some(instance.0);
        session.phase = Phase::Committed;
        session.requested_pack = requested_pack;
        Ok(session)
    }

    /// Adopts a `.setup` without a record whose pack install, update or
    /// repair 2.0.40 (or an earlier build) finished applying before its game
    /// version install failed (see [`legacy_pack_applied`]). It becomes a
    /// committed pack setup whose target is the `packinfo.json` that apply
    /// promoted, so Play goes straight to the game version install, as
    /// 2.0.40 did, instead of rebuilding the pack from its archive (which
    /// needs the platform's API and every file it lists to still resolve)
    /// and applying it again. Like any fully installed pack (see
    /// [`pack_installed`](Self::pack_installed)), a version or loader change
    /// may still replace it.
    ///
    /// 2.0.40 wrote the requested pack and its game version to the
    /// configuration before applying, so the configuration is both the
    /// installed and the pending one (one the user unpaired since names no
    /// pack). When it names another pack, has no game version or a record
    /// can't be read, nothing is written and `None` is returned: Play
    /// rebuilds such a setup from its archive. Callers hold the instance gate
    /// (or the instance's own preparation).
    pub async fn adopt_legacy_applied_pack(
        root: &Path,
        instance: InstanceId,
        config: &info::Instance,
    ) -> anyhow::Result<Option<Self>> {
        if config.game_configuration.version.is_none() || !legacy_pack_applied(root).await? {
            return Ok(None);
        }
        let setup = root.join(".setup");
        let read = |name: &str| {
            let path = setup.join(name);
            async move {
                match tokio::fs::read_to_string(path).await {
                    Ok(text) => Ok(Some(text)),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(error) => Err(error),
                }
            }
        };
        let requested_pack = read("change-pack-version.json").await?;
        if let Some(text) = &requested_pack {
            let Ok(requested) = serde_json::from_str::<super::PackVersionFile>(text) else {
                return Ok(None);
            };
            let requested = info::Modpack::from(requested);
            if config
                .modpack
                .as_ref()
                .is_some_and(|pack| pack.modpack != requested)
            {
                return Ok(None);
            }
        }
        let options = match read("repair").await? {
            None => None,
            Some(text) => match serde_json::from_str::<RepairMarkerFile>(&text) {
                Ok(options) => Some(options),
                Err(_) => return Ok(None),
            },
        };
        let Ok(installed) =
            packinfo::parse_packinfo(&tokio::fs::read_to_string(root.join("packinfo.json")).await?)
        else {
            return Ok(None);
        };
        let (digest, options_digest) = binding(&installed, options.as_ref())?;
        let mut session = Self::new(config, Producer::Reconstructed, Kind::Pack, options)?;
        session.instance_id = Some(instance.0);
        session.phase = Phase::Committed;
        session.requested_pack = requested_pack;
        session.pending_config = Some(session.installed_config.clone());
        session.target = Some(Target {
            packinfo: packinfo::make_packinfo(installed.clone())?,
            digest,
            options_digest,
            // Nothing is staged: every file is already in place.
            sources: BTreeMap::new(),
            skipped: installed
                .files
                .into_keys()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        });
        session.create(root).await?;
        Ok(Some(session))
    }

    /// Publishes a session for a `.setup` that 2.0.40 (or an earlier 2.0.x
    /// build) left without a record, when [`classify_legacy`] proves it never
    /// reached pack apply. The current configuration is still the installed
    /// state for those shapes, so it becomes `installed_config`. The record is
    /// created no-clobber; callers hold the instance gate (or the instance's
    /// own preparation).
    pub async fn adopt_legacy(
        root: &Path,
        instance: InstanceId,
        config: &info::Instance,
    ) -> anyhow::Result<Option<Self>> {
        let Some(request) = classify_legacy(root).await? else {
            return Ok(None);
        };
        let over_installed = request.over_installed_pack(config);
        let mut session = match request {
            LegacyRequest::VersionOnly => {
                // The same record the current create_instance writes for a
                // non-pack instance.
                Self::new(config, Producer::Create, Kind::VersionOnly, None)?
            }
            LegacyRequest::Pack {
                requested_pack,
                options,
                ..
            } => {
                // Only a never-finished first install has no installed version
                // to cancel back to. A 2.0.40 Repair of a first install that
                // never finished is still that first install: it keeps its
                // repair options, so Play runs it as the repair it was, but it
                // is never cancellable.
                let producer = match (over_installed, options.is_some()) {
                    (false, _) => Producer::Create,
                    (true, true) => Producer::Repair,
                    (true, false) => Producer::Change,
                };
                let mut session = Self::new(config, producer, Kind::Pack, options)?;
                session.requested_pack = Some(requested_pack);
                session
            }
            LegacyRequest::Complete { requested_pack } => {
                Self::legacy_complete(config, instance, requested_pack)?
            }
        };
        session.instance_id = Some(instance.0);
        session.create(root).await?;
        Ok(Some(session))
    }

    /// The pending setup, adopting a provably untouched legacy one first.
    pub async fn load_or_adopt_legacy(
        root: &Path,
        instance: InstanceId,
        config: &info::Instance,
    ) -> anyhow::Result<Option<Self>> {
        match Self::load(root).await? {
            Some(session) => Ok(Some(session)),
            None => Self::adopt_legacy(root, instance, config).await,
        }
    }

    /// Whether this is `instance`'s own record: bound to it, and not a copy a
    /// duplicate made of another instance's.
    pub fn owned_by(&self, instance: InstanceId) -> bool {
        self.instance_id == Some(instance.0) && self.copied_to.is_none()
    }

    /// Whether this record is provenance rather than ownership for
    /// `instance`, the instance registered for the folder it is in: any
    /// record that is not its own (see [`owned_by`](Self::owned_by)). It was
    /// produced before the instance had an id, copied by a duplicate
    /// (`copied_to`), or kept from an earlier registration of the folder: the
    /// instance scan gives a folder a new id after a database reset or
    /// restore (including the one going back to 2.0.40 needs), or once the
    /// folder was copied, moved or renamed outside the launcher. An instance
    /// only ever works in its own folder, so no other instance acts on a
    /// record found in this one. Like a duplicate's, such a record is
    /// continued as `instance`'s own only when it qualifies (see
    /// [`copy_qualification`](Self::copy_qualification)), or rebuilt from its
    /// original archive; `copied_to` only tells the messages it was a copy.
    pub fn binds_to(&self, instance: InstanceId) -> bool {
        !self.owned_by(instance)
    }

    /// Whether `instance` continues such a record under its own identity.
    /// `None`: the record is not one it qualifies (its own, or a pack setup
    /// past its download, which reconstruction handles). `Some(false)`:
    /// it would qualify but shows apply evidence. Non-pack records never run
    /// pack apply. A pack request qualifies only before its archive arrived,
    /// and only when its repair options hold nothing bound to the source
    /// instance's storage: legacy cleanup paths are preserved, never executed,
    /// and the copy rescans its own files.
    pub async fn copy_qualification(
        &self,
        root: &Path,
        instance: InstanceId,
    ) -> anyhow::Result<Option<bool>> {
        if !self.binds_to(instance) {
            return Ok(None);
        }
        let unbound_options = self.original_options.as_ref().is_none_or(|options| {
            options.cleanup_selections.is_empty() && options.cleanup_batch.is_none()
        });
        let pre_download_pack = self.kind == Kind::Pack
            && self.phase == Phase::Reserved
            && self.target.is_none()
            && self.archive.is_none()
            && unbound_options
            && untouched_beside_record(root).await?;
        if self.kind == Kind::Pack && !pre_download_pack {
            return Ok(None);
        }
        let setup = root.join(".setup");
        let ambiguous = self.target.is_some()
            || self.original_options.is_some() && !pre_download_pack
            || !matches!(self.phase, Phase::Reserved | Phase::Committed)
            || setup.join("apply-started").try_exists()?
            || setup.join("representation-transitions").try_exists()?
            || setup.join("staging-packinfo.json").try_exists()?;
        Ok(Some(!ambiguous))
    }

    /// Republishes a record that [`copy_qualification`](Self::copy_qualification)
    /// accepted as `instance`'s own: a new operation when it had an owner,
    /// which stays as `adopted_from`. Callers hold the instance gate (or the
    /// instance's own preparation).
    pub async fn qualify_copy(&self, root: &Path, instance: InstanceId) -> anyhow::Result<Self> {
        let mut qualified = self.clone();
        qualified.adopted_from = self.owner().ok();
        if self.instance_id.is_some() {
            qualified.operation = uuid::Uuid::new_v4().to_string();
        }
        qualified.instance_id = Some(instance.0);
        qualified.copied_to = None;
        atomic_json(&root.join(".setup").join(RECORD), &qualified, false).await?;
        Ok(qualified)
    }

    /// A setup that never stages or applies pack content: a version-only
    /// install (a new instance, a version or loader change, or a 2.0.40
    /// leftover) or a completed import (including a 2.0.40 modpack-complete
    /// leftover) whose files are in place and only waits for its game
    /// version. Preparation marks such a setup Committed before its Java,
    /// Minecraft and loader steps, and it stays until they succeed, so a
    /// failed or cancelled install is content-free in either phase. Change
    /// and Repair only ever produce pack setups, so a record claiming
    /// otherwise is not trusted.
    pub fn is_content_free(&self) -> bool {
        matches!(
            self.producer,
            Producer::Create | Producer::Import | Producer::Settings | Producer::Reconstructed
        ) && self.kind != Kind::Pack
            && matches!(self.phase, Phase::Reserved | Phase::Committed)
            && self.target.is_none()
            && self.original_options.is_none()
            && self.archive.is_none()
            && self.pending_config.is_none()
    }

    /// A pack setup whose pack is fully installed: its files were applied,
    /// the instance's `packinfo.json` is still exactly the target it promoted
    /// and its configuration was published. Like a completed import, only the
    /// game version install remains; Play would only re-check that record
    /// (as `finish_session_commit` does) before installing the game version.
    /// Its archive and apply markers are the finished install's leftovers.
    pub async fn pack_installed(&self, root: &Path) -> anyhow::Result<bool> {
        let Some(target) = self.target.as_ref() else {
            return Ok(false);
        };
        if self.kind != Kind::Pack
            || self.phase != Phase::Committed
            || entry_metadata(&root.join("tmp-packinfo.json"))?.is_some()
        {
            return Ok(false);
        }
        let promoted = root.join("packinfo.json");
        if !entry_metadata(&promoted)?.is_some_and(|metadata| is_plain_file(&metadata)) {
            return Ok(false);
        }
        let Ok(installed) = packinfo::parse_packinfo(&tokio::fs::read_to_string(&promoted).await?)
        else {
            return Ok(false);
        };
        Ok(binding(&installed, self.original_options.as_ref())?.0 == target.digest)
    }

    /// Whether a new game version or loader choice may retire this setup,
    /// whoever it belongs to: it is content-free with no pack input or apply
    /// marker beside it, or its pack is fully installed.
    async fn replaceable(&self, root: &Path) -> anyhow::Result<bool> {
        if !self.is_content_free() {
            return self.pack_installed(root).await;
        }
        let setup = root.join(".setup");
        for marker in [
            "apply-started",
            "representation-transitions",
            "staging-packinfo.json",
            "curseforge",
            "modrinth",
            "gdlpack",
        ] {
            if entry_metadata(&setup.join(marker))?.is_some() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether a new game version or loader choice may retire this pending
    /// setup instead of being refused: it belongs to this instance and is
    /// [`replaceable`](Self::replaceable).
    pub async fn supersedable(&self, root: &Path, instance: InstanceId) -> anyhow::Result<bool> {
        if !self.owned_by(instance) {
            return Ok(false);
        }
        self.replaceable(root).await
    }

    /// Whether this is a content-free record that `instance` continues as its
    /// own (one a duplicate copied, one written before the instance had an
    /// id, or one kept from an earlier registration of its folder; see
    /// [`binds_to`](Self::binds_to)) and that is
    /// supersedable once it does. Play makes such a record the instance's own
    /// before running it; a version or loader change may do the same under
    /// the instance gate instead of making the copy repeat a failed install
    /// first.
    pub async fn supersedable_once_continued(
        &self,
        root: &Path,
        instance: InstanceId,
    ) -> anyhow::Result<bool> {
        Ok(self.is_content_free()
            && self.copy_qualification(root, instance).await? == Some(true)
            && self.replaceable(root).await?)
    }

    /// Whether this is a fully installed pack (see
    /// [`pack_installed`](Self::pack_installed), checked against `instance`'s
    /// own `packinfo.json`) whose record is not its own (see
    /// [`binds_to`](Self::binds_to)): a duplicate copied it, it was written
    /// before the instance had an id, or the folder kept it from an earlier
    /// registration. Like the original's, only its
    /// game version install remains, so a version or loader change may retire
    /// it as is. Play would instead rebuild the pack from the copied archive
    /// and apply it again before installing the game version.
    pub async fn copied_pack_installed(
        &self,
        root: &Path,
        instance: InstanceId,
    ) -> anyhow::Result<bool> {
        Ok(self.binds_to(instance) && self.pack_installed(root).await?)
    }

    pub fn cancel_reason(&self, current: &info::Instance) -> Option<String> {
        const FIRST_INSTALL: &str = "This is the instance's first install, so there is no earlier version to go back to. Press Play to finish installing it.";
        match (self.producer, self.kind) {
            (Producer::Create | Producer::Import, Kind::Pack) => {
                return Some(FIRST_INSTALL.into());
            }
            // Also an unfinished version change: 2.0.40 did not record which
            // install an empty `.setup` belonged to. Only the game version is
            // pending, so there is nothing to cancel back to; choosing another
            // version replaces it instead (see `supersedable`). That is not
            // advertised for a modpack, whose files target its own version.
            (
                Producer::Create | Producer::Import | Producer::Reconstructed | Producer::Settings,
                Kind::VersionOnly | Kind::ImportedComplete,
            ) => {
                const UNFINISHED: &str = "This instance's game version or loader hasn't finished installing. Press Play to finish installing it";
                return Some(if current.modpack.is_some() {
                    format!("{UNFINISHED}.")
                } else {
                    format!("{UNFINISHED}, or choose another version with Edit.")
                });
            }
            _ => {}
        }
        // Its files may be part-way between the installed record and this
        // repair's target, so there is no installed version to go back to.
        if self.replaces_interrupted.is_some() {
            return Some(
                "This repair replaces an install or update that stopped while changing the modpack's files, so it can't be cancelled. Press Play to finish it, or use Repair again.".into(),
            );
        }
        if !matches!(self.producer, Producer::Change | Producer::Repair)
            || !matches!(self.phase, Phase::Reserved | Phase::Resolved)
        {
            return Some(
                "This setup may have changed installed content; resume it to finish safely.".into(),
            );
        }
        let installed = match schema::parse_instance_config(&self.installed_config) {
            Ok(v) => v,
            Err(_) => return Some("Installed metadata cannot be verified.".into()),
        };
        // A pack that never finished installing has no game version, and
        // cancelling back to it would leave an instance that cannot launch,
        // whichever operation started the install.
        if self.kind == Kind::Pack && installed.game_configuration.version.is_none() {
            return Some(FIRST_INSTALL.into());
        }
        if version_key(&current.game_configuration.version)
            != version_key(&installed.game_configuration.version)
        {
            return Some("The installed game version changed independently.".into());
        }
        if current
            .modpack
            .as_ref()
            .map(|p| &p.modpack)
            .is_some_and(|p| Some(p) != installed.modpack.as_ref().map(|p| &p.modpack))
        {
            return Some("The installed pack association changed independently.".into());
        }
        None
    }

    /// Whether this setup may have changed installed content without
    /// finishing, so the root `packinfo.json` (the installed record) may not
    /// describe what is on disk: it [changed files](Self::changed_files)
    /// itself, or it is a Repair that replaced such a setup and has not
    /// finished. A committed setup's pack (if any) is fully installed and
    /// recorded.
    pub fn interrupted(&self, root: &Path) -> bool {
        self.phase != Phase::Committed
            && (self.replaces_interrupted.is_some() || self.changed_files(root))
    }

    /// Whether this setup itself may have changed installed content without
    /// finishing: its apply started and never committed, an apply marker is
    /// beside it, or it reconstructs a 2.0.40 pack install that may have (see
    /// [`reconstructed_untouched`](Self::reconstructed_untouched)). A Repair
    /// that replaced such a setup but never reached apply itself changed
    /// nothing: its downloads are fresh and nothing reads them again.
    ///
    /// A marker that can't be checked counts as present.
    pub fn changed_files(&self, root: &Path) -> bool {
        if self.phase == Phase::Committed {
            return false;
        }
        let marked =
            |name: &str| !matches!(entry_metadata(&root.join(".setup").join(name)), Ok(None));
        let reconstructed = self.kind == Kind::Pack && self.producer == Producer::Reconstructed;
        matches!(self.phase, Phase::Applying | Phase::Applied)
            // This build marks an apply only once it saved Applying, so beside
            // a reconstructed setup that hasn't, the marker is 2.0.40's, which
            // stayed in place once its pack step finished.
            || marked("apply-started") && !(reconstructed && legacy_pack_step_finished(root))
            || marked("representation-transitions")
            || reconstructed && !self.reconstructed_untouched(root)
    }

    /// For a pack setup this build reconstructed from one 2.0.40 left:
    /// whether that setup provably left the installed files and their record
    /// as they were before this build's own apply. Either its pack step had
    /// finished (see [`legacy_pack_step_finished`]), or it never applied
    /// anything and its request didn't leave the configuration naming a pack
    /// the installed record doesn't describe (see
    /// [`staged_request_untouched`]). Builds before 2.0.40 marked a staged
    /// apply with `staging.json`.
    fn reconstructed_untouched(&self, root: &Path) -> bool {
        if legacy_pack_step_finished(root) {
            return true;
        }
        if !matches!(entry_metadata(&root.join(".setup/staging.json")), Ok(None)) {
            return false;
        }
        let Ok(installed) = schema::parse_instance_config(&self.installed_config) else {
            return false;
        };
        let Some(requested) = self
            .requested_pack
            .as_deref()
            .and_then(|text| serde_json::from_str::<super::PackVersionFile>(text).ok())
        else {
            return false;
        };
        staged_request_untouched(root, &installed, &requested.into())
    }

    /// The pack a Repair that replaces this setup installs when it is not the
    /// configured one: once this setup's apply started, the pack it applies
    /// (from its pending configuration), as 2.0.40's Repair did after writing
    /// that pack to the configuration; for a Repair that replaced an
    /// interrupted setup, the pack that repair installs.
    pub fn repair_target(&self) -> anyhow::Result<Option<info::Modpack>> {
        if matches!(
            self.phase,
            Phase::Applying | Phase::Applied | Phase::Committed
        ) {
            if let Some(pending) = &self.pending_config {
                if let Some(pack) = schema::parse_instance_config(pending)?.modpack {
                    return Ok(Some(pack.modpack));
                }
            }
        }
        if self.replaces_interrupted.is_some() {
            if let Some(requested) = &self.requested_pack {
                let requested: super::PackVersionFile = serde_json::from_str(requested)
                    .context("while parsing the repaired pack request")?;
                return Ok(Some(requested.into()));
            }
        }
        Ok(None)
    }
}

/// Whether 2.0.40 (or an earlier build) had finished a `.setup`'s pack step:
/// it wrote `modpack-complete` once the files were in place and their record
/// promoted, and left its `apply-started` beside it. This build never writes
/// that marker before its own setup committed. Builds before 2.0.40 marked a
/// staged apply with `staging.json`, which never counts as finished.
fn legacy_pack_step_finished(root: &Path) -> bool {
    let setup = root.join(".setup");
    matches!(entry_metadata(&setup.join("staging.json")), Ok(None))
        && matches!(entry_metadata(&setup.join("modpack-complete")), Ok(Some(_)))
}

/// Whether a 2.0.40 `.setup` that staged a `requested` pack, and wrote none
/// of the apply records that follow staging (see
/// [`unrecorded_interrupted`]), left an instance configured as `config` with
/// a configuration its installed record describes. 2.0.40 applied nothing
/// before those records, but once every file was downloaded it switched the
/// configuration to the requested pack: for an update, that names a pack
/// the installed files and record don't match. It never switched when the
/// configuration still names another pack. The switch changed nothing for a
/// Repair (the `repair` marker), which 2.0.40 ran on the configured pack, or
/// for a first install it hadn't reached: it gave a new install its game
/// version at the switch, and no record of installed files exists.
fn staged_request_untouched(
    root: &Path,
    config: &info::Instance,
    requested: &info::Modpack,
) -> bool {
    if config.modpack.as_ref().map(|pack| &pack.modpack) != Some(requested) {
        return true;
    }
    if entry_metadata(&root.join(".setup/repair"))
        .is_ok_and(|metadata| metadata.is_some_and(|metadata| is_plain_file(&metadata)))
    {
        return true;
    }
    config.game_configuration.version.is_none()
        && matches!(entry_metadata(&root.join("packinfo.json")), Ok(None))
}

/// [`Session::changed_files`] for a `.setup` without a readable record (one
/// 2.0.40 left, or one whose record is damaged), for an instance configured
/// as `config`.
///
/// 2.0.40 downloaded a pack into `.setup/staging` and extracted its
/// overrides there (`modpack-skip-overrides`), touching nothing live. Once
/// every file was downloaded it switched the configuration to the requested
/// pack, then wrote `staging-packinfo.json` and the root `tmp-packinfo.json`
/// (builds before it wrote `staging.json`), then `apply-started` right
/// before applying, and `modpack-complete` once the files were in place and
/// their record promoted. This build records its target in the root
/// `tmp-packinfo.json` before applying. So it may have changed installed
/// content when any of those apply records is left without
/// `modpack-complete`, or when something was staged and the configuration
/// may already name the requested pack where the installed record describes
/// another (see [`staged_request_untouched`]).
///
/// Anything that can't be checked (say, a `.setup` that is not a folder, or
/// a request that can't be read) counts as a marker.
pub fn unrecorded_interrupted(root: &Path, config: &info::Instance) -> bool {
    let marked = |path: &Path| !matches!(entry_metadata(path), Ok(None));
    let setup = root.join(".setup");
    // Windows reports paths below a file as missing rather than failing, so a
    // `.setup` that is not a folder is checked directly.
    if std::fs::metadata(&setup).is_ok_and(|metadata| !metadata.is_dir()) {
        return true;
    }
    // Applied and promoted: the installed record describes the files.
    if matches!(complete_without_staging(root), Ok(true)) {
        return false;
    }
    if marked(&root.join("tmp-packinfo.json"))
        || [
            "staging-packinfo.json",
            "staging.json",
            "apply-started",
            "representation-transitions",
        ]
        .iter()
        .any(|marker| marked(&setup.join(marker)))
    {
        return true;
    }
    // 2.0.40 made an empty staging folder for version-only prepares too.
    let staging = setup.join("staging");
    let staged = marked(&setup.join("modpack-skip-overrides"))
        || match entry_metadata(&staging) {
            Ok(None) => false,
            Ok(Some(metadata)) if is_plain_dir(&metadata) => {
                !holds_no_files(&staging).unwrap_or(false)
            }
            _ => true,
        };
    if !staged {
        return false;
    }
    let request = setup.join("change-pack-version.json");
    let requested = match entry_metadata(&request) {
        Ok(Some(metadata)) if is_plain_file(&metadata) => std::fs::read_to_string(&request)
            .ok()
            .and_then(|text| serde_json::from_str::<super::PackVersionFile>(&text).ok())
            .map(info::Modpack::from),
        _ => None,
    };
    requested.is_none_or(|requested| !staged_request_untouched(root, config, &requested))
}

/// Prefix of the folder a replacing request writes its setup into before it
/// takes `.setup`'s place (see [`begin_replacement`]).
const REPLACEMENT_PREFIX: &str = ".setup-next-";
/// Where a replacement keeps a root `tmp-packinfo.json` that no pending setup
/// claimed: evidence, never mistaken for the replacement's own target.
const STALE_TARGET: &str = "previous-tmp-packinfo.json";

/// Starts a replacement setup in a new folder beside `.setup`. The pending
/// setup (if any) is retired only once the replacement is fully written
/// there, its record last, and the two renames that swap them are adjacent
/// (see [`publish_replacement`]). A failure while writing it changes
/// nothing, and a crash between the renames leaves no `.setup` beside a
/// complete replacement, which [`settle_replacement`] publishes before
/// anything reads the setup again.
pub async fn begin_replacement(root: &Path) -> anyhow::Result<std::path::PathBuf> {
    let next = root.join(format!("{REPLACEMENT_PREFIX}{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir(&next).await?;
    Ok(next)
}

/// Moves a root `tmp-packinfo.json` that no pending setup claimed into the
/// replacement being written in `next` (see [`STALE_TARGET`]).
pub async fn keep_stale_target(root: &Path, next: &Path) -> anyhow::Result<()> {
    let temporary = root.join("tmp-packinfo.json");
    if entry_metadata(&temporary)?.is_some() {
        tokio::fs::rename(&temporary, next.join(STALE_TARGET)).await?;
    }
    Ok(())
}

/// Makes the replacement written in `next` the pending setup. `.setup` must
/// be gone already (retired, or never there).
pub async fn publish_replacement(root: &Path, next: &Path) -> anyhow::Result<()> {
    sync_dir(next).await?;
    let setup = root.join(".setup");
    if entry_metadata(&setup)?.is_some() {
        bail!("a setup is still pending where its replacement goes");
    }
    crate::util::publish::rename_dir(next, &setup).await?;
    sync_dir(root).await
}

/// Removes a replacement that was never published, after putting back the
/// root `tmp-packinfo.json` it took (see [`keep_stale_target`]). It holds
/// only what the replacing request wrote. Best effort.
pub async fn discard_replacement(root: &Path, next: &Path) {
    let temporary = root.join("tmp-packinfo.json");
    let kept = next.join(STALE_TARGET);
    let restored = match (entry_metadata(&kept), entry_metadata(&temporary)) {
        (Ok(Some(_)), Ok(None)) => tokio::fs::rename(&kept, &temporary)
            .await
            .map_err(anyhow::Error::from),
        (Ok(_), Ok(_)) => Ok(()),
        (Err(error), _) | (_, Err(error)) => Err(error.into()),
    };
    if let Err(error) = restored {
        tracing::warn!(
            ?error,
            "unclaimed temporary record left in the discarded replacement"
        );
    }
    if let Err(error) = tokio::fs::remove_dir_all(next).await {
        tracing::warn!(?error, path = %next.display(), "discarded setup replacement kept");
    }
}

/// Finishes or undoes a replacement that a crash interrupted (see
/// [`begin_replacement`]): with no `.setup`, a complete replacement (one with
/// its record) takes its place, as the pending setup it replaces was already
/// retired, or there was none. Any other replacement never took effect and
/// goes. Callers hold the instance gate, under which replacements are
/// written, so none is being written.
pub async fn settle_replacement(root: &Path) -> anyhow::Result<()> {
    let mut replacements = Vec::new();
    let mut entries = tokio::fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let named = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(REPLACEMENT_PREFIX));
        if named && entry_metadata(&path)?.is_some_and(|metadata| is_plain_dir(&metadata)) {
            replacements.push(path);
        }
    }
    replacements.sort();
    for next in replacements {
        let complete =
            entry_metadata(&next.join(RECORD))?.is_some_and(|metadata| is_plain_file(&metadata));
        if complete && entry_metadata(&root.join(".setup"))?.is_none() {
            tracing::warn!(path = %next.display(), "publishing a setup replacement a crash interrupted");
            publish_replacement(root, &next).await?;
        } else {
            discard_replacement(root, &next).await;
        }
    }
    Ok(())
}

/// A setup that a replacing request retired, kept as `.setup-cancelled-*`
/// (see [`retire`]) while its replacement takes its place.
pub struct Retired {
    pub path: std::path::PathBuf,
    /// Where the root `tmp-packinfo.json` went inside it, if there was one.
    moved_target: Option<String>,
}

/// Retires the pending setup for a replacing request (see [`retire`]), with
/// the root `tmp-packinfo.json` it left: that record is the retired setup's
/// target, never the replacing one's. The root `packinfo.json` stays: it
/// still records what the installed pack owns, which the replacement plans
/// against.
pub async fn retire_replaced(root: &Path, label: &str) -> anyhow::Result<Retired> {
    let temporary = root.join("tmp-packinfo.json");
    let mut moved_target = None;
    if entry_metadata(&temporary)?.is_some() {
        let setup = root.join(".setup");
        let mut name = "tmp-packinfo.json".to_owned();
        if entry_metadata(&setup.join(&name))?.is_some() {
            name = format!("tmp-packinfo-{}.json", uuid::Uuid::new_v4());
        }
        tokio::fs::rename(&temporary, setup.join(&name)).await?;
        moved_target = Some(name);
    }
    match retire(root, label).await {
        Ok(path) => Ok(Retired { path, moved_target }),
        Err(error) => {
            if let Some(name) = moved_target {
                if let Err(restore) =
                    tokio::fs::rename(root.join(".setup").join(name), &temporary).await
                {
                    tracing::warn!(?restore, "temporary record left in the pending setup");
                }
            }
            Err(error)
        }
    }
}

impl Retired {
    /// Puts the retired setup back when its replacement could not be
    /// published (see [`publish_replacement`]), so the instance is not left
    /// with no setup in place of one that may have changed its files. Never
    /// replaces a `.setup` that is there instead.
    pub async fn restore(self, root: &Path) -> anyhow::Result<()> {
        let setup = root.join(".setup");
        if entry_metadata(&setup)?.is_some() {
            bail!("another setup took the replaced one's place");
        }
        crate::util::publish::rename_dir(&self.path, &setup).await?;
        if let Some(name) = self.moved_target {
            let temporary = root.join("tmp-packinfo.json");
            if entry_metadata(&temporary)?.is_none() {
                tokio::fs::rename(setup.join(name), temporary).await?;
            }
        }
        sync_dir(root).await
    }
}

/// Removes the bulky payload of a retired setup that nothing reads again: the
/// downloaded pack archive and the pack files staged or being prepared from
/// it, which are launcher downloads, never user files. Its records and markers
/// stay as evidence until a later setup finishes (see [`prune_retired`]).
/// Call only for a setup that never changed installed content, or once a
/// later setup finished.
pub async fn trim_retired(retained: &Path) -> anyhow::Result<()> {
    let directory = retained.to_owned();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            let Some(metadata) = entry_metadata(&path)? else {
                continue;
            };
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            match name.as_str() {
                "curseforge" | "modrinth" | "gdlpack" if is_plain_file(&metadata) => {
                    std::fs::remove_file(&path)?;
                }
                _ if is_plain_dir(&metadata)
                    && (name == "staging"
                        || name.starts_with("preparing-")
                        || name.starts_with("prior-staging-")) =>
                {
                    std::fs::remove_dir_all(&path)?;
                }
                _ => {}
            }
        }
        Ok(())
    })
    .await??;
    sync_dir(retained).await
}

/// Prefix of a retired setup's folder (see [`retire`]).
const RETIRED_PREFIX: &str = ".setup-cancelled-";

/// Whether `name` is a top-level entry of an instance folder that earlier
/// setups left behind for that instance alone: a retired setup (see
/// [`retire`]) or the recovery copies Repair moved aside (see
/// `prune_settled_recovery`). Duplicating an instance leaves them with the
/// original.
pub fn setup_leftover(name: &std::ffi::OsStr) -> bool {
    let name = name.to_string_lossy();
    name == ".repair-recovery" || name.starts_with(RETIRED_PREFIX)
}

/// Removes every setup retired in `root` once a later setup finished:
/// whatever they held is settled, so nothing reads their records again, as
/// 2.0.40 removed a setup once its install finished. The recovery copies
/// they made and nothing needs go first (see `prune_settled_recovery`): their
/// records name them. One that can't be removed is trimmed instead (see
/// [`trim_retired`]), so it never holds a pack's worth of downloads for longer
/// than the next successful install. Best effort per directory.
pub async fn prune_retired(root: &Path) -> anyhow::Result<()> {
    let mut entries = tokio::fs::read_dir(root).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let retired = path
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(RETIRED_PREFIX));
        if !retired || !entry_metadata(&path)?.is_some_and(|metadata| is_plain_dir(&metadata)) {
            continue;
        }
        crate::managers::instance::run::modpack::prune_settled_recovery(root, &path).await;
        if let Err(error) = tokio::fs::remove_dir_all(&path).await {
            tracing::warn!(?error, path = %path.display(), "retired setup kept");
            if let Err(error) = trim_retired(&path).await {
                tracing::warn!(?error, path = %path.display(), "retired setup payload kept");
            }
        }
    }
    Ok(())
}

/// Removes `owner`'s setup once it committed and its game install
/// succeeded, with the recovery copies its apply made and nothing needs (see
/// `prune_settled_recovery`; its records name them, so they go first), then
/// every setup it replaced (see [`prune_retired`]): whatever they held is
/// settled. Callers hold the instance gate.
pub async fn dispose(root: &Path, owner: &Owner) -> anyhow::Result<()> {
    let session = Session::require(root, owner).await?;
    if session.phase != Phase::Committed {
        bail!("uncommitted setup cannot be disposed");
    }
    let setup = root.join(".setup");
    crate::managers::instance::run::modpack::prune_settled_recovery(root, &setup).await;
    tokio::fs::remove_dir_all(&setup).await?;
    if let Err(error) = prune_retired(root).await {
        tracing::warn!(?error, "retired setups kept");
    }
    Ok(())
}

/// Retires a pending setup without deleting anything: `.setup` becomes
/// `.setup-cancelled-<label>-<uuid>`, so a retire never collides with an
/// earlier one. `label` is the retired operation, or `legacy`.
pub async fn retire(root: &Path, label: &str) -> anyhow::Result<std::path::PathBuf> {
    let retained = root.join(format!("{RETIRED_PREFIX}{label}-{}", uuid::Uuid::new_v4()));
    crate::util::publish::rename_dir(&root.join(".setup"), &retained).await?;
    sync_dir(root).await?;
    Ok(retained)
}

/// Removes the pack archive from a retired setup whose pack was fully
/// installed (see [`Session::pack_installed`]). Nothing reads it again: the
/// pack's files are in place and the root `packinfo.json` records them, and a
/// successful game install would have removed it with the rest of `.setup`.
/// Everything else in `retained` stays.
pub async fn drop_installed_pack_archive(retained: &Path) -> anyhow::Result<()> {
    for name in ["curseforge", "modrinth", "gdlpack"] {
        let archive = retained.join(name);
        if entry_metadata(&archive)?.is_some_and(|metadata| is_plain_file(&metadata)) {
            tokio::fs::remove_file(&archive).await?;
        }
    }
    sync_dir(retained).await
}

/// What a `.setup` without a session record asked for, when it provably
/// never reached pack apply. See [`classify_legacy`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyRequest {
    /// An empty `.setup`: a create or a version/loader change whose prepare
    /// never finished. 2.0.40 ran it as a plain version install.
    VersionOnly,
    /// A `.setup` holding `change-pack-version.json` (plus `repair` after a
    /// Repair): a pack install, update or repair that stopped before its pack
    /// archive finished downloading.
    Pack {
        requested_pack: String,
        options: Option<RepairMarkerFile>,
        /// A root packinfo.json records an installed pack.
        installed_record: bool,
    },
    /// A clean `modpack-complete` marker (plus the import's
    /// `change-pack-version.json`): a 2.0.40 import that was never launched,
    /// or a version-only install that failed after its pack step, at the
    /// Java, Minecraft or loader install.
    Complete { requested_pack: Option<String> },
}

impl LegacyRequest {
    /// Whether this is a pack request over an installed pack, which
    /// cancelling goes back to. 2.0.40 writes a pack's game version once its
    /// archive is processed (importers know it up front), so a version, like
    /// a packinfo.json, proves an installed pack.
    pub fn over_installed_pack(&self, config: &info::Instance) -> bool {
        matches!(self, Self::Pack { installed_record, .. }
            if *installed_record || config.game_configuration.version.is_some())
    }
}

/// Whether `.setup` holds an original pack archive. Play reconstructs a
/// sessionless setup from it.
pub fn has_original_archive(setup: &Path) -> bool {
    ["curseforge", "modrinth", "gdlpack"]
        .iter()
        .any(|name| setup.join(name).is_file())
}

/// Whether a sessionless `.setup` carries 2.0.40's modpack-complete marker
/// with no staging and no root `tmp-packinfo.json` left: Play adopts it as a
/// completed install even beside other leftovers, which [`classify_legacy`]
/// does not accept.
pub fn complete_without_staging(root: &Path) -> std::io::Result<bool> {
    let setup = root.join(".setup");
    Ok(setup.join("modpack-complete").exists()
        && !setup.join("staging").try_exists()?
        && !root.join("tmp-packinfo.json").try_exists()?)
}

/// Whether Play resumes a sessionless `.setup`, even one that
/// [`classify_legacy`] does not adopt: from its original archive, or as a
/// completed install. Every other leftover may hold partly applied files.
pub fn legacy_resumed_by_play(root: &Path) -> std::io::Result<bool> {
    Ok(has_original_archive(&root.join(".setup")) || complete_without_staging(root)?)
}

/// Whether a `.setup` without a session record is a pack install, update or
/// repair that 2.0.40 (or an earlier build) finished applying, so only its
/// game version install remains: a modpack-complete marker beside exactly one
/// original archive, with no staging and no root `tmp-packinfo.json`, and the
/// root `packinfo.json` that apply promoted. Every build writes that marker
/// for a pack only once its files are in place (with staged applies, once
/// staging is gone and `tmp-packinfo.json` is promoted), and 2.0.40 skipped
/// the pack step beside it.
///
/// Play adopts such a setup as the committed install it is (see
/// [`Session::adopt_legacy_applied_pack`]) and only installs the game
/// version, as 2.0.40 did; a version or loader change retires it instead, as
/// 2.0.40 installed the new choice straight away. Besides what 2.0.40's apply
/// leaves, only `preparing-*` folders are allowed: earlier builds of this
/// launcher rebuilt such a setup at Play, and a rebuild writes nothing else,
/// and touches nothing outside them, before it records a session. Read-only.
pub async fn legacy_pack_applied(root: &Path) -> anyhow::Result<bool> {
    let root = root.to_owned();
    tokio::task::spawn_blocking(move || legacy_pack_applied_blocking(&root)).await?
}

fn legacy_pack_applied_blocking(root: &Path) -> anyhow::Result<bool> {
    let setup = root.join(".setup");
    if !entry_metadata(&setup)?.is_some_and(|metadata| is_plain_dir(&metadata))
        || entry_metadata(&root.join("tmp-packinfo.json"))?.is_some()
    {
        return Ok(false);
    }
    let promoted = root.join("packinfo.json");
    if !entry_metadata(&promoted)?.is_some_and(|metadata| is_plain_file(&metadata))
        || packinfo::parse_packinfo(&std::fs::read_to_string(&promoted)?).is_err()
    {
        return Ok(false);
    }
    let mut archives = 0;
    let mut complete = false;
    for entry in std::fs::read_dir(&setup)? {
        let path = entry?.path();
        let Some(metadata) = entry_metadata(&path)? else {
            continue;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folded = name.to_ascii_lowercase();
        match name.as_str() {
            "curseforge" | "modrinth" | "gdlpack" if is_plain_file(&metadata) => {
                archives += 1;
                continue;
            }
            // A file after a staged apply, a folder from older builds.
            "modpack-complete" if is_plain_file(&metadata) || is_plain_dir(&metadata) => {
                complete = true;
                continue;
            }
            // What 2.0.40's pack request and apply leave beside it.
            "change-pack-version.json" | "repair" | "staging-packinfo.json" | "apply-started"
                if is_plain_file(&metadata) =>
            {
                continue;
            }
            "modpack-skip-overrides" if is_plain_dir(&metadata) => continue,
            _ if name.starts_with("preparing-") && is_plain_dir(&metadata) => continue,
            _ => {}
        }
        if ["change-pack-version.json", "repair", "staging"].contains(&folded.as_str())
            || LEGACY_APPLY_EVIDENCE
                .iter()
                .any(|evidence| evidence.eq_ignore_ascii_case(&folded))
            || LEGACY_APPLY_EVIDENCE_PREFIXES
                .iter()
                .any(|prefix| folded.starts_with(prefix))
        {
            return Ok(false);
        }
    }
    Ok(complete && archives == 1)
}

/// Entries of a sessionless `.setup` that show a pack archive arrived or that
/// apply may have started, so the setup is not provably untouched. Compared
/// case-insensitively. Original archives keep their own reconstruction path.
/// A plain modpack-complete marker is accepted on its own, see
/// [`LegacyRequest::Complete`].
const LEGACY_APPLY_EVIDENCE: [&str; 11] = [
    "curseforge",
    "modrinth",
    "gdlpack",
    "modpack-complete",
    "staging-packinfo.json",
    "modpack-skip-overrides",
    "apply-started",
    "representation-transitions",
    "previous-tmp-packinfo.json",
    "cancelled-target.json",
    RECORD,
];
const LEGACY_APPLY_EVIDENCE_PREFIXES: [&str; 2] = ["preparing-", "prior-"];

fn entry_metadata(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn is_plain_file(metadata: &std::fs::Metadata) -> bool {
    metadata.is_file() && !metadata.file_type().is_symlink()
}

fn is_plain_dir(metadata: &std::fs::Metadata) -> bool {
    metadata.is_dir() && !metadata.file_type().is_symlink()
}

/// True when `dir` holds nothing but (nested) plain directories. 2.0.40 made
/// an empty `staging/instance` for version-only prepares before removing it.
fn holds_no_files(dir: &Path) -> std::io::Result<bool> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let Some(metadata) = entry_metadata(&path)? else {
            continue;
        };
        if !is_plain_dir(&metadata) || !holds_no_files(&path)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Classifies a `.setup` that has no session record, without writing. Only a
/// plain `.setup` with no apply evidence qualifies: no original archive, no
/// staging holding files, no staging/apply/transition markers, no
/// `preparing-*` or `prior-*` work of the current build, and no root
/// `tmp-packinfo.json`. 2.0.40 downloads a pack archive to a temporary file,
/// moves it into `.setup` only once complete and never deletes it before
/// `.setup` itself; it extracts, touches live files and rewrites
/// instance.json only after that. So a setup without any of these has not
/// touched the instance, and its current configuration is still the
/// installed one. Other entries (for example file-manager metadata) are not
/// evidence of anything.
///
/// A plain `modpack-complete` marker with none of that evidence, no staging
/// and no `repair` is the other shape 2.0.40 leaves: its pack step had
/// finished (or an importer copied the files in), so only the game version
/// install remains. 2.0.40 writes that marker for version-only prepares only
/// after removing staging, and keeps a pack's archive beside it after apply.
pub async fn classify_legacy(root: &Path) -> anyhow::Result<Option<LegacyRequest>> {
    let root = root.to_owned();
    tokio::task::spawn_blocking(move || classify_setup_blocking(&root, false)).await?
}

/// The same proof for a `.setup` whose record was copied from another
/// instance: true when nothing beside the record shows that a pack archive
/// arrived or that apply may have started.
pub async fn untouched_beside_record(root: &Path) -> anyhow::Result<bool> {
    let root = root.to_owned();
    Ok(matches!(
        tokio::task::spawn_blocking(move || classify_setup_blocking(&root, true)).await??,
        Some(LegacyRequest::VersionOnly | LegacyRequest::Pack { .. })
    ))
}

fn classify_setup_blocking(
    root: &Path,
    beside_record: bool,
) -> anyhow::Result<Option<LegacyRequest>> {
    let setup = root.join(".setup");
    match entry_metadata(&setup)? {
        Some(metadata) if is_plain_dir(&metadata) => {}
        _ => return Ok(None),
    }
    if entry_metadata(&root.join("tmp-packinfo.json"))?.is_some() {
        return Ok(None);
    }
    let mut requested_pack = None;
    let mut repair = None;
    let mut staging = false;
    let mut complete = false;
    for entry in std::fs::read_dir(&setup)? {
        let path = entry?.path();
        let Some(metadata) = entry_metadata(&path)? else {
            continue;
        };
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folded = name.to_ascii_lowercase();
        if beside_record && name == RECORD && is_plain_file(&metadata) {
            continue;
        }
        match name.as_str() {
            "change-pack-version.json" if is_plain_file(&metadata) => {
                requested_pack = Some(path);
                continue;
            }
            "repair" if is_plain_file(&metadata) => {
                repair = Some(path);
                continue;
            }
            "staging" if is_plain_dir(&metadata) && holds_no_files(&path)? => {
                staging = true;
                continue;
            }
            // A file after a version-only prepare, an empty directory from an
            // importer.
            "modpack-complete"
                if (is_plain_file(&metadata) || is_plain_dir(&metadata)) && !beside_record =>
            {
                complete = true;
                continue;
            }
            _ => {}
        }
        // A non-plain or differently spelled request, repair or staging entry
        // is ambiguous, like every apply marker.
        if ["change-pack-version.json", "repair", "staging"].contains(&folded.as_str())
            || LEGACY_APPLY_EVIDENCE
                .iter()
                .any(|evidence| evidence.eq_ignore_ascii_case(&folded))
            || LEGACY_APPLY_EVIDENCE_PREFIXES
                .iter()
                .any(|prefix| folded.starts_with(prefix))
        {
            return Ok(None);
        }
    }
    if complete {
        // 2.0.40 removes staging before writing the marker, and a repair
        // always downloads an archive first.
        if staging || repair.is_some() {
            return Ok(None);
        }
        return Ok(Some(LegacyRequest::Complete {
            requested_pack: requested_pack.map(std::fs::read_to_string).transpose()?,
        }));
    }
    Ok(match (requested_pack, repair) {
        (None, None) => Some(LegacyRequest::VersionOnly),
        // 2.0.40 writes `repair` and `change-pack-version.json` together.
        (None, Some(_)) => None,
        (Some(requested), repair) => {
            let options = repair
                .map(|path| -> anyhow::Result<RepairMarkerFile> {
                    serde_json::from_str(&std::fs::read_to_string(path)?)
                        .context("while parsing repair marker json")
                })
                .transpose()?;
            Some(LegacyRequest::Pack {
                requested_pack: std::fs::read_to_string(requested)?,
                options,
                installed_record: entry_metadata(&root.join("packinfo.json"))?.is_some(),
            })
        }
    })
}

/// What a stat of the record shows. Every writer replaces the record with a
/// new file (`atomic_json`). On Unix that new file's identity (device and
/// inode) always differs, so any rewrite changes the stamp. Elsewhere only
/// length and times are compared, which is best effort: a rewrite of the same
/// length within one timestamp tick (and NTFS hands the replaced file's
/// creation time to the new one) looks unchanged. The instance gate, which
/// every core writer takes, is the real protection; the stamp only catches
/// changes made from outside the app.
#[derive(PartialEq, Eq)]
struct RecordStamp {
    len: u64,
    modified: Option<std::time::SystemTime>,
    created: Option<std::time::SystemTime>,
    #[cfg(unix)]
    file: (u64, u64),
}

async fn record_stamp(root: &Path) -> Option<RecordStamp> {
    let metadata = tokio::fs::symlink_metadata(root.join(".setup").join(RECORD))
        .await
        .ok()?;
    Some(RecordStamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        created: metadata.created().ok(),
        #[cfg(unix)]
        file: {
            use std::os::unix::fs::MetadataExt;
            (metadata.dev(), metadata.ino())
        },
    })
}

/// Checks a setup's ownership in full once, then again only when the record
/// changes on disk. Loading a session parses and re-hashes the whole target,
/// so loading it before every file of an n-file apply cost O(n²). The caller
/// holds the instance gate, which every core writer of the record also takes;
/// the stat below only has to catch a change made from outside the app, and
/// such a change still gets the full check before the next mutation.
pub struct OwnershipWatch<'a> {
    root: &'a Path,
    owner: &'a Owner,
    stamp: Option<RecordStamp>,
}

impl<'a> OwnershipWatch<'a> {
    pub async fn begin(root: &'a Path, owner: &'a Owner) -> anyhow::Result<Self> {
        let mut watch = Self {
            root,
            owner,
            stamp: None,
        };
        watch.revalidate().await?;
        Ok(watch)
    }

    pub async fn check(&mut self) -> anyhow::Result<()> {
        if self.stamp.is_some() && record_stamp(self.root).await == self.stamp {
            return Ok(());
        }
        self.revalidate().await
    }

    async fn revalidate(&mut self) -> anyhow::Result<()> {
        // Stamp before loading: a rewrite racing the load lands after this
        // stamp and is caught by the next check.
        let stamp = record_stamp(self.root).await;
        Session::require(self.root, self.owner).await?;
        self.stamp = stamp;
        Ok(())
    }
}

/// Full session loads per instance root, so parallel tests stay apart.
#[cfg(test)]
pub(crate) mod load_count {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{LazyLock, Mutex},
    };
    static LOADS: LazyLock<Mutex<HashMap<PathBuf, usize>>> = LazyLock::new(Default::default);
    pub(super) fn record(root: &Path) {
        *LOADS.lock().unwrap().entry(root.to_owned()).or_default() += 1;
    }
    pub(crate) fn of(root: &Path) -> usize {
        LOADS.lock().unwrap().get(root).copied().unwrap_or(0)
    }
}

fn version_key(version: &Option<info::GameVersion>) -> String {
    match version {
        Some(info::GameVersion::Standard(v)) => {
            let mut loaders: Vec<_> = v
                .modloaders
                .iter()
                .map(|l| format!("{:?}:{}", l.type_, l.version))
                .collect();
            loaders.sort();
            format!("standard:{}:{loaders:?}", v.release)
        }
        Some(info::GameVersion::Custom(v)) => format!("custom:{v}"),
        None => "none".into(),
    }
}

/// Produced only while the stable instance gate is held. The reserved worker
/// receives the same RuntimeActivity that admitted this producer.
pub struct Request {
    pub producer: Producer,
    pub kind: Kind,
    pub files: Vec<(String, Vec<u8>)>,
    pub options: Option<RepairMarkerFile>,
    pub reset: bool,
    /// The pending setup a Repair replaces, identified under the same gate.
    pub replaces: Option<Replaced>,
    /// What the producer already knows the user should hear once the setup
    /// succeeds, such as files a version change keeps because it could not
    /// check them (see `imported_baseline::Baseline::warning`). Recorded with
    /// the setup, so whichever task finishes it reports them, once.
    pub warnings: Vec<crate::domain::vtask::TaskWarning>,
}

/// A pending setup that a Repair replaces, whatever it is: retired under
/// `label` (its operation, or `legacy` without a readable record).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replaced {
    pub label: String,
    /// It may have changed installed content, or continues a setup that may
    /// have (see [`Session::interrupted`]), so the repair continues it (see
    /// [`Session::replaces_interrupted`]).
    pub interrupted: bool,
    /// Its retired payload (its archive and the downloads staged or being
    /// prepared from it) stays until a later setup finishes, when the whole
    /// retired setup goes (see [`prune_retired`]): it may have changed
    /// installed content itself (see [`Session::changed_files`]), and it is
    /// not a Repair. Otherwise only its records stay until then: its
    /// downloads never touched the instance, or they are a Repair's fresh
    /// downloads. Nothing reads either again (the replacing Repair downloads
    /// its own), and what a Repair's apply displaced is kept in
    /// `.repair-recovery`, never in its setup. So only the first interrupted
    /// setup a chain of failed repairs replaced keeps a payload.
    pub keep_payload: bool,
}

#[cfg(test)]
pub(crate) mod publication_fixture {
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{Arc, Condvar, LazyLock, Mutex},
        time::Duration,
    };
    static PAUSES: LazyLock<Mutex<HashMap<PathBuf, Arc<Pause>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    pub(crate) struct Pause {
        pub entered: tokio::sync::Notify,
        pub finished: tokio::sync::Notify,
        released: Mutex<bool>,
        cv: Condvar,
    }
    impl Pause {
        pub(crate) fn release(&self) {
            *self.released.lock().unwrap() = true;
            self.cv.notify_all();
        }
    }
    pub(crate) fn arm(path: PathBuf) -> Arc<Pause> {
        let pause = Arc::new(Pause {
            entered: Default::default(),
            finished: Default::default(),
            released: Mutex::new(false),
            cv: Condvar::new(),
        });
        PAUSES.lock().unwrap().insert(path, pause.clone());
        pause
    }
    /// Pauses the next write of `name` into a replacement setup of `root`,
    /// whose folder name is random (see [`super::begin_replacement`]).
    pub(crate) fn arm_replacement(root: &Path, name: &str) -> Arc<Pause> {
        arm(root.join(super::REPLACEMENT_PREFIX).join(name))
    }
    pub(super) fn wait(path: &Path) -> Option<Arc<Pause>> {
        let replacement = path
            .parent()
            .filter(|folder| {
                folder.file_name().is_some_and(|name| {
                    name.to_string_lossy()
                        .starts_with(super::REPLACEMENT_PREFIX)
                })
            })
            .and_then(|folder| {
                Some(
                    folder
                        .parent()?
                        .join(super::REPLACEMENT_PREFIX)
                        .join(path.file_name()?),
                )
            });
        let pause = {
            let mut pauses = PAUSES.lock().unwrap();
            pauses.remove(path).or_else(|| pauses.remove(&replacement?))
        }?;
        pause.entered.notify_one();
        let result = pause
            .cv
            .wait_timeout_while(
                pause.released.lock().unwrap(),
                Duration::from_secs(10),
                |released| !*released,
            )
            .unwrap();
        assert!(*result.0, "bounded session publication fixture timed out");
        drop(result);
        Some(pause)
    }
}

#[cfg(test)]
mod task33_shader_recovery_tests {
    use super::*;
    use crate::{domain::vtask::VisualTaskId, managers::vtask::TaskOutcome};

    fn config() -> info::Instance {
        schema::parse_instance_config(r#"{"_version":"1","name":"Synthetic","game_configuration":{"version":{"release":"1.20.1","modloaders":[{"type":"Fabric","version":"0.15.11"}]}}}"#).unwrap()
    }
    fn bound() -> (Session, info::Instance) {
        let config = config();
        let mut session =
            Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap();
        session.instance_id = Some(7);
        session
            .bind_preparation("core-A", VisualTaskId(11), &config)
            .unwrap();
        (session, config)
    }
    fn validate(session: &Session, config: &info::Instance) -> anyhow::Result<()> {
        session.validate_shader_recovery(
            config,
            InstanceId(7),
            VisualTaskId(11),
            "1.20.1",
            "core-A",
            Some(VisualTaskId(11)),
            &TaskOutcome::Dismissed,
        )
    }
    #[test]
    fn task33_exact_owner_and_repeated_rebinding_reject_old_task() {
        let (mut session, config) = bound();
        validate(&session, &config).unwrap();
        session
            .bind_preparation("core-A", VisualTaskId(21), &config)
            .unwrap();
        assert!(validate(&session, &config).is_err());
        session
            .validate_shader_recovery(
                &config,
                InstanceId(7),
                VisualTaskId(21),
                "1.20.1",
                "core-A",
                Some(VisualTaskId(21)),
                &TaskOutcome::Dismissed,
            )
            .unwrap();
    }
    #[test]
    fn task33_foreign_replaced_copied_and_unbound_sessions_refused() {
        let (session, config) = bound();
        for altered in 0..5 {
            let mut other = session.clone();
            match altered {
                0 => other.instance_id = Some(8),
                1 => other.operation = uuid::Uuid::new_v4().to_string(),
                2 => other.copied_to = Some(8),
                3 => {
                    other.adopted_from = Some(Owner {
                        instance_id: 8,
                        operation: uuid::Uuid::new_v4().to_string(),
                    })
                }
                _ => other.accepted_preparation = None,
            }
            assert!(validate(&other, &config).is_err(), "case {altered}");
        }
    }
    #[test]
    fn task33_non_settings_non_version_only_and_non_fabric_refused() {
        let (session, config) = bound();
        let mut other = session.clone();
        other.producer = Producer::Repair;
        assert!(validate(&other, &config).is_err());
        let mut other = session.clone();
        other.kind = Kind::Pack;
        assert!(validate(&other, &config).is_err());
        let mut vanilla = config.clone();
        if let Some(info::GameVersion::Standard(version)) = &mut vanilla.game_configuration.version
        {
            version.modloaders.clear();
        }
        let mut other = session.clone();
        other.installed_config = schema::make_instance_config(vanilla).unwrap();
        assert!(validate(&other, &config).is_err());
    }
    #[test]
    fn task33_changed_game_loader_and_expected_mc_refused() {
        let (session, config) = bound();
        for change in 0..2 {
            let mut current = config.clone();
            if let Some(info::GameVersion::Standard(version)) =
                &mut current.game_configuration.version
            {
                if change == 0 {
                    version.release = "1.21".into();
                } else {
                    version.modloaders = [info::ModLoader {
                        type_: info::ModLoaderType::Fabric,
                        version: "0.16.0".into(),
                    }]
                    .into();
                }
            }
            assert!(validate(&session, &current).is_err());
        }
        assert!(
            session
                .validate_shader_recovery(
                    &config,
                    InstanceId(7),
                    VisualTaskId(11),
                    "1.21",
                    "core-A",
                    Some(VisualTaskId(11)),
                    &TaskOutcome::Dismissed
                )
                .is_err()
        );
    }
    #[test]
    fn task33_generation_mismatch_stale_failed_owner_and_unavailable_refused() {
        let (session, config) = bound();
        assert!(
            session
                .validate_shader_recovery(
                    &config,
                    InstanceId(7),
                    VisualTaskId(11),
                    "1.20.1",
                    "core-B",
                    Some(VisualTaskId(11)),
                    &TaskOutcome::Dismissed
                )
                .is_err()
        );
        for failed in [None, Some(VisualTaskId(12))] {
            assert!(
                session
                    .validate_shader_recovery(
                        &config,
                        InstanceId(7),
                        VisualTaskId(11),
                        "1.20.1",
                        "core-A",
                        failed,
                        &TaskOutcome::Dismissed
                    )
                    .is_err()
            );
        }
        for outcome in [TaskOutcome::Unavailable, TaskOutcome::Succeeded] {
            assert!(
                session
                    .validate_shader_recovery(
                        &config,
                        InstanceId(7),
                        VisualTaskId(11),
                        "1.20.1",
                        "core-A",
                        Some(VisualTaskId(11)),
                        &outcome
                    )
                    .is_err()
            );
        }
        assert_ne!(
            crate::managers::vtask::VisualTaskManager::new().generation(),
            crate::managers::vtask::VisualTaskManager::new().generation()
        );
    }
    #[test]
    fn task33_failed_outcome_has_exact_task_identity() {
        let (session, config) = bound();
        let task = crate::domain::vtask::Task {
            id: VisualTaskId(11),
            name: crate::api::translation::Translation::Test,
            progress: crate::domain::vtask::Progress::Failed(std::sync::Arc::new(anyhow::anyhow!(
                "synthetic"
            ))),
            downloaded: 0,
            download_total: 0,
            active_subtasks: vec![],
        };
        session
            .validate_shader_recovery(
                &config,
                InstanceId(7),
                VisualTaskId(11),
                "1.20.1",
                "core-A",
                Some(VisualTaskId(11)),
                &TaskOutcome::Failed(task),
            )
            .unwrap();
    }
    #[test]
    fn task33_running_or_foreign_failed_outcome_cannot_authorize_recovery() {
        let (session, config) = bound();
        let task = |id, progress| crate::domain::vtask::Task {
            id: VisualTaskId(id),
            name: crate::api::translation::Translation::Test,
            progress,
            downloaded: 0,
            download_total: 0,
            active_subtasks: vec![],
        };
        for outcome in [
            TaskOutcome::Running(task(11, crate::domain::vtask::Progress::Known(1.0))),
            TaskOutcome::Failed(task(
                12,
                crate::domain::vtask::Progress::Failed(std::sync::Arc::new(anyhow::anyhow!(
                    "foreign synthetic failure"
                ))),
            )),
        ] {
            assert!(
                session
                    .validate_shader_recovery(
                        &config,
                        InstanceId(7),
                        VisualTaskId(11),
                        "1.20.1",
                        "core-A",
                        Some(VisualTaskId(11)),
                        &outcome
                    )
                    .is_err()
            );
        }
    }
    #[tokio::test]
    async fn task33_later_reload_rejects_a_valid_replacement_and_rebound_authority() {
        let (expected, config) = bound();
        let root = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(root.path().join(".setup"))
            .await
            .unwrap();
        expected.create(root.path()).await.unwrap();
        let captured = Session::load(root.path()).await.unwrap().unwrap();
        validate(&captured, &config).unwrap();
        Session::require_preparation(root.path(), &captured)
            .await
            .unwrap();
        // A completely valid same-instance replacement passes its own initial
        // admission proof, but cannot be adopted by this captured continuation.
        let (replacement, _) = bound();
        validate(&replacement, &config).unwrap();
        atomic_json(
            &root.path().join(".setup").join(RECORD),
            &replacement,
            false,
        )
        .await
        .unwrap();
        assert!(
            Session::require_preparation(root.path(), &captured)
                .await
                .is_err()
        );
        let mut rebound = captured.clone();
        rebound
            .bind_preparation("core-A", VisualTaskId(21), &config)
            .unwrap();
        atomic_json(&root.path().join(".setup").join(RECORD), &rebound, false)
            .await
            .unwrap();
        assert!(
            Session::require_preparation(root.path(), &captured)
                .await
                .is_err()
        );
        atomic_json(&root.path().join(".setup").join(RECORD), &captured, false)
            .await
            .unwrap();
        Session::require_preparation(root.path(), &captured)
            .await
            .unwrap();
    }
    #[tokio::test]
    async fn task33_actual_session_publication_keeps_binding_and_old_record_default() {
        let (mut session, config) = bound();
        let root = tempfile::tempdir().unwrap();
        tokio::fs::create_dir(root.path().join(".setup"))
            .await
            .unwrap();
        session.create(root.path()).await.unwrap();
        let loaded = Session::load(root.path()).await.unwrap().unwrap();
        validate(&loaded, &config).unwrap();
        let owner = session.owner().unwrap();
        session
            .bind_preparation("core-A", VisualTaskId(21), &config)
            .unwrap();
        session.save(root.path(), &owner).await.unwrap();
        let loaded = Session::load(root.path()).await.unwrap().unwrap();
        assert_eq!(loaded.accepted_preparation.unwrap().task_id, 21);
        let mut old = serde_json::to_value(&session).unwrap();
        old.as_object_mut().unwrap().remove("accepted_preparation");
        let old: Session = serde_json::from_value(old).unwrap();
        assert!(old.accepted_preparation.is_none());
        assert!(validate(&old, &config).is_err());
        assert!(
            Session::load(&root.path().join("missing"))
                .await
                .unwrap()
                .is_none()
        );
    }
}

/// macOS data folders on exFAT, FAT, NTFS or network shares have no
/// no-replace rename: setup admission still publishes there, and still never
/// replaces another setup's record.
#[cfg(all(test, unix))]
mod no_replace_volume_tests {
    use super::*;
    use crate::util::publish::fault::{ENOTSUP, Step, inject, pending};

    fn session() -> Session {
        let config = schema::parse_instance_config(r#"{"_version":"1","name":"Synthetic","game_configuration":{"version":{"release":"1.20.1","modloaders":[]}}}"#).unwrap();
        Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap()
    }

    #[tokio::test]
    async fn create_publishes_once_without_no_replace_rename() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".setup")).unwrap();
        let record = root.path().join(".setup").join(RECORD);
        let first = session();
        inject(Step::NoReplace, &record, ENOTSUP);
        first.create(root.path()).await.unwrap();
        assert!(!pending(Step::NoReplace, &record));
        let saved = std::fs::read(&record).unwrap();
        assert_eq!(
            Session::load(root.path()).await.unwrap().unwrap().operation,
            first.operation
        );

        inject(Step::NoReplace, &record, ENOTSUP);
        let error = session().create(root.path()).await.unwrap_err();
        assert!(!pending(Step::NoReplace, &record));
        assert_eq!(
            error.downcast_ref::<std::io::Error>().map(|e| e.kind()),
            Some(std::io::ErrorKind::AlreadyExists)
        );
        assert_eq!(std::fs::read(&record).unwrap(), saved);
        let names: Vec<_> = std::fs::read_dir(root.path().join(".setup"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, [std::ffi::OsString::from(RECORD)]);
    }
}

/// An instance folder moved to another drive and linked back: records in it
/// are written through the link, while a link in place of a folder the setup
/// made is still refused.
#[cfg(all(test, unix))]
mod linked_root_tests {
    use super::*;

    #[tokio::test]
    async fn root_records_are_written_through_a_linked_instance_folder() {
        let drive = tempfile::tempdir().unwrap();
        let moved = drive.path().join("Pack");
        std::fs::create_dir_all(moved.join(".setup")).unwrap();
        let instances = tempfile::tempdir().unwrap();
        let root = instances.path().join("Pack");
        std::os::unix::fs::symlink(&moved, &root).unwrap();

        atomic_bytes_in_root(&root.join("tmp-packinfo.json"), b"first".to_vec(), false)
            .await
            .unwrap();
        atomic_bytes_in_root(&root.join("tmp-packinfo.json"), b"target".to_vec(), false)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(moved.join("tmp-packinfo.json")).unwrap(),
            b"target"
        );
        // The setup's own folder inside it is a plain folder, as before.
        atomic_bytes(&root.join(".setup").join(RECORD), b"record".to_vec(), true)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(moved.join(".setup").join(RECORD)).unwrap(),
            b"record"
        );
        assert!(
            std::fs::symlink_metadata(&root)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        // A link in place of a folder the setup made stays refused.
        let elsewhere = drive.path().join("elsewhere");
        std::fs::create_dir(&elsewhere).unwrap();
        std::os::unix::fs::symlink(&elsewhere, moved.join("staging")).unwrap();
        let error = atomic_bytes(&moved.join("staging/record"), b"x".to_vec(), true)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("plain directory"),
            "{error:#}"
        );
        assert!(!elsewhere.join("record").exists());
        // A record never lands where the instance folder link no longer leads.
        std::fs::remove_dir_all(&moved).unwrap();
        let error = atomic_bytes_in_root(&root.join("tmp-packinfo.json"), b"x".to_vec(), false)
            .await
            .unwrap_err();
        assert!(
            format!("{error:#}").contains("could not open the instance folder"),
            "{error:#}"
        );
    }
}

#[cfg(test)]
mod legacy_setup_tests {
    use super::*;

    fn config() -> info::Instance {
        schema::parse_instance_config(r#"{"_version":"1","name":"Legacy","game_configuration":{"version":{"release":"1.20.1","modloaders":[{"type":"Fabric","version":"0.15.11"}]}}}"#).unwrap()
    }

    const PACK: &str = r#"{"type":"modrinth","project_id":"synthetic","version_id":"B"}"#;

    fn tree(root: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
        walkdir::WalkDir::new(root)
            .into_iter()
            .map(|entry| entry.unwrap())
            .map(|entry| {
                let relative = entry
                    .path()
                    .strip_prefix(root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                let bytes = entry
                    .file_type()
                    .is_file()
                    .then(|| std::fs::read(entry.path()).unwrap());
                (relative, bytes)
            })
            .collect()
    }

    fn legacy_root(files: &[(&str, &[u8])], dirs: &[&str]) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join(".setup")).unwrap();
        std::fs::create_dir(root.path().join("instance")).unwrap();
        for dir in dirs {
            std::fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        for (path, bytes) in files {
            let path = root.path().join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        root
    }

    fn first_install_config() -> info::Instance {
        // A 2.0.40 Browse install: the game version is only known once the
        // pack archive is read.
        schema::parse_instance_config(
            r#"{"_version":"1","name":"Legacy","game_configuration":{"version":null}}"#,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn legacy_untouched_shapes_adopt_with_their_2_0_40_meaning() {
        let repair = RepairMarkerFile {
            re_enable_disabled: true,
            cleanup_paths: vec!["/mods/old.jar".into()],
            ..Default::default()
        };
        let repair_text = serde_json::to_vec(&repair).unwrap();
        let installed = config();
        let first = first_install_config();
        type Case<'a> = (
            &'a str,
            Vec<(&'a str, &'a [u8])>,
            Vec<&'a str>,
            &'a info::Instance,
            (Kind, Producer, Phase),
            Option<&'a str>,
        );
        let version_only = (Kind::VersionOnly, Producer::Create, Phase::Reserved);
        let complete = (
            Kind::ImportedComplete,
            Producer::Reconstructed,
            Phase::Committed,
        );
        let cases: Vec<Case> = vec![
            ("empty", vec![], vec![], &installed, version_only, None),
            (
                "empty staging tree from a version-only prepare",
                vec![],
                vec![".setup/staging/instance"],
                &installed,
                version_only,
                None,
            ),
            (
                "file-manager metadata",
                vec![(".setup/.DS_Store", b"finder")],
                vec![],
                &installed,
                version_only,
                None,
            ),
            (
                "update of an installed pack",
                vec![
                    (".setup/change-pack-version.json", PACK.as_bytes()),
                    ("packinfo.json", b"installed record"),
                ],
                vec![],
                &installed,
                (Kind::Pack, Producer::Change, Phase::Reserved),
                Some(PACK),
            ),
            (
                // 2.0.40 importers never write packinfo.json, but know the
                // game version.
                "update of an imported pack",
                vec![(".setup/change-pack-version.json", PACK.as_bytes())],
                vec![],
                &installed,
                (Kind::Pack, Producer::Change, Phase::Reserved),
                Some(PACK),
            ),
            (
                "repair",
                vec![
                    (".setup/change-pack-version.json", PACK.as_bytes()),
                    (".setup/repair", &repair_text),
                    ("packinfo.json", b"installed record"),
                ],
                vec![],
                &installed,
                (Kind::Pack, Producer::Repair, Phase::Reserved),
                Some(PACK),
            ),
            (
                "first install",
                vec![(".setup/change-pack-version.json", PACK.as_bytes())],
                vec![],
                &first,
                (Kind::Pack, Producer::Create, Phase::Reserved),
                Some(PACK),
            ),
            (
                // A 2.0.40 Repair of a first install that never finished.
                "repair of a first install",
                vec![
                    (".setup/change-pack-version.json", PACK.as_bytes()),
                    (".setup/repair", &repair_text),
                ],
                vec![],
                &first,
                (Kind::Pack, Producer::Create, Phase::Reserved),
                Some(PACK),
            ),
            (
                "version-only install that failed after its pack step",
                vec![(".setup/modpack-complete", b"")],
                vec![],
                &installed,
                complete,
                None,
            ),
            (
                "never-launched import",
                vec![(".setup/change-pack-version.json", PACK.as_bytes())],
                vec![".setup/modpack-complete"],
                &installed,
                complete,
                Some(PACK),
            ),
        ];
        for (name, files, dirs, config, (kind, producer, phase), requested) in cases {
            let repaired = files.iter().any(|(path, _)| *path == ".setup/repair");
            let root = legacy_root(&files, &dirs);
            let before = tree(root.path());
            let adopted = Session::adopt_legacy(root.path(), InstanceId(7), config)
                .await
                .unwrap()
                .unwrap_or_else(|| panic!("{name} must qualify"));
            assert_eq!(
                (adopted.kind, adopted.producer, adopted.phase),
                (kind, producer, phase),
                "{name}"
            );
            assert_eq!(adopted.owner().unwrap().instance_id, 7, "{name}");
            assert!(
                adopted.target.is_none() && adopted.archive.is_none(),
                "{name}"
            );
            assert!(adopted.pending_config.is_none(), "{name}");
            assert_eq!(
                version_key(
                    &schema::parse_instance_config(&adopted.installed_config)
                        .unwrap()
                        .game_configuration
                        .version
                ),
                version_key(&config.game_configuration.version),
                "{name}"
            );
            assert_eq!(adopted.requested_pack.as_deref(), requested, "{name}");
            // Repair options are kept whatever the producer, so Play runs the
            // repair 2.0.40 started.
            if repaired {
                assert_eq!(adopted.original_options.as_ref(), Some(&repair), "{name}");
            } else {
                assert!(adopted.original_options.is_none(), "{name}");
            }
            // Only a pack with an installed version can be cancelled back to it.
            if kind == Kind::Pack {
                let reason = adopted.cancel_reason(config);
                if config.game_configuration.version.is_none() {
                    let reason = reason.unwrap_or_else(|| panic!("{name} must not cancel"));
                    assert!(reason.contains("first install"), "{name}: {reason}");
                    assert!(reason.contains("Press Play"), "{name}: {reason}");
                } else {
                    assert_eq!(reason, None, "{name}");
                }
            }
            // Only the record is added; every input keeps its exact bytes.
            let mut after = tree(root.path());
            assert!(
                after.remove(".setup/setup-session.json").is_some(),
                "{name}"
            );
            assert_eq!(after, before, "{name}");
            let loaded = Session::load(root.path()).await.unwrap().unwrap();
            assert_eq!(loaded.operation, adopted.operation, "{name}");
            // A second adoption never replaces the published record.
            assert!(
                Session::adopt_legacy(root.path(), InstanceId(8), config)
                    .await
                    .unwrap()
                    .is_none(),
                "{name}"
            );
            assert_eq!(
                Session::load(root.path()).await.unwrap().unwrap().operation,
                adopted.operation,
                "{name}"
            );
        }
    }

    #[tokio::test]
    async fn legacy_apply_evidence_is_never_adopted_and_nothing_is_written() {
        let request: (&str, &[u8]) = (".setup/change-pack-version.json", PACK.as_bytes());
        let cases: Vec<(&str, Vec<(&str, &[u8])>, Vec<&str>)> = vec![
            (
                "curseforge archive",
                vec![request, (".setup/curseforge", b"zip")],
                vec![],
            ),
            (
                "modrinth archive",
                vec![request, (".setup/modrinth", b"zip")],
                vec![],
            ),
            ("gdlpack archive", vec![(".setup/gdlpack", b"zip")], vec![]),
            (
                "modpack-complete beside a modrinth archive",
                vec![
                    request,
                    (".setup/modrinth", b"zip"),
                    (".setup/modpack-complete", b""),
                ],
                vec![],
            ),
            (
                "modpack-complete beside empty staging",
                vec![(".setup/modpack-complete", b"")],
                vec![".setup/staging/instance"],
            ),
            (
                "modpack-complete beside apply-started",
                vec![
                    (".setup/modpack-complete", b""),
                    (".setup/apply-started", b""),
                ],
                vec![],
            ),
            (
                "modpack-complete beside a repair marker",
                vec![
                    request,
                    (".setup/repair", br#"{"re_enable_disabled":false}"#),
                    (".setup/modpack-complete", b""),
                ],
                vec![],
            ),
            (
                "modpack-complete with a root tmp-packinfo.json",
                vec![
                    (".setup/modpack-complete", b""),
                    ("tmp-packinfo.json", b"tmp"),
                ],
                vec![],
            ),
            (
                "case variant modpack-complete",
                vec![(".setup/Modpack-Complete", b"")],
                vec![],
            ),
            (
                "staging with a file",
                vec![request, (".setup/staging/instance/mods/a.jar", b"jar")],
                vec![],
            ),
            (
                "staging-packinfo.json",
                vec![request, (".setup/staging-packinfo.json", b"[]")],
                vec![],
            ),
            (
                "modpack-skip-overrides",
                vec![request],
                vec![".setup/modpack-skip-overrides"],
            ),
            (
                "apply-started",
                vec![request, (".setup/apply-started", b"")],
                vec![],
            ),
            (
                "representation-transitions",
                vec![request],
                vec![".setup/representation-transitions"],
            ),
            ("preparing-x", vec![request], vec![".setup/preparing-x"]),
            (
                "prior-staging",
                vec![request],
                vec![".setup/prior-staging-x"],
            ),
            (
                "root tmp-packinfo.json",
                vec![request, ("tmp-packinfo.json", b"tmp")],
                vec![],
            ),
            (
                "repair without change-pack-version.json",
                vec![(
                    ".setup/repair",
                    br#"{"re_enable_disabled":false,"cleanup_paths":[]}"#,
                )],
                vec![],
            ),
            (
                "case variant marker",
                vec![(".setup/Apply-Started", b"")],
                vec![],
            ),
            (
                "request that is a directory",
                vec![],
                vec![".setup/change-pack-version.json"],
            ),
        ];
        for (name, files, dirs) in cases {
            let root = legacy_root(&files, &dirs);
            let before = tree(root.path());
            assert!(
                classify_legacy(root.path()).await.unwrap().is_none(),
                "{name}"
            );
            assert!(
                Session::adopt_legacy(root.path(), InstanceId(7), &config())
                    .await
                    .unwrap()
                    .is_none(),
                "{name}"
            );
            assert_eq!(tree(root.path()), before, "{name}");
        }
        // No `.setup` at all is not a legacy setup either.
        let root = tempfile::tempdir().unwrap();
        assert!(classify_legacy(root.path()).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn legacy_existing_record_is_never_overwritten() {
        let root = legacy_root(&[], &[]);
        let config = config();
        let mut existing =
            Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap();
        existing.instance_id = Some(7);
        existing.create(root.path()).await.unwrap();
        let bytes = std::fs::read(root.path().join(".setup").join(RECORD)).unwrap();
        assert!(classify_legacy(root.path()).await.unwrap().is_none());
        assert!(
            Session::adopt_legacy(root.path(), InstanceId(7), &config)
                .await
                .unwrap()
                .is_none()
        );
        let loaded = Session::load_or_adopt_legacy(root.path(), InstanceId(7), &config)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(loaded.producer, Producer::Settings);
        assert_eq!(
            std::fs::read(root.path().join(".setup").join(RECORD)).unwrap(),
            bytes
        );
        // A copied record beside an untouched request is pre-download; any
        // archive or apply marker beside it is not.
        std::fs::write(
            root.path().join(".setup/change-pack-version.json"),
            PACK.as_bytes(),
        )
        .unwrap();
        assert!(untouched_beside_record(root.path()).await.unwrap());
        for evidence in ["modrinth", "apply-started", "modpack-complete"] {
            std::fs::write(root.path().join(".setup").join(evidence), b"").unwrap();
            assert!(
                !untouched_beside_record(root.path()).await.unwrap(),
                "{evidence}"
            );
            std::fs::remove_file(root.path().join(".setup").join(evidence)).unwrap();
        }
    }

    #[tokio::test]
    async fn legacy_version_only_adoption_is_supersedable_and_pack_is_not() {
        let config = config();
        let root = legacy_root(&[], &[]);
        let adopted = Session::adopt_legacy(root.path(), InstanceId(7), &config)
            .await
            .unwrap()
            .unwrap();
        assert!(adopted.is_content_free());
        assert!(
            adopted
                .supersedable(root.path(), InstanceId(7))
                .await
                .unwrap()
        );
        assert!(
            !adopted
                .supersedable(root.path(), InstanceId(8))
                .await
                .unwrap()
        );
        std::fs::write(root.path().join(".setup/apply-started"), b"").unwrap();
        assert!(
            !adopted
                .supersedable(root.path(), InstanceId(7))
                .await
                .unwrap()
        );
        // Once Play marks it Committed it still holds no pack content.
        let mut committed = adopted.clone();
        committed.phase = Phase::Committed;
        assert!(committed.is_content_free());
        // 2.0.40 did not record whether an empty setup was a first install or
        // a version change, so the reason claims neither.
        let reason = adopted.cancel_reason(&config).unwrap();
        assert!(reason.contains("Press Play"), "{reason}");
        assert!(!reason.contains("first install"), "{reason}");

        // A clean modpack-complete leftover only waits for its game version.
        for marker_is_dir in [false, true] {
            let root = if marker_is_dir {
                legacy_root(&[], &[".setup/modpack-complete"])
            } else {
                legacy_root(&[(".setup/modpack-complete", b"")], &[])
            };
            let complete = Session::adopt_legacy(root.path(), InstanceId(7), &config)
                .await
                .unwrap()
                .unwrap();
            assert!(complete.is_content_free());
            assert!(
                complete
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap()
            );
            let reason = complete.cancel_reason(&config).unwrap();
            assert!(reason.contains("Press Play"), "{reason}");
            assert!(!reason.contains("first install"), "{reason}");
            for evidence in ["apply-started", "modrinth"] {
                std::fs::write(root.path().join(".setup").join(evidence), b"").unwrap();
                assert!(
                    !complete
                        .supersedable(root.path(), InstanceId(7))
                        .await
                        .unwrap(),
                    "{evidence}"
                );
                std::fs::remove_file(root.path().join(".setup").join(evidence)).unwrap();
            }
        }

        let root = legacy_root(
            &[
                (".setup/change-pack-version.json", PACK.as_bytes()),
                ("packinfo.json", b"installed record"),
            ],
            &[],
        );
        let pack = Session::adopt_legacy(root.path(), InstanceId(7), &config)
            .await
            .unwrap()
            .unwrap();
        assert!(!pack.is_content_free());
        assert!(!pack.supersedable(root.path(), InstanceId(7)).await.unwrap());
        // An untouched pending change can be cancelled back to the installed
        // version; a first install has no installed version to go back to.
        assert!(pack.cancel_reason(&config).is_none());
        let mut first = pack.clone();
        first.producer = Producer::Create;
        let reason = first.cancel_reason(&config).unwrap();
        assert!(reason.contains("first install"), "{reason}");
        assert!(reason.contains("Press Play"), "{reason}");
        // Whichever operation restarted it, a pack install that never set a
        // game version has nothing launchable to cancel back to.
        let versionless = first_install_config();
        for producer in [Producer::Change, Producer::Repair] {
            let mut restarted = Session::new(&versionless, producer, Kind::Pack, None).unwrap();
            restarted.instance_id = Some(7);
            let reason = restarted.cancel_reason(&versionless).unwrap();
            assert!(reason.contains("first install"), "{producer:?}: {reason}");
            assert!(reason.contains("Press Play"), "{producer:?}: {reason}");
        }
    }

    #[tokio::test]
    async fn legacy_copied_pre_download_requests_qualify_unless_bound_or_touched() {
        let config = config();
        let repair = RepairMarkerFile {
            cleanup_paths: vec!["/mods/old.jar".into()],
            ..Default::default()
        };
        let root = legacy_root(
            &[
                (".setup/change-pack-version.json", PACK.as_bytes()),
                (".setup/repair", &serde_json::to_vec(&repair).unwrap()),
                ("packinfo.json", b"installed record"),
            ],
            &[],
        );
        let mut source = Session::adopt_legacy(root.path(), InstanceId(7), &config)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(source.producer, Producer::Repair);
        // The source owns it. The instance the folder is registered to after
        // a database reset (or a copy made outside the launcher) qualifies it,
        // like a duplicate does.
        assert_eq!(
            source
                .copy_qualification(root.path(), InstanceId(7))
                .await
                .unwrap(),
            None
        );
        assert!(!source.binds_to(InstanceId(7)));
        assert!(source.binds_to(InstanceId(8)));
        assert_eq!(
            source
                .copy_qualification(root.path(), InstanceId(8))
                .await
                .unwrap(),
            Some(true)
        );
        source.copied_to = Some(8);
        assert!(source.binds_to(InstanceId(8)));
        assert_eq!(
            source
                .copy_qualification(root.path(), InstanceId(8))
                .await
                .unwrap(),
            Some(true)
        );
        // Options bound to the source's storage stay with reconstruction.
        let mut bound = source.clone();
        bound.original_options.as_mut().unwrap().cleanup_batch = Some("batch".into());
        assert_eq!(
            bound
                .copy_qualification(root.path(), InstanceId(8))
                .await
                .unwrap(),
            None
        );
        // So does a request whose archive arrived or whose apply started.
        for evidence in ["modrinth", "apply-started"] {
            std::fs::write(root.path().join(".setup").join(evidence), b"").unwrap();
            assert_eq!(
                source
                    .copy_qualification(root.path(), InstanceId(8))
                    .await
                    .unwrap(),
                None,
                "{evidence}"
            );
            std::fs::remove_file(root.path().join(".setup").join(evidence)).unwrap();
        }
        // A non-pack copy beside apply evidence is refused, not qualified.
        let mut version_only = source.clone();
        version_only.kind = Kind::VersionOnly;
        version_only.original_options = None;
        std::fs::write(root.path().join(".setup/apply-started"), b"").unwrap();
        assert_eq!(
            version_only
                .copy_qualification(root.path(), InstanceId(8))
                .await
                .unwrap(),
            Some(false)
        );
    }

    /// Every field and enum value that decides whether a setup may hold pack
    /// content, one at a time.
    #[test]
    fn content_free_truth_table() {
        let config = config();
        let base = |producer, kind, phase| {
            let mut session = Session::new(&config, producer, kind, None).unwrap();
            session.instance_id = Some(7);
            session.phase = phase;
            session
        };
        let producers = [
            Producer::Change,
            Producer::Repair,
            Producer::Create,
            Producer::Import,
            Producer::Settings,
            Producer::Reconstructed,
        ];
        let kinds = [Kind::Pack, Kind::VersionOnly, Kind::ImportedComplete];
        let phases = [
            Phase::Reserved,
            Phase::Resolved,
            Phase::Applying,
            Phase::Applied,
            Phase::Committed,
        ];
        for producer in producers {
            for kind in kinds {
                for phase in phases {
                    let expected = !matches!(producer, Producer::Change | Producer::Repair)
                        && kind != Kind::Pack
                        && matches!(phase, Phase::Reserved | Phase::Committed);
                    assert_eq!(
                        base(producer, kind, phase).is_content_free(),
                        expected,
                        "{producer:?} {kind:?} {phase:?}"
                    );
                }
            }
        }
        // Any recorded pack input or pending configuration means content.
        let target = packinfo::PackInfo {
            files: Default::default(),
        };
        let (digest, options_digest) = binding(&target, None).unwrap();
        for (producer, kind) in [
            (Producer::Settings, Kind::VersionOnly),
            (Producer::Create, Kind::VersionOnly),
            (Producer::Import, Kind::ImportedComplete),
            (Producer::Reconstructed, Kind::ImportedComplete),
        ] {
            for phase in [Phase::Reserved, Phase::Committed] {
                let clean = base(producer, kind, phase);
                assert!(clean.is_content_free(), "{producer:?} {kind:?} {phase:?}");
                let mut with_target = clean.clone();
                with_target.target = Some(Target {
                    packinfo: packinfo::make_packinfo(target.clone()).unwrap(),
                    digest: digest.clone(),
                    options_digest: options_digest.clone(),
                    sources: BTreeMap::new(),
                    skipped: Vec::new(),
                });
                let mut with_options = clean.clone();
                with_options.original_options = Some(RepairMarkerFile::default());
                let mut with_archive = clean.clone();
                with_archive.archive = Some(Archive {
                    name: "modrinth".into(),
                    sha512: "00".into(),
                });
                let mut with_pending = clean.clone();
                with_pending.pending_config = Some(clean.installed_config.clone());
                for (field, session) in [
                    ("target", with_target),
                    ("original_options", with_options),
                    ("archive", with_archive),
                    ("pending_config", with_pending),
                ] {
                    assert!(
                        !session.is_content_free(),
                        "{producer:?} {kind:?} {phase:?} with {field}"
                    );
                }
            }
        }
    }

    /// A new version may replace only this instance's own content-free setup
    /// with nothing beside it that a pack setup would leave.
    #[tokio::test]
    async fn supersedable_needs_owner_content_free_and_no_pack_evidence() {
        let config = config();
        let root = legacy_root(&[], &[]);
        let mut settings =
            Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap();
        settings.instance_id = Some(7);
        settings.create(root.path()).await.unwrap();
        for phase in [Phase::Reserved, Phase::Committed] {
            settings.phase = phase;
            assert!(
                settings
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap(),
                "{phase:?}"
            );
        }
        let mut unbound = settings.clone();
        unbound.instance_id = None;
        let mut copied = settings.clone();
        copied.copied_to = Some(8);
        let mut pack = settings.clone();
        pack.kind = Kind::Pack;
        let mut applying = settings.clone();
        applying.phase = Phase::Applying;
        for (name, session, instance) in [
            ("another instance", settings.clone(), 8),
            ("unbound", unbound, 7),
            ("copied", copied.clone(), 7),
            ("copied, seen from the copy", copied, 8),
            ("pack", pack, 7),
            ("applying", applying, 7),
        ] {
            assert!(
                !session
                    .supersedable(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
        }
        for evidence in [
            "apply-started",
            "representation-transitions",
            "staging-packinfo.json",
            "curseforge",
            "modrinth",
            "gdlpack",
        ] {
            std::fs::write(root.path().join(".setup").join(evidence), b"").unwrap();
            assert!(
                !settings
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap(),
                "{evidence}"
            );
            std::fs::remove_file(root.path().join(".setup").join(evidence)).unwrap();
        }
        // Still never cancellable on its own: there is no earlier version to
        // go back to, and the reason says how to finish it. Choosing another
        // version is offered only without a modpack, whose files target the
        // pack's own version.
        let mut modpack = config.clone();
        modpack.modpack = Some(info::ModpackInfo {
            modpack: info::Modpack::Modrinth(info::ModrinthModpack {
                project_id: "synthetic".into(),
                version_id: "B".into(),
            }),
            locked: false,
        });
        for producer in [
            Producer::Settings,
            Producer::Create,
            Producer::Import,
            Producer::Reconstructed,
        ] {
            let mut session = settings.clone();
            session.producer = producer;
            let reason = session.cancel_reason(&config).unwrap();
            assert!(reason.contains("Press Play"), "{producer:?}: {reason}");
            assert!(reason.contains("with Edit"), "{producer:?}: {reason}");
            let reason = session.cancel_reason(&modpack).unwrap();
            assert!(reason.contains("Press Play"), "{producer:?}: {reason}");
            assert!(!reason.contains("Edit"), "{producer:?}: {reason}");
        }
    }

    /// Only a record bound to the instance, and not marked as a duplicate's
    /// copy, is the instance's own. Any other record in the instance's folder
    /// is provenance it may continue: one written before it had an id, one a
    /// duplicate copied, or one the folder kept from an earlier registration,
    /// which a database reset or an outside copy, move or rename leaves
    /// naming an id the folder no longer has. Such a record used to be
    /// refused as another instance's, for Play, Edit and addon changes alike.
    #[tokio::test]
    async fn a_record_from_an_earlier_registration_is_provenance() {
        let config = config();
        let root = legacy_root(&[], &[]);
        let mut original =
            Session::new(&config, Producer::Create, Kind::VersionOnly, None).unwrap();
        original.instance_id = Some(7);
        original.create(root.path()).await.unwrap();
        let mut copied = original.clone();
        copied.copied_to = Some(8);
        let mut unbound = original.clone();
        unbound.instance_id = None;
        for (name, session, instance, owned) in [
            ("its own", &original, 7, true),
            ("under a new id", &original, 9, false),
            ("unbound", &unbound, 7, false),
            ("copied, seen from the copy", &copied, 8, false),
            ("copied, under a new id", &copied, 9, false),
            ("copied, under the original's id", &copied, 7, false),
        ] {
            assert_eq!(session.owned_by(InstanceId(instance)), owned, "{name}");
            assert_eq!(session.binds_to(InstanceId(instance)), !owned, "{name}");
            // A content-free record is continued as the instance's own.
            assert_eq!(
                session
                    .copy_qualification(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                (!owned).then_some(true),
                "{name}"
            );
        }
        // Continuing it makes it the new id's own, keeping where it came from.
        let continued = original
            .qualify_copy(root.path(), InstanceId(9))
            .await
            .unwrap();
        assert!(continued.owned_by(InstanceId(9)));
        assert_eq!(continued.adopted_from, original.owner().ok());
        assert_ne!(continued.operation, original.operation);
        let saved = Session::load(root.path()).await.unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(saved).unwrap(),
            serde_json::to_value(&continued).unwrap()
        );
    }

    /// A content-free record that the instance continues as its own (copied
    /// by a duplicate, or written before the instance had an id) may be
    /// replaced once it does, as Play would continue it. Anything Play would
    /// not continue as a clean version install stays.
    #[tokio::test]
    async fn content_free_copies_are_supersedable_once_continued() {
        let config = config();
        let root = legacy_root(&[], &[]);
        let mut original =
            Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap();
        original.instance_id = Some(7);
        original.phase = Phase::Committed;
        original.create(root.path()).await.unwrap();
        let mut copied = original.clone();
        copied.copied_to = Some(8);
        let mut unbound = original.clone();
        unbound.instance_id = None;
        // The folder kept it from an earlier registration: another id took
        // over the original's folder or the copy's (a database reset or
        // restore, or the folder copied or moved outside the launcher).
        for (name, session, instance) in [
            ("copied", &copied, 8),
            ("unbound", &unbound, 7),
            ("the original's, under a new id", &original, 9),
            ("copied, under a new id", &copied, 9),
            ("copied, under the original's id", &copied, 7),
        ] {
            assert!(
                !session
                    .supersedable(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
            assert!(
                session
                    .supersedable_once_continued(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
        }
        let mut copied_pack = copied.clone();
        copied_pack.kind = Kind::Pack;
        copied_pack.phase = Phase::Reserved;
        for (name, session, instance) in [
            ("the original's own", &original, 7),
            ("a copied pack request", &copied_pack, 8),
        ] {
            assert!(
                !session
                    .supersedable_once_continued(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
        }
        for evidence in ["apply-started", "staging-packinfo.json", "modrinth"] {
            std::fs::write(root.path().join(".setup").join(evidence), b"").unwrap();
            assert!(
                !copied
                    .supersedable_once_continued(root.path(), InstanceId(8))
                    .await
                    .unwrap(),
                "{evidence}"
            );
            std::fs::remove_file(root.path().join(".setup").join(evidence)).unwrap();
        }
    }

    /// A pack setup whose files and configuration were published only waits
    /// for its game version install, like a completed import: a new version
    /// may replace it, whatever finished-install leftovers sit beside it, but
    /// only while the instance's packinfo.json is still exactly the target it
    /// promoted.
    #[tokio::test]
    async fn pack_installed_needs_the_promoted_target_in_place() {
        let config = config();
        let root = legacy_root(&[], &[]);
        let files = |byte| packinfo::PackInfo {
            files: std::collections::HashMap::from([(
                "/mods/a.jar".to_string(),
                packinfo::FileHashes {
                    sha512: [byte; 64],
                    md5: [byte; 16],
                },
            )]),
        };
        let target = files(1);
        let (digest, options_digest) = binding(&target, None).unwrap();
        let mut pack = Session::new(&config, Producer::Import, Kind::Pack, None).unwrap();
        pack.instance_id = Some(7);
        pack.phase = Phase::Committed;
        pack.pending_config = Some(pack.installed_config.clone());
        pack.target = Some(Target {
            packinfo: packinfo::make_packinfo(target.clone()).unwrap(),
            digest,
            options_digest,
            sources: BTreeMap::from([("/mods/a.jar".to_string(), "/mods/a.jar".to_string())]),
            skipped: Vec::new(),
        });
        pack.create(root.path()).await.unwrap();
        let promoted = root.path().join("packinfo.json");
        std::fs::write(&promoted, packinfo::make_packinfo(target.clone()).unwrap()).unwrap();
        // What a finished pack install leaves beside its record.
        for leftover in [
            "modrinth",
            "change-pack-version.json",
            "apply-started",
            "representation-transitions",
            "staging-packinfo.json",
            "modpack-complete",
        ] {
            std::fs::write(root.path().join(".setup").join(leftover), b"").unwrap();
        }
        for producer in [
            Producer::Create,
            Producer::Import,
            Producer::Change,
            Producer::Repair,
            Producer::Reconstructed,
        ] {
            let mut session = pack.clone();
            session.producer = producer;
            assert!(
                session.pack_installed(root.path()).await.unwrap(),
                "{producer:?}"
            );
            assert!(
                session
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap(),
                "{producer:?}"
            );
        }
        // Not this instance's own: a copy, a record from before the instance
        // had an id, or one the folder kept from an earlier registration
        // (another id, or a copy's, after a database reset). Each is still a
        // pack installed for the instance the folder is registered to now.
        let mut copied = pack.clone();
        copied.copied_to = Some(8);
        let mut unbound = pack.clone();
        unbound.instance_id = None;
        for (name, session, instance, installed_for_it) in [
            ("another instance's id", &pack, 8, true),
            ("copied, now under the original's id", &copied, 7, true),
            ("copied, seen from the copy", &copied, 8, true),
            ("copied, now under a third id", &copied, 9, true),
            ("unbound", &unbound, 7, true),
        ] {
            assert!(
                !session
                    .supersedable(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
            assert!(
                !session
                    .supersedable_once_continued(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                "{name}"
            );
            assert_eq!(
                session
                    .copied_pack_installed(root.path(), InstanceId(instance))
                    .await
                    .unwrap(),
                installed_for_it,
                "{name}"
            );
        }
        assert!(
            !pack
                .copied_pack_installed(root.path(), InstanceId(7))
                .await
                .unwrap()
        );
        // Before its configuration was published.
        for phase in [
            Phase::Reserved,
            Phase::Resolved,
            Phase::Applying,
            Phase::Applied,
        ] {
            let mut session = pack.clone();
            session.phase = phase;
            assert!(
                !session.pack_installed(root.path()).await.unwrap(),
                "{phase:?}"
            );
            assert!(
                !session
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap(),
                "{phase:?}"
            );
            let mut copy = copied.clone();
            copy.phase = phase;
            assert!(
                !copy
                    .copied_pack_installed(root.path(), InstanceId(8))
                    .await
                    .unwrap(),
                "{phase:?}"
            );
        }
        let mut unresolved = pack.clone();
        unresolved.target = None;
        assert!(!unresolved.pack_installed(root.path()).await.unwrap());
        // A promotion still waiting.
        let tmp = root.path().join("tmp-packinfo.json");
        std::fs::write(&tmp, packinfo::make_packinfo(target.clone()).unwrap()).unwrap();
        assert!(!pack.pack_installed(root.path()).await.unwrap());
        std::fs::remove_file(&tmp).unwrap();
        // An installed record that is not the target it promoted.
        for (name, bytes) in [
            (
                "another target",
                packinfo::make_packinfo(files(2)).unwrap().into_bytes(),
            ),
            ("unreadable", b"not a record".to_vec()),
        ] {
            std::fs::write(&promoted, bytes).unwrap();
            assert!(!pack.pack_installed(root.path()).await.unwrap(), "{name}");
            assert!(
                !pack.supersedable(root.path(), InstanceId(7)).await.unwrap(),
                "{name}"
            );
            assert!(
                !copied
                    .copied_pack_installed(root.path(), InstanceId(8))
                    .await
                    .unwrap(),
                "{name}"
            );
        }
        std::fs::remove_file(&promoted).unwrap();
        assert!(!pack.pack_installed(root.path()).await.unwrap());
        std::fs::create_dir(&promoted).unwrap();
        assert!(!pack.pack_installed(root.path()).await.unwrap());
    }

    /// What 2.0.40 leaves once a pack install, update or repair applied its
    /// files and its game version install then failed: a modpack-complete
    /// marker beside one archive, the request and apply leftovers, and the
    /// promoted packinfo.json. Anything that shows the apply did not finish,
    /// or that this build went further than a rebuild's `preparing-*` folder,
    /// is not that shape.
    #[tokio::test]
    async fn legacy_pack_applied_needs_a_finished_apply_beside_one_archive() {
        let applied = || {
            let root = legacy_root(
                &[
                    (".setup/modrinth", b"archive"),
                    (".setup/change-pack-version.json", PACK.as_bytes()),
                    (".setup/staging-packinfo.json", b"[]"),
                    (".setup/apply-started", b""),
                    (".setup/modpack-complete", b""),
                    (".setup/.DS_Store", b""),
                ],
                &[".setup/modpack-skip-overrides"],
            );
            std::fs::write(
                root.path().join("packinfo.json"),
                packinfo::make_packinfo(packinfo::PackInfo {
                    files: Default::default(),
                })
                .unwrap(),
            )
            .unwrap();
            root
        };
        let root = applied();
        let before = tree(root.path());
        assert!(legacy_pack_applied(root.path()).await.unwrap());
        assert_eq!(tree(root.path()), before);
        // A repair applies the same way; older builds wrote the marker as a
        // folder; a rebuild by this build that stopped before its record
        // touched nothing outside its own folder.
        let accepted: [(&str, fn(&Path)); 4] = [
            ("repair", |setup| {
                std::fs::write(setup.join("repair"), b"{}").unwrap()
            }),
            ("marker folder", |setup| {
                std::fs::remove_file(setup.join("modpack-complete")).unwrap();
                std::fs::create_dir(setup.join("modpack-complete")).unwrap();
            }),
            ("rebuild", |setup| {
                std::fs::create_dir_all(setup.join("preparing-x/instance/mods")).unwrap();
                std::fs::write(setup.join("preparing-x/instance/mods/a.jar"), b"a").unwrap();
            }),
            ("curseforge", |setup| {
                std::fs::rename(setup.join("modrinth"), setup.join("curseforge")).unwrap();
            }),
        ];
        for (name, change) in accepted {
            let root = applied();
            change(&root.path().join(".setup"));
            assert!(legacy_pack_applied(root.path()).await.unwrap(), "{name}");
        }
        let refused: [(&str, fn(&Path)); 15] = [
            ("no marker", |root| {
                std::fs::remove_file(root.join(".setup/modpack-complete")).unwrap()
            }),
            ("no archive", |root| {
                std::fs::remove_file(root.join(".setup/modrinth")).unwrap()
            }),
            ("two archives", |root| {
                std::fs::write(root.join(".setup/gdlpack"), b"archive").unwrap()
            }),
            ("archive folder", |root| {
                std::fs::remove_file(root.join(".setup/modrinth")).unwrap();
                std::fs::create_dir(root.join(".setup/modrinth")).unwrap();
            }),
            ("staging", |root| {
                std::fs::create_dir(root.join(".setup/staging")).unwrap()
            }),
            ("promotion waiting", |root| {
                std::fs::write(root.join("tmp-packinfo.json"), b"{}").unwrap()
            }),
            ("no installed record", |root| {
                std::fs::remove_file(root.join("packinfo.json")).unwrap()
            }),
            ("unreadable installed record", |root| {
                std::fs::write(root.join("packinfo.json"), b"not a record").unwrap()
            }),
            ("record", |root| {
                std::fs::write(root.join(".setup").join(RECORD), b"{}").unwrap()
            }),
            ("prior staging", |root| {
                std::fs::create_dir(root.join(".setup/prior-staging-x")).unwrap()
            }),
            ("transitions", |root| {
                std::fs::create_dir(root.join(".setup/representation-transitions")).unwrap()
            }),
            ("previous tmp record", |root| {
                std::fs::write(root.join(".setup/previous-tmp-packinfo.json"), b"{}").unwrap()
            }),
            ("cancelled target", |root| {
                std::fs::write(root.join(".setup/cancelled-target.json"), b"{}").unwrap()
            }),
            ("request folder", |root| {
                std::fs::remove_file(root.join(".setup/change-pack-version.json")).unwrap();
                std::fs::create_dir(root.join(".setup/change-pack-version.json")).unwrap();
            }),
            ("setup file", |root| {
                std::fs::remove_dir_all(root.join(".setup")).unwrap();
                std::fs::write(root.join(".setup"), b"").unwrap();
            }),
        ];
        for (name, change) in refused {
            let root = applied();
            change(root.path());
            assert!(!legacy_pack_applied(root.path()).await.unwrap(), "{name}");
        }
        assert!(
            !legacy_pack_applied(&root.path().join("missing"))
                .await
                .unwrap()
        );
    }

    /// Play adopts a pack 2.0.40 finished applying as a committed install
    /// whose target is the record that apply promoted, so only the game
    /// version install remains, as in 2.0.40. Only when the configuration
    /// names the requested pack and has a game version, as 2.0.40 left it;
    /// otherwise nothing is written and Play rebuilds it from its archive.
    #[tokio::test]
    async fn an_applied_legacy_pack_is_adopted_as_its_installed_record() {
        let installed = packinfo::PackInfo {
            files: [(
                "/mods/a.jar".to_owned(),
                packinfo::FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]
            .into_iter()
            .collect(),
        };
        let request =
            serde_json::to_string(&super::super::PackVersionFile::from(pack("B"))).unwrap();
        let applied = |repair: Option<&RepairMarkerFile>| {
            let root = legacy_root(
                &[
                    (".setup/modrinth", b"archive"),
                    (".setup/change-pack-version.json", request.as_bytes()),
                    (".setup/staging-packinfo.json", b"[]"),
                    (".setup/apply-started", b""),
                    (".setup/modpack-complete", b""),
                ],
                &[".setup/modpack-skip-overrides"],
            );
            if let Some(repair) = repair {
                std::fs::write(
                    root.path().join(".setup/repair"),
                    serde_json::to_vec(repair).unwrap(),
                )
                .unwrap();
            }
            std::fs::write(
                root.path().join("packinfo.json"),
                packinfo::make_packinfo(installed.clone()).unwrap(),
            )
            .unwrap();
            root
        };
        let mut configured = config();
        configured.modpack = Some(info::ModpackInfo {
            modpack: pack("B"),
            locked: true,
        });
        let repair = RepairMarkerFile {
            re_enable_disabled: true,
            ..Default::default()
        };
        for options in [None, Some(&repair)] {
            let root = applied(options);
            let adopted =
                Session::adopt_legacy_applied_pack(root.path(), InstanceId(7), &configured)
                    .await
                    .unwrap()
                    .expect("adopted");
            assert_eq!(
                (adopted.producer, adopted.kind, adopted.phase),
                (Producer::Reconstructed, Kind::Pack, Phase::Committed)
            );
            assert_eq!(adopted.instance_id, Some(7));
            assert_eq!(adopted.original_options.as_ref(), options);
            assert_eq!(adopted.requested_pack.as_ref(), Some(&request));
            assert_eq!(
                adopted.pending_config,
                Some(adopted.installed_config.clone())
            );
            let target = adopted.target.as_ref().unwrap();
            assert!(target.sources.is_empty());
            assert_eq!(target.skipped, ["/mods/a.jar"]);
            // What Play, Repair and a version change read back.
            let loaded = Session::load(root.path()).await.unwrap().unwrap();
            assert_eq!(loaded.operation, adopted.operation);
            assert!(loaded.pack_installed(root.path()).await.unwrap());
            assert!(
                loaded
                    .supersedable(root.path(), InstanceId(7))
                    .await
                    .unwrap()
            );
            assert!(!loaded.interrupted(root.path()));
        }

        let mut other = configured.clone();
        other.modpack.as_mut().unwrap().modpack = pack("A");
        let mut versionless = configured.clone();
        versionless.game_configuration.version = None;
        for (name, config, change) in [
            ("another pack", &other, None),
            ("no game version", &versionless, None),
            (
                "damaged request",
                &configured,
                Some((".setup/change-pack-version.json", b"{".as_slice())),
            ),
            (
                "damaged repair",
                &configured,
                Some((".setup/repair", b"{}".as_slice())),
            ),
            (
                "damaged record",
                &configured,
                Some(("packinfo.json", b"not a record".as_slice())),
            ),
            (
                "archive missing",
                &configured,
                Some((".setup/modrinth", b"".as_slice())),
            ),
        ] {
            let root = applied(None);
            if let Some((path, bytes)) = change {
                if name == "archive missing" {
                    std::fs::remove_file(root.path().join(path)).unwrap();
                } else {
                    std::fs::write(root.path().join(path), bytes).unwrap();
                }
            }
            let before = tree(root.path());
            assert!(
                Session::adopt_legacy_applied_pack(root.path(), InstanceId(7), config)
                    .await
                    .unwrap()
                    .is_none(),
                "{name}"
            );
            assert_eq!(tree(root.path()), before, "{name}");
        }
        // Without a recorded request (an older build's), or once the user
        // unpaired the pack, nothing names another pack.
        let root = applied(None);
        std::fs::remove_file(root.path().join(".setup/change-pack-version.json")).unwrap();
        let adopted = Session::adopt_legacy_applied_pack(root.path(), InstanceId(7), &configured)
            .await
            .unwrap()
            .expect("adopted");
        assert_eq!(adopted.requested_pack, None);
        let mut unlinked = configured.clone();
        unlinked.modpack = None;
        let root = applied(None);
        let adopted = Session::adopt_legacy_applied_pack(root.path(), InstanceId(7), &unlinked)
            .await
            .unwrap()
            .expect("adopted");
        assert_eq!(adopted.requested_pack.as_ref(), Some(&request));
    }

    /// Retiring an installed pack's setup drops only its archive.
    #[tokio::test]
    async fn dropping_an_installed_pack_archive_keeps_the_rest() {
        let root = legacy_root(
            &[
                (".setup/modrinth", b"archive"),
                (".setup/curseforge", b"archive"),
                (".setup/setup-session.json", b"record"),
                (".setup/apply-started", b""),
            ],
            &[".setup/gdlpack", ".setup/staging/instance"],
        );
        let retained = retire(root.path(), "op").await.unwrap();
        drop_installed_pack_archive(&retained).await.unwrap();
        assert!(!retained.join("modrinth").exists());
        assert!(!retained.join("curseforge").exists());
        // Only plain files are archives.
        assert!(retained.join("gdlpack").is_dir());
        assert_eq!(std::fs::read(retained.join(RECORD)).unwrap(), b"record");
        assert!(retained.join("apply-started").is_file());
        assert!(retained.join("staging/instance").is_dir());
    }

    #[tokio::test]
    async fn retire_keeps_every_setup_under_a_unique_name() {
        let root = legacy_root(&[(".setup/evidence", b"keep")], &[]);
        let first = retire(root.path(), "op").await.unwrap();
        std::fs::create_dir(root.path().join(".setup")).unwrap();
        let second = retire(root.path(), "op").await.unwrap();
        assert_ne!(first, second);
        assert!(!root.path().join(".setup").exists());
        assert_eq!(std::fs::read(first.join("evidence")).unwrap(), b"keep");
        for retained in [&first, &second] {
            assert!(
                retained
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".setup-cancelled-op-")
            );
        }
    }

    /// Windows refuses to rename a setup folder while a scanner holds a file
    /// in it. Retiring, publishing a replacement and putting a retired setup
    /// back wait that out instead of failing the request.
    #[tokio::test(start_paused = true)]
    async fn setup_folder_renames_wait_out_a_busy_folder() {
        use crate::util::publish::fault::{SHARING_VIOLATION, Step, inject, pending};
        let root = legacy_root(&[(".setup/setup-session.json", b"old")], &[]);
        let setup = root.path().join(".setup");
        for _ in 0..2 {
            inject(Step::RenameDir, &setup, SHARING_VIOLATION);
        }
        let retired = retire_replaced(root.path(), "op").await.unwrap();
        assert!(!pending(Step::RenameDir, &setup));
        assert!(!setup.exists());

        let next = begin_replacement(root.path()).await.unwrap();
        atomic_bytes(&next.join(RECORD), b"new".to_vec(), true)
            .await
            .unwrap();
        inject(Step::RenameDir, &next, 5);
        publish_replacement(root.path(), &next).await.unwrap();
        assert!(!pending(Step::RenameDir, &next));
        assert_eq!(std::fs::read(setup.join(RECORD)).unwrap(), b"new");

        std::fs::remove_dir_all(&setup).unwrap();
        inject(Step::RenameDir, &retired.path, SHARING_VIOLATION);
        let path = retired.path.clone();
        retired.restore(root.path()).await.unwrap();
        assert!(!pending(Step::RenameDir, &path));
        assert_eq!(std::fs::read(setup.join(RECORD)).unwrap(), b"old");
    }

    fn pack(version: &str) -> info::Modpack {
        info::Modpack::Modrinth(info::ModrinthModpack {
            project_id: "synthetic".into(),
            version_id: version.into(),
        })
    }

    /// A setup that a replacing request retires takes the root
    /// `tmp-packinfo.json` with it, and comes back whole, record included,
    /// when its replacement can't be published.
    #[tokio::test]
    async fn a_replaced_setup_takes_its_target_record_and_can_be_put_back() {
        for collision in [false, true] {
            let mut files: Vec<(&str, &[u8])> = vec![
                (".setup/setup-session.json", b"record"),
                (".setup/staging/instance/a.jar", b"staged"),
                ("tmp-packinfo.json", b"target"),
            ];
            if collision {
                files.push((".setup/tmp-packinfo.json", b"inside"));
            }
            let root = legacy_root(&files, &[]);
            let before = tree(root.path());
            let retired = retire_replaced(root.path(), "op").await.unwrap();
            assert!(
                retired
                    .path
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".setup-cancelled-op-")
            );
            assert!(!root.path().join(".setup").exists());
            assert!(!root.path().join("tmp-packinfo.json").exists());
            let moved: Vec<_> = std::fs::read_dir(&retired.path)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| std::fs::read(path).ok().as_deref() == Some(b"target"))
                .collect();
            assert_eq!(moved.len(), 1, "{collision}");
            assert_eq!(
                moved[0].file_name().unwrap() == "tmp-packinfo.json",
                !collision
            );
            retired.restore(root.path()).await.unwrap();
            assert_eq!(tree(root.path()), before, "{collision}");
        }
        // A setup that took its place is never replaced by putting it back.
        let root = legacy_root(&[(".setup/setup-session.json", b"record")], &[]);
        let retired = retire_replaced(root.path(), "op").await.unwrap();
        std::fs::create_dir(root.path().join(".setup")).unwrap();
        std::fs::write(root.path().join(".setup/other"), b"other").unwrap();
        let before = tree(root.path());
        assert!(retired.restore(root.path()).await.is_err());
        assert_eq!(tree(root.path()), before);
    }

    fn replacements(root: &Path) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(root)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| {
                path.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(REPLACEMENT_PREFIX)
            })
            .collect()
    }

    /// A replacement is written in full beside the pending setup and only
    /// then takes its place; one that is given up leaves nothing behind but
    /// the temporary record it took, back where it was.
    #[tokio::test]
    async fn a_replacement_is_written_beside_the_setup_and_published_whole() {
        let root = legacy_root(&[("tmp-packinfo.json", b"stale")], &[]);
        std::fs::remove_dir(root.path().join(".setup")).unwrap();
        let before = tree(root.path());
        let next = begin_replacement(root.path()).await.unwrap();
        keep_stale_target(root.path(), &next).await.unwrap();
        atomic_bytes(&next.join(RECORD), b"record".to_vec(), true)
            .await
            .unwrap();
        assert!(!root.path().join(".setup").exists());
        assert!(!root.path().join("tmp-packinfo.json").exists());
        discard_replacement(root.path(), &next).await;
        assert_eq!(tree(root.path()), before);

        let next = begin_replacement(root.path()).await.unwrap();
        keep_stale_target(root.path(), &next).await.unwrap();
        atomic_bytes(&next.join(RECORD), b"record".to_vec(), true)
            .await
            .unwrap();
        publish_replacement(root.path(), &next).await.unwrap();
        assert!(replacements(root.path()).is_empty());
        assert_eq!(
            std::fs::read(root.path().join(".setup").join(RECORD)).unwrap(),
            b"record"
        );
        assert_eq!(
            std::fs::read(root.path().join(".setup").join(STALE_TARGET)).unwrap(),
            b"stale"
        );
        // Never over a setup that is still pending.
        let next = begin_replacement(root.path()).await.unwrap();
        let before = tree(root.path());
        assert!(publish_replacement(root.path(), &next).await.is_err());
        assert_eq!(tree(root.path()), before);
    }

    /// A crash between retiring the pending setup and publishing its
    /// replacement never leaves the instance without a setup for longer than
    /// it takes to read it again: the complete replacement is published. A
    /// replacement that never took effect goes.
    #[tokio::test]
    async fn a_replacement_a_crash_interrupted_is_settled() {
        // Retired, not yet published.
        let root = legacy_root(&[(".setup/setup-session.json", b"old")], &[]);
        retire(root.path(), "op").await.unwrap();
        let next = begin_replacement(root.path()).await.unwrap();
        atomic_bytes(&next.join("repair"), b"{}".to_vec(), true)
            .await
            .unwrap();
        atomic_bytes(&next.join(RECORD), b"new".to_vec(), true)
            .await
            .unwrap();
        settle_replacement(root.path()).await.unwrap();
        assert!(replacements(root.path()).is_empty());
        assert_eq!(
            std::fs::read(root.path().join(".setup").join(RECORD)).unwrap(),
            b"new"
        );
        assert!(root.path().join(".setup/repair").is_file());

        // Written (or half written) while the old setup was still pending.
        for complete in [false, true] {
            let root = legacy_root(&[(".setup/setup-session.json", b"old")], &[]);
            let next = begin_replacement(root.path()).await.unwrap();
            if complete {
                atomic_bytes(&next.join(RECORD), b"new".to_vec(), true)
                    .await
                    .unwrap();
            }
            let mut before = tree(root.path());
            before.retain(|path, _| !path.starts_with(REPLACEMENT_PREFIX));
            settle_replacement(root.path()).await.unwrap();
            assert_eq!(tree(root.path()), before, "{complete}");
        }

        // Half written with nothing pending: its record never landed.
        let root = legacy_root(&[], &[]);
        std::fs::remove_dir(root.path().join(".setup")).unwrap();
        let next = begin_replacement(root.path()).await.unwrap();
        atomic_bytes(&next.join("repair"), b"{}".to_vec(), true)
            .await
            .unwrap();
        settle_replacement(root.path()).await.unwrap();
        assert!(replacements(root.path()).is_empty());
        assert!(!root.path().join(".setup").exists());
    }

    /// A retired setup keeps its records and markers, never the downloads it
    /// staged or prepared.
    #[tokio::test]
    async fn trimming_a_retired_setup_keeps_only_its_records() {
        let root = legacy_root(
            &[
                (".setup/setup-session.json", b"record"),
                (".setup/change-pack-version.json", PACK.as_bytes()),
                (".setup/repair", b"{}"),
                (".setup/apply-started", b""),
                (".setup/modrinth", b"archive"),
                (".setup/gdlpack", b"archive"),
                (".setup/staging-packinfo.json", b"[]"),
                (".setup/prior-record-tmp-packinfo.json-1", b"prior"),
                (".setup/staging/instance/mods/a.jar", b"staged"),
                (".setup/preparing-1/instance/mods/a.jar", b"staged"),
                (".setup/prior-staging-2/instance/mods/a.jar", b"staged"),
                (".setup/representation-transitions/t.json", b"{}"),
            ],
            &[".setup/curseforge"],
        );
        let retired = retire(root.path(), "op").await.unwrap();
        trim_retired(&retired).await.unwrap();
        let mut kept: Vec<_> = std::fs::read_dir(&retired)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        kept.sort();
        assert_eq!(
            kept,
            [
                "apply-started",
                "change-pack-version.json",
                // Only a plain file is an archive.
                "curseforge",
                "prior-record-tmp-packinfo.json-1",
                "repair",
                "representation-transitions",
                "setup-session.json",
                "staging-packinfo.json",
            ]
        );
    }

    /// Once a later setup finished, every retired setup goes; the pending
    /// setup, recovery batches no retired record names and the instance are
    /// not touched.
    #[tokio::test]
    async fn pruning_removes_every_retired_setup_and_nothing_else() {
        let root = legacy_root(
            &[
                (".setup-cancelled-a-1/setup-session.json", b"a"),
                (".setup-cancelled-a-1/staging/instance/x", b"x"),
                (".setup-cancelled-legacy-2/modrinth", b"archive"),
                (".setup-cancelled-legacy-2/change-pack-version.json", b"b"),
                (".setup/staging/instance/x", b"live staging"),
                (".setup/modrinth", b"live archive"),
                (".repair-recovery/cleanup-1/entry/payload", b"user"),
                ("instance/staging/x", b"user folder"),
                // Not a retired setup's folder.
                (".setup-cancelled", b"file"),
            ],
            &[],
        );
        let untouched = |root: &Path| {
            tree(root)
                .into_iter()
                .filter(|(path, _)| !path.starts_with(".setup-cancelled-"))
                .collect::<BTreeMap<_, _>>()
        };
        let before = untouched(root.path());
        prune_retired(root.path()).await.unwrap();
        assert_eq!(untouched(root.path()), before);
        assert!(!root.path().join(".setup-cancelled-a-1").exists());
        assert!(!root.path().join(".setup-cancelled-legacy-2").exists());
    }

    /// What a retired Repair moved aside goes with it once a later setup
    /// finished, as far as its records show nothing needs it: a ticked file
    /// that left exactly as ticked, and what a finished re-enable moved out.
    /// A conflict, an unfinished move and anything unreadable stay.
    #[tokio::test]
    async fn pruning_a_retired_repair_deletes_the_copies_nothing_needs() {
        use crate::managers::instance::modpack::{CleanupSelection, PackProofContext};
        let key = |path: &str| hex::encode(&Sha512::digest(path.as_bytes())[..16]);
        let entry = |path: &str, status: &str| {
            let selection = CleanupSelection {
                path: path.into(),
                sha512: "00".repeat(64),
                context: PackProofContext {
                    platform: "modrinth".into(),
                    project_id: "p".into(),
                    version_id: "v".into(),
                },
                proven: true,
            };
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1,
                "selection": selection,
                "payload": "payload",
                "status": status,
            }))
            .unwrap()
        };
        let transition = |path: &str, batch: &str, completed: bool, conflict: bool| {
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1,
                "owner": null,
                "target_digest": "d",
                "options_digest": "o",
                "recovery_batch": batch,
                "entry": {"path": path, "action": "ReEnable", "reason": "ReEnabled"},
                "disabled": false,
                "destination_before": null,
                "opposite_before": null,
                "published": completed,
                "completed": completed,
                "conflict": conflict,
            }))
            .unwrap()
        };
        let marker = serde_json::to_vec(&RepairMarkerFile {
            re_enable_disabled: true,
            cleanup_batch: Some("cleanup-ticked".into()),
            ..Default::default()
        })
        .unwrap();
        let (removed, changed, pending) = (
            "/config/removed.cfg",
            "/config/changed.cfg",
            "/config/pending.cfg",
        );
        let (reenabled, conflicted, unfinished) = ("/mods/a.jar", "/mods/b.jar", "/mods/c.jar");
        let files: Vec<(String, Vec<u8>)> = vec![
            (".setup-cancelled-op-1/repair".into(), marker),
            (
                format!(
                    ".repair-recovery/cleanup-ticked/{}/entry.json",
                    key(removed)
                ),
                entry(removed, "removed-at-user-request"),
            ),
            (
                format!(".repair-recovery/cleanup-ticked/{}/payload", key(removed)),
                b"ticked".to_vec(),
            ),
            (
                format!(
                    ".repair-recovery/cleanup-ticked/{}/entry.json",
                    key(changed)
                ),
                entry(changed, "preserved-conflict"),
            ),
            (
                format!(".repair-recovery/cleanup-ticked/{}/payload", key(changed)),
                b"edited".to_vec(),
            ),
            (
                format!(
                    ".repair-recovery/cleanup-ticked/{}/entry.json",
                    key(pending)
                ),
                entry(pending, "pending"),
            ),
            (
                format!(".repair-recovery/cleanup-ticked/{}/payload", key(pending)),
                b"moving".to_vec(),
            ),
            // A ticked entry under another path's name is not this record's.
            (
                format!(
                    ".repair-recovery/cleanup-ticked/{}/entry.json",
                    key("/other")
                ),
                entry(removed, "removed-at-user-request"),
            ),
            (
                format!(
                    ".setup-cancelled-op-1/representation-transitions/{}.json",
                    key(reenabled)
                ),
                transition(reenabled, "cleanup-a", true, false),
            ),
            (
                format!(".repair-recovery/cleanup-a/{}/entry.json", key(reenabled)),
                b"{}".to_vec(),
            ),
            (
                format!(".repair-recovery/cleanup-a/{}/payload", key(reenabled)),
                b"twin".to_vec(),
            ),
            (
                format!(
                    ".setup-cancelled-op-1/representation-transitions/{}.json",
                    key(conflicted)
                ),
                transition(conflicted, "cleanup-b", true, true),
            ),
            (
                format!(".repair-recovery/cleanup-b/{}/payload", key(conflicted)),
                b"conflict".to_vec(),
            ),
            (
                format!(
                    ".setup-cancelled-op-1/representation-transitions/{}.json",
                    key(unfinished)
                ),
                transition(unfinished, "cleanup-c", false, false),
            ),
            (
                format!(".repair-recovery/cleanup-c/{}/payload", key(unfinished)),
                b"moved".to_vec(),
            ),
            (
                ".setup-cancelled-op-1/representation-transitions/damaged.json".into(),
                b"{".to_vec(),
            ),
            // A batch no record names, and one that escapes the folder.
            (
                ".repair-recovery/cleanup-other/x/payload".into(),
                b"other".to_vec(),
            ),
            (
                format!(
                    ".setup-cancelled-op-2/representation-transitions/{}.json",
                    key(reenabled)
                ),
                transition(reenabled, "..", true, false),
            ),
        ];
        let files: Vec<(&str, &[u8])> = files
            .iter()
            .map(|(path, bytes)| (path.as_str(), bytes.as_slice()))
            .collect();
        let root = legacy_root(&files, &[]);
        prune_retired(root.path()).await.unwrap();
        let recovery = root.path().join(".repair-recovery");
        let ticked = recovery.join("cleanup-ticked");
        assert!(!ticked.join(key(removed)).exists());
        assert_eq!(
            std::fs::read(ticked.join(key(changed)).join("payload")).unwrap(),
            b"edited"
        );
        assert_eq!(
            std::fs::read(ticked.join(key(pending)).join("payload")).unwrap(),
            b"moving"
        );
        assert!(ticked.join(key("/other")).join("entry.json").is_file());
        assert!(
            !recovery.join("cleanup-a").exists(),
            "an emptied batch goes"
        );
        assert_eq!(
            std::fs::read(
                recovery
                    .join("cleanup-b")
                    .join(key(conflicted))
                    .join("payload")
            )
            .unwrap(),
            b"conflict"
        );
        assert_eq!(
            std::fs::read(
                recovery
                    .join("cleanup-c")
                    .join(key(unfinished))
                    .join("payload")
            )
            .unwrap(),
            b"moved"
        );
        assert_eq!(
            std::fs::read(recovery.join("cleanup-other/x/payload")).unwrap(),
            b"other"
        );
        assert!(root.path().join(".setup").is_dir());
        assert!(!root.path().join(".setup-cancelled-op-1").exists());
        assert!(!root.path().join(".setup-cancelled-op-2").exists());
    }

    /// Disposing a committed setup deletes what its own apply moved aside
    /// and nothing needs, then the setup, then every retired one; the
    /// recovery folder goes once it is empty.
    #[tokio::test]
    async fn disposing_a_committed_setup_deletes_its_settled_recovery_copies() {
        let key = |path: &str| hex::encode(&Sha512::digest(path.as_bytes())[..16]);
        let root = legacy_root(
            &[(".setup-cancelled-op-1/setup-session.json", b"retired")],
            &[".setup/representation-transitions"],
        );
        let mut session = Session::new(&config(), Producer::Repair, Kind::Pack, None).unwrap();
        session.instance_id = Some(7);
        session.phase = Phase::Committed;
        session.create(root.path()).await.unwrap();
        let record = serde_json::json!({
            "schema_version": 2,
            "owner": session.owner().unwrap(),
            "target_digest": "d",
            "options_digest": "o",
            "recovery_batch": "cleanup-a",
            "entry": {"path": "/mods/a.jar", "action": "ReEnable", "reason": "ReEnabled"},
            "disabled": false,
            "destination_before": null,
            "opposite_before": null,
            "published": true,
            "completed": true,
            "conflict": false,
        });
        std::fs::write(
            root.path()
                .join(".setup/representation-transitions")
                .join(format!("{}.json", key("/mods/a.jar"))),
            serde_json::to_vec(&record).unwrap(),
        )
        .unwrap();
        let entry = root
            .path()
            .join(".repair-recovery/cleanup-a")
            .join(key("/mods/a.jar"));
        std::fs::create_dir_all(&entry).unwrap();
        std::fs::write(entry.join("payload"), b"twin").unwrap();
        // A batch a Repair made but never used, and one no record names.
        std::fs::create_dir(root.path().join(".repair-recovery/cleanup-unused")).unwrap();
        std::fs::create_dir_all(root.path().join(".repair-recovery/cleanup-other/x")).unwrap();
        std::fs::write(
            root.path().join(".repair-recovery/cleanup-other/x/payload"),
            b"x",
        )
        .unwrap();
        dispose(root.path(), &session.owner().unwrap())
            .await
            .unwrap();
        assert!(!root.path().join(".setup").exists());
        assert!(!root.path().join(".setup-cancelled-op-1").exists());
        assert!(!root.path().join(".repair-recovery/cleanup-a").exists());
        assert!(!root.path().join(".repair-recovery/cleanup-unused").exists());
        assert_eq!(
            std::fs::read(root.path().join(".repair-recovery/cleanup-other/x/payload")).unwrap(),
            b"x"
        );
        // Once nothing is left in it, the folder itself goes too.
        std::fs::remove_dir_all(root.path().join(".repair-recovery/cleanup-other")).unwrap();
        let mut again = Session::new(&config(), Producer::Repair, Kind::Pack, None).unwrap();
        again.instance_id = Some(7);
        again.phase = Phase::Committed;
        std::fs::create_dir(root.path().join(".setup")).unwrap();
        again.create(root.path()).await.unwrap();
        dispose(root.path(), &again.owner().unwrap()).await.unwrap();
        assert!(!root.path().join(".repair-recovery").exists());
        assert!(root.path().join("instance").is_dir());
    }

    /// A staged file that went missing names a way out the instance has:
    /// Repair for a pack linked to a platform, importing the file again for
    /// one imported from a file no platform publishes (Repair needs a link).
    #[tokio::test]
    async fn a_missing_staged_file_names_a_way_out_the_instance_has() {
        let root = legacy_root(&[], &[".setup/staging/instance"]);
        let target = packinfo::PackInfo {
            files: std::collections::HashMap::from([(
                "/mods/a.jar".to_string(),
                packinfo::FileHashes {
                    sha512: [1; 64],
                    md5: [1; 16],
                },
            )]),
        };
        let (digest, options_digest) = binding(&target, None).unwrap();
        let mut linked = config();
        linked.modpack = Some(info::ModpackInfo {
            modpack: pack("B"),
            locked: true,
        });
        let staged = root.path().join(".setup/staging");
        for (producer, installed, repairable) in [
            (Producer::Import, config(), false),
            (Producer::Reconstructed, config(), false),
            (Producer::Import, linked.clone(), true),
            (Producer::Create, linked.clone(), true),
            (Producer::Change, linked.clone(), true),
            (Producer::Repair, linked.clone(), true),
        ] {
            let mut session = Session::new(&installed, producer, Kind::Pack, None).unwrap();
            // What it installs never makes it repairable before it commits.
            session.pending_config = Some(schema::make_instance_config(linked.clone()).unwrap());
            session.target = Some(Target {
                packinfo: packinfo::make_packinfo(target.clone()).unwrap(),
                digest: digest.clone(),
                options_digest: options_digest.clone(),
                sources: BTreeMap::from([("/mods/a.jar".to_string(), "/mods/a.jar".to_string())]),
                skipped: Vec::new(),
            });
            let message = format!(
                "{:#}",
                session
                    .verify_sources(&staged, &root.path().join("instance"))
                    .await
                    .unwrap_err()
            );
            assert!(message.contains("mods/a.jar"), "{message}");
            assert_eq!(
                message.contains("Use Repair"),
                repairable,
                "{producer:?}: {message}"
            );
            assert_eq!(
                message.contains("import the modpack file again"),
                !repairable,
                "{producer:?}: {message}"
            );
        }
    }

    /// On a volume without hard links the apply moves staged files into
    /// place, as 2.0.40 did. A resumed apply counts one that is gone from
    /// staging while its live file (or twin) is there as published, and
    /// leaves that file to the planner. A missing source with nothing in its
    /// place, or one missing before the apply started, is still refused.
    #[tokio::test]
    async fn a_resumed_apply_counts_a_source_moved_into_place_as_published() {
        let root = legacy_root(&[], &[".setup/staging/instance"]);
        let staged = root.path().join(".setup/staging");
        let live = root.path().join("instance");
        let hashes = |seed: u8| packinfo::FileHashes {
            sha512: [seed; 64],
            md5: [seed; 16],
        };
        let target = packinfo::PackInfo {
            files: std::collections::HashMap::from([
                ("/mods/a.jar".to_string(), hashes(1)),
                ("/mods/off.jar".to_string(), hashes(2)),
            ]),
        };
        let (digest, options_digest) = binding(&target, None).unwrap();
        let mut session = Session::new(&config(), Producer::Change, Kind::Pack, None).unwrap();
        session.target = Some(Target {
            packinfo: packinfo::make_packinfo(target).unwrap(),
            digest,
            options_digest,
            sources: BTreeMap::from([
                ("/mods/a.jar".to_string(), "/mods/a.jar".to_string()),
                (
                    "/mods/off.jar".to_string(),
                    "/mods/off.jar.disabled".to_string(),
                ),
            ]),
            skipped: Vec::new(),
        });
        std::fs::create_dir_all(live.join("mods")).unwrap();
        // Edited since: the planner judges it, not this check.
        std::fs::write(live.join("mods/a.jar"), b"moved, then edited").unwrap();
        std::fs::write(live.join("mods/off.jar.disabled"), b"moved").unwrap();

        session.phase = Phase::Applying;
        let check = session.verify_sources(&staged, &live).await.unwrap();
        assert_eq!(
            check.published,
            [live.join("mods/a.jar"), live.join("mods/off.jar.disabled")]
        );
        assert!(check.changed.is_empty());

        session.phase = Phase::Resolved;
        let message = format!(
            "{:#}",
            session.verify_sources(&staged, &live).await.unwrap_err()
        );
        assert!(message.contains("mods/a.jar"), "{message}");

        session.phase = Phase::Applying;
        std::fs::remove_file(live.join("mods/a.jar")).unwrap();
        let message = format!(
            "{:#}",
            session.verify_sources(&staged, &live).await.unwrap_err()
        );
        assert!(message.contains("mods/a.jar"), "{message}");
        assert!(message.contains("went missing"), "{message}");
    }

    /// What Repair installs over a pending setup, and whether that setup may
    /// have changed installed content.
    #[test]
    fn interrupted_setups_and_their_repair_targets() {
        let root = legacy_root(&[], &[]);
        let mut pending = config();
        pending.modpack = Some(info::ModpackInfo {
            modpack: pack("B"),
            locked: true,
        });
        let mut session = Session::new(&config(), Producer::Change, Kind::Pack, None).unwrap();
        session.instance_id = Some(7);
        session.requested_pack =
            Some(serde_json::to_string(&super::super::PackVersionFile::from(pack("B"))).unwrap());
        session.pending_config = Some(schema::make_instance_config(pending).unwrap());
        for (phase, interrupted, target) in [
            (Phase::Reserved, false, None),
            (Phase::Resolved, false, None),
            (Phase::Applying, true, Some(pack("B"))),
            (Phase::Applied, true, Some(pack("B"))),
            (Phase::Committed, false, Some(pack("B"))),
        ] {
            session.phase = phase;
            assert_eq!(session.interrupted(root.path()), interrupted, "{phase:?}");
            assert_eq!(session.repair_target().unwrap(), target, "{phase:?}");
        }
        // An apply marker beside a pre-apply phase.
        session.phase = Phase::Resolved;
        std::fs::write(root.path().join(".setup/apply-started"), b"").unwrap();
        assert!(session.interrupted(root.path()));
        assert!(session.changed_files(root.path()));
        std::fs::remove_file(root.path().join(".setup/apply-started")).unwrap();
        // A reconstructed 2.0.40 install whose configuration already named
        // the pack it requested may have applied part of it.
        session.producer = Producer::Reconstructed;
        session.installed_config = schema::make_instance_config(
            schema::parse_instance_config(session.pending_config.as_ref().unwrap()).unwrap(),
        )
        .unwrap();
        assert!(session.interrupted(root.path()));
        assert_eq!(session.repair_target().unwrap(), None);
        // Unless 2.0.40 had finished its pack step, beside the apply records
        // it left with it: this build marks an apply only once it saved
        // Applying, so before that the marker is 2.0.40's.
        let setup = root.path().join(".setup");
        let applied = ["modpack-complete", "apply-started", "staging-packinfo.json"];
        for marker in applied {
            std::fs::write(setup.join(marker), b"").unwrap();
        }
        for phase in [Phase::Reserved, Phase::Resolved] {
            session.phase = phase;
            assert!(!session.interrupted(root.path()), "{phase:?}");
            assert!(!session.changed_files(root.path()), "{phase:?}");
        }
        session.phase = Phase::Applying;
        assert!(session.changed_files(root.path()));
        session.phase = Phase::Resolved;
        // Beside any other setup, the marker is its own.
        session.producer = Producer::Change;
        assert!(session.changed_files(root.path()));
        session.producer = Producer::Reconstructed;
        // Without modpack-complete, 2.0.40's apply never finished.
        std::fs::remove_file(setup.join("modpack-complete")).unwrap();
        assert!(session.interrupted(root.path()));
        for marker in &applied[1..] {
            std::fs::remove_file(setup.join(marker)).unwrap();
        }
        // ...or never switched the configuration, which it did before
        // applying anything (older builds marked it with staging.json)...
        let switched = session.installed_config.clone();
        session.installed_config = schema::make_instance_config(config()).unwrap();
        assert!(!session.interrupted(root.path()));
        std::fs::write(setup.join("staging.json"), b"[]").unwrap();
        assert!(session.interrupted(root.path()));
        std::fs::remove_file(setup.join("staging.json")).unwrap();
        std::fs::write(setup.join("apply-started"), b"").unwrap();
        assert!(session.interrupted(root.path()));
        std::fs::remove_file(setup.join("apply-started")).unwrap();
        // ...or the switch changed nothing: 2.0.40 repaired the configured
        // pack...
        session.installed_config = switched;
        std::fs::write(setup.join("repair"), b"{}").unwrap();
        assert!(!session.interrupted(root.path()));
        std::fs::remove_file(setup.join("repair")).unwrap();
        // ...or installed it for the first time and never reached the
        // switch, which gave it its game version; nothing records its files.
        let mut first = schema::parse_instance_config(&session.installed_config).unwrap();
        first.game_configuration.version = None;
        session.installed_config = schema::make_instance_config(first).unwrap();
        assert!(!session.interrupted(root.path()));
        std::fs::write(root.path().join("packinfo.json"), b"{}").unwrap();
        assert!(session.interrupted(root.path()));
        std::fs::remove_file(root.path().join("packinfo.json")).unwrap();
        // A repair that replaced an interrupted setup continues it: never
        // cancelled, and a Repair over it installs what it requested. It
        // changed nothing itself until its own apply starts.
        let mut repair = Session::new(&config(), Producer::Repair, Kind::Pack, None).unwrap();
        repair.instance_id = Some(7);
        repair.requested_pack =
            Some(serde_json::to_string(&super::super::PackVersionFile::from(pack("B"))).unwrap());
        assert!(!repair.interrupted(root.path()));
        assert_eq!(repair.repair_target().unwrap(), None);
        assert!(repair.cancel_reason(&config()).is_none());
        repair.replaces_interrupted = Some("op".into());
        for phase in [Phase::Reserved, Phase::Resolved] {
            repair.phase = phase;
            assert!(repair.interrupted(root.path()), "{phase:?}");
            assert!(!repair.changed_files(root.path()), "{phase:?}");
        }
        assert_eq!(repair.repair_target().unwrap(), Some(pack("B")));
        assert!(
            repair
                .cancel_reason(&config())
                .unwrap()
                .contains("can't be cancelled")
        );
        repair.phase = Phase::Applying;
        assert!(repair.changed_files(root.path()));
        // Once it committed, its pack is installed.
        repair.phase = Phase::Committed;
        assert!(!repair.interrupted(root.path()));
        assert!(!repair.changed_files(root.path()));
    }

    /// The same question for a `.setup` without a readable record, given
    /// the configured pack: what 2.0.40 left at each step of an install or
    /// update (see `unrecorded_interrupted`).
    #[test]
    fn unrecorded_setups_changed_files_once_their_configuration_may_have_switched() {
        let a = pack("A");
        let b = pack("B");
        let requested =
            serde_json::to_string(&super::super::PackVersionFile::from(b.clone())).unwrap();
        let request = (".setup/change-pack-version.json", requested.as_bytes());
        let archive = (".setup/modrinth", b"archive".as_slice());
        let staged = (".setup/staging/instance/mods/new.jar", b"new".as_slice());
        let overrides = ".setup/modpack-skip-overrides";
        let staging_record = (".setup/staging-packinfo.json", b"[]".as_slice());
        let started = (".setup/apply-started", b"".as_slice());
        let target = ("tmp-packinfo.json", b"{}".as_slice());
        let complete = (".setup/modpack-complete", b"".as_slice());
        let repair = (".setup/repair", b"{}".as_slice());
        let record = ("packinfo.json", b"{}".as_slice());
        let configured = |pack: &info::Modpack| {
            let mut config = config();
            config.modpack = Some(info::ModpackInfo {
                modpack: pack.clone(),
                locked: true,
            });
            config
        };
        // A new install of `pack` whose game version 2.0.40 hadn't learned.
        let installing = |pack: &info::Modpack| {
            let mut config = first_install_config();
            config.modpack = Some(info::ModpackInfo {
                modpack: pack.clone(),
                locked: true,
            });
            config
        };
        for (shape, files, dirs, config, changed) in [
            ("empty", vec![], vec![], configured(&a), false),
            ("request", vec![request], vec![], configured(&a), false),
            (
                "archive",
                vec![request, archive],
                vec![],
                configured(&a),
                false,
            ),
            ("complete", vec![complete], vec![], configured(&b), false),
            (
                "empty staging",
                vec![],
                vec![".setup/staging/instance"],
                config(),
                false,
            ),
            // An update to B that failed while downloading: A is configured
            // and untouched.
            (
                "update downloading",
                vec![request, archive, staged],
                vec![overrides],
                configured(&a),
                false,
            ),
            // The same once 2.0.40 switched the configuration to B.
            (
                "update switched",
                vec![request, archive, staged],
                vec![overrides],
                configured(&b),
                true,
            ),
            // The configured pack staged without a repair marker: an update
            // that may have switched the configuration.
            (
                "staged",
                vec![request, staged],
                vec![],
                configured(&b),
                true,
            ),
            // A repair of the configured pack that failed while downloading:
            // its switch changed nothing.
            (
                "repair downloading",
                vec![request, archive, staged, repair],
                vec![overrides],
                configured(&b),
                false,
            ),
            (
                "repair marker that is not a file",
                vec![request, staged],
                vec![".setup/repair"],
                configured(&b),
                true,
            ),
            (
                "repair staging record",
                vec![request, archive, staged, repair, staging_record],
                vec![],
                configured(&b),
                true,
            ),
            (
                "repair applying",
                vec![request, staged, repair, started],
                vec![],
                configured(&b),
                true,
            ),
            // A first install that failed while downloading, before the
            // switch gave it a game version.
            (
                "first install downloading",
                vec![request, archive, staged],
                vec![overrides],
                installing(&b),
                false,
            ),
            (
                "first install beside an installed record",
                vec![request, archive, staged, record],
                vec![overrides],
                installing(&b),
                true,
            ),
            (
                "first install applying",
                vec![request, staged, staging_record, target],
                vec![],
                installing(&b),
                true,
            ),
            (
                "staged without request",
                vec![staged],
                vec![],
                configured(&a),
                true,
            ),
            (
                "overrides without request",
                vec![],
                vec![overrides],
                configured(&a),
                true,
            ),
            (
                "staging record",
                vec![request, archive, staged, staging_record],
                vec![],
                configured(&a),
                true,
            ),
            (
                "older staging record",
                vec![request, staged, (".setup/staging.json", b"[]")],
                vec![],
                configured(&a),
                true,
            ),
            (
                "apply started",
                vec![request, started],
                vec![],
                configured(&a),
                true,
            ),
            (
                "staging-packinfo",
                vec![staging_record],
                vec![],
                configured(&b),
                true,
            ),
            ("target", vec![target], vec![], configured(&b), true),
            // Applied and promoted, then the game install failed.
            (
                "applied",
                vec![request, archive, complete, started, staging_record],
                vec![overrides],
                configured(&b),
                false,
            ),
            (
                "applied, target left",
                vec![request, archive, complete, started, target],
                vec![],
                configured(&b),
                true,
            ),
            (
                "complete beside staging",
                vec![request, archive, complete, staged],
                vec![],
                configured(&b),
                true,
            ),
            (
                "unreadable request",
                vec![(".setup/change-pack-version.json", b"{".as_slice()), staged],
                vec![],
                configured(&a),
                true,
            ),
        ] {
            let root = legacy_root(&files, &dirs);
            assert_eq!(
                unrecorded_interrupted(root.path(), &config),
                changed,
                "{shape}"
            );
        }
        // A `.setup` that is not a folder can't be checked.
        let root = legacy_root(&[], &[]);
        std::fs::remove_dir(root.path().join(".setup")).unwrap();
        std::fs::write(root.path().join(".setup"), b"").unwrap();
        assert!(unrecorded_interrupted(root.path(), &configured(&a)));
    }
}

/// A data folder on a VirtualBox shared folder answers every folder sync
/// with EINVAL, and one on a macOS network share refuses std's F_FULLFSYNC.
/// 2.0.40 changed an instance's game version in Settings there without a
/// folder sync, so its setup record is still written and synced there. A
/// real error still fails.
#[cfg(all(test, unix))]
mod sync_refusing_volume_tests {
    use super::*;
    use crate::util::durable::test_hooks::{FileSyncs, FolderSyncs};

    #[tokio::test]
    async fn a_setup_record_is_written_where_folders_cannot_be_synced() {
        let config = schema::parse_instance_config(r#"{"_version":"1","name":"Synthetic","game_configuration":{"version":{"release":"1.20.1","modloaders":[]}}}"#).unwrap();
        let session = Session::new(&config, Producer::Settings, Kind::VersionOnly, None).unwrap();
        let root = tempfile::tempdir().unwrap();
        let setup = root.path().join(".setup");
        std::fs::create_dir(&setup).unwrap();
        let files = FileSyncs::refusing_full_flush();
        {
            let folders = FolderSyncs::failing_with(root.path(), libc::EINVAL);
            session.create(root.path()).await.unwrap();
            assert_eq!(folders.synced(), [setup.clone(), root.path().to_owned()]);
        }
        assert!(files.synced(&setup.join(RECORD)));
        assert_eq!(
            Session::load(root.path()).await.unwrap().unwrap().operation,
            session.operation
        );

        let _folders = FolderSyncs::failing_with(root.path(), libc::EIO);
        assert!(sync_dir(&setup).await.is_err());
    }
}
