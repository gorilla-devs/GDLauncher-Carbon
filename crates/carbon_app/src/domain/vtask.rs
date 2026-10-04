use std::sync::Arc;

use crate::api::translation::Translation;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub struct VisualTaskId(pub i32);

#[derive(Debug, PartialEq)]
pub struct Task {
    pub id: VisualTaskId,
    pub name: Translation,
    pub progress: Progress,
    pub downloaded: u32,
    pub download_total: u32,
    pub active_subtasks: Vec<Subtask>,
}

#[derive(Debug)]
pub enum Progress {
    Indeterminate,
    Known(f32),
    Failed(Arc<anyhow::Error>),
}

#[derive(Debug, PartialEq)]
pub struct Subtask {
    pub name: Translation,
    pub progress: SubtaskProgress,
}

#[derive(Debug, PartialEq)]
pub enum SubtaskProgress {
    Download { downloaded: u32, total: u32 },
    Item { current: u32, total: u32 },
    Opaque,
}

impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Progress::Indeterminate, Progress::Indeterminate) => true,
            (Progress::Known(a), Progress::Known(b)) if a == b => true,
            (Progress::Failed(_), Progress::Failed(_)) => true,
            _ => false,
        }
    }
}

/// Something a task did differently from what was asked, reported to the user
/// after the task succeeds. Stored in a setup's record until that setup
/// finishes (see `modpack::session::Session::warnings`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TaskWarning {
    /// A required dependency was left out because it cannot be installed.
    SkippedDependency {
        /// The project's name, when the platform still knows it.
        name: Option<String>,
        /// Where the requirement was declared, to name it when `name` is unknown.
        platform: DependencyPlatform,
        /// The platform's project id, or the file name of a dependency declared
        /// by file name only.
        project_id: String,
        reason: SkippedDependencyReason,
    },
    /// A required dependency was installed at another release than the one
    /// the addon pins, because the pinned one is not for this game and loader.
    ReplacedPinnedVersion {
        name: String,
        project_id: String,
        /// Version numbers as the platform shows them.
        pinned: String,
        installed: String,
    },
    /// Two addons in one installation ask for different versions of a
    /// dependency; only one of them can be installed.
    ConflictingVersions {
        name: String,
        platform: DependencyPlatform,
        project_id: String,
        /// The version number that was installed, as the platform shows it.
        installed: String,
    },
    /// A modpack's files under a link in the server folder (a `config/`
    /// shared with another server) were not installed, as GDLauncher does
    /// not write through links.
    PackFilesSkippedAtLink {
        /// The link, as a `/`-separated path inside the server folder.
        path: String,
    },
    /// A world was installed as a new save under another name, because a
    /// save already has its own (an earlier install of the same map, or a
    /// map called "New World"). The existing save is not touched.
    WorldNameTaken {
        /// The world's own folder name.
        name: String,
        /// The new save's name, with a "(copy N)" suffix.
        saved_as: String,
    },
    /// A modpack version change kept files of the version it replaced that it
    /// could not check against that version, rather than risk removing the
    /// user's own: every file in place when the version itself could not be
    /// read. A file it kept may now sit beside the new version's copy.
    PreviousPackFilesKept {
        /// Where the replaced version was published.
        platform: DependencyPlatform,
        reason: PreviousPackFilesReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DependencyPlatform {
    Curseforge,
    Modrinth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PreviousPackFilesReason {
    /// The platform no longer serves the replaced version.
    VersionUnavailable,
    /// The replaced version's files could not be checked for another reason,
    /// such as an archive format the launcher does not know, or a folder that
    /// won't take their record.
    VersionUnreadable,
    /// Some of the replaced version's files could not be checked, for example
    /// because the platform no longer serves them.
    SomeFilesUnchecked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkippedDependencyReason {
    /// No version for this Minecraft version and loader.
    NoCompatibleVersion,
    /// The author only allows downloads from the CurseForge website.
    NotDownloadable,
    /// The platform no longer serves it, or it is not on the platform at all.
    Unavailable,
    /// Its authors mark it as not running on servers.
    NotForServers,
    /// It was installed when the installation started, and was removed from
    /// the server before the installation finished.
    RemovedDuringInstall,
    /// It is installed but disabled. It is left as it is, so the addon may not
    /// load until the user enables it.
    Disabled,
}

/// Warnings of a task that succeeded, kept until the UI acknowledges them.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskWarnings {
    pub id: VisualTaskId,
    pub name: Translation,
    /// What the warnings are about when the task name does not say it plainly,
    /// for example the project title of an addon whose version is named "4.0".
    pub subject: Option<String>,
    pub warnings: Vec<TaskWarning>,
}
