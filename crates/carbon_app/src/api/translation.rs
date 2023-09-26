use rspc::Type;
use serde::Serialize;

#[derive(Debug, Type, Serialize, Clone, PartialEq)]
#[serde(tag = "translation", content = "args")]
pub enum Translation {
    #[cfg(test)]
    Test,
    ModCacheTaskUpdate,
    ModCacheTaskUpdateScanFiles,
    ModCacheTaskUpdateQueryApis,
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
    InstanceTaskLaunchDownloadJava,
    InstanceTaskLaunchExtractJava,
    InstanceTaskLaunchDownloadFiles,
    InstanceTaskLaunchExtractNatives,
    InstanceTaskReconstructAssets,
    InstanceTaskLaunchRunForgeProcessors,
    InstanceTaskInstallMod {
        mod_name: String,
        instance_name: String,
    },
    InstanceTaskInstallModDownloadFile,
    FinalizingImport,
    InstanceImportLegacyBadConfigFile,
}
