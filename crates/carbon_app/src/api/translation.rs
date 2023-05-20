use rspc::Type;
use serde::Serialize;

#[derive(Debug, Type, Serialize, Clone, PartialEq)]
#[serde(tag = "translation", content = "args")]
pub enum Translation {
    #[cfg(test)]
    Test,
    InstanceTaskLaunch {
        name: String,
    },
    InstanceTaskPrepare {
        name: String,
    },
    InstanceTaskLaunchWaiting,
    InstanceTaskLaunchRequestVersions,
    InstanceTaskLaunchRequestModpack,
    InstanceTaskLaunchDownloadModpackFiles,
    InstanceTaskLaunchExtractModpackFiles,
    InstanceTaskLaunchDownloadAddonMetadata,
    InstanceTaskLaunchDownloadFiles,
    InstanceTaskLaunchExtractNatives,
    InstanceTaskLaunchRunForgeProcessors,
}
