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
        })
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
