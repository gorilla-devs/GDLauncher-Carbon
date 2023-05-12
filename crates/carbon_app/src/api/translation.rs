use rspc::Type;
use serde::Serialize;

#[derive(Debug, Type, Serialize, Clone, PartialEq)]
pub enum Translation {
    #[cfg(test)]
    Test,
    InstanceTaskLaunch(String),
    InstanceTaskPrepare(String),
    InstanceTaskLaunchWaiting,
    InstanceTaskLaunchRequestVersions,
    InstanceTaskLaunchDownloadFiles,
    InstanceTaskLaunchExtractNatives,
    InstanceTaskLaunchRunForgeProcessors,
}
