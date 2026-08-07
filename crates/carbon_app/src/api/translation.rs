use serde::Serialize;
use specta::Type;

#[derive(Debug, Type, Serialize, Clone, PartialEq)]
#[serde(tag = "translation", content = "args")]
pub enum Translation {
    #[cfg(test)]
    Test,
    InstanceTaskDeleting,
    InstanceTaskLaunch {
        name: String,
    },
    InstanceTaskPrepare {
        name: String,
    },
    InstanceTaskCheckPackOrigin {
        name: String,
    },
    InstanceTaskCheckPackOriginVersions,
    InstanceTaskLaunchRequestVersions,
    InstanceTaskLaunchRequestModpack,
    InstanceTaskLaunchDownloadModpack,
    InstanceTaskLaunchDownloadModpackFiles,
    InstanceTaskLaunchExtractModpackFiles,
    InstanceTaskLaunchRequestAddonMetadata,
    InstanceTaskLaunchApplyStagedPatches,
    InstanceTaskLaunchDownloadJava,
    InstanceTaskLaunchExtractJava,
    InstanceTaskRequestModloaderInfo,
    InstanceTaskRequestMinecraftFiles,
    InstanceTaskLaunchCheckingFiles,
    InstanceTaskLaunchDownloadFiles,
    InstanceTaskGeneratingPackInfo,
    InstanceTaskFillCache,
    InstanceTaskLaunchExtractNatives,
    InstanceTaskReconstructAssets,
    InstanceTaskLaunchRunForgeProcessors,
    InstanceTaskLaunchRunNeoforgeProcessors,
    InstanceTaskInstallMod {
        mod_name: String,
        instance_name: String,
    },
    InstanceTaskInstallModDownloadFile,
    ServerTaskInstall {
        server_name: String,
    },
    ServerTaskInstallFromModpack {
        server_name: String,
    },
    ServerTaskDownloadServerPack {
        server_name: String,
    },
    ServerTaskExtractServerPack,
    ServerTaskDownloadModpackFiles,
    ServerTaskExtractModpackOverrides,
    ServerTaskDownloadServerJar,
    ServerTaskInstallModloader,
    ServerTaskInstallMod {
        mod_name: String,
        server_name: String,
    },
    ServerTaskInstallModDownloadFile,
    FinalizingImport,
    InstanceImportLegacyBadConfigFile,
    InstanceImportCfZipMalformed,
    InstanceImportCfZipMissingManifest,
    InstanceImportCfZipMalformedManifest,
    InstanceImportCfZipNotMinecraftModpack,
    InstanceImportMrpackMalformed,
    InstanceImportMrpackMissingManifest,
    InstanceImportMrpackMalformedManifest,
    InstanceImportGdlpackMalformed,
    InstanceImportGdlpackMissingManifest,
    InstanceImportGdlpackMalformedManifest,
    InstanceExport,
    InstanceExportCreatingBundle,
    InstanceExportCalculateSize,
    InstanceExportScanningMods,
    InstanceImportShareCode,
    CacheTaskLocal {
        instance_name: String,
    },
    CacheTaskCurseForge {
        instance_name: String,
    },
    CacheTaskModrinth {
        instance_name: String,
    },
    CacheSubtaskScanningFiles,
    CacheSubtaskQueryingPlatform {
        platform: String,
    },
    CacheSubtaskDownloadingImages,
    CacheSubtaskFinalizingCache,
    CacheStatusTitle,
    CacheStatusNoActiveTasks,
    CacheStatusIdle,
    CacheStatusRunning,
    CacheStatusFailed,
    CacheStatusActiveTasks {
        count: u32,
    },
    CacheStatusActiveCount {
        count: u32,
    },
    CacheStatusOperationsInfo,
    CacheErrorGeneric,
    CacheSubtaskScanningDirectories,
    CacheSubtaskQueryingPlatformDatabase {
        platform: String,
    },
    CacheSubtaskDownloadingThumbnails,
    CacheSubtaskProcessingFiles,
    CacheProgressDirectories {
        current: u32,
        total: u32,
    },
    CacheProgressFiles {
        current: u32,
        total: u32,
    },
    CacheCleanup,
}
