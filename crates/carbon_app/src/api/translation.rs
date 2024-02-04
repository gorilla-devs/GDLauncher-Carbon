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
    InstanceTaskLaunchRequestVersions,
    InstanceTaskLaunchRequestModpack,
    InstanceTaskLaunchDownloadModpack,
    InstanceTaskLaunchDownloadModpackFiles,
    InstanceTaskLaunchExtractModpackFiles,
    InstanceTaskLaunchDownloadAddonMetadata,
    InstanceTaskLaunchApplyStagedPatches,
    InstanceTaskLaunchDownloadJava,
    InstanceTaskLaunchExtractJava,
    InstanceTaskLaunchWaitDownloadFiles,
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
    FinalizingImport,
    InstanceImportLegacyBadConfigFile,
    InstanceImportCfZipMalformed,
    InstanceImportCfZipMissingManifest,
    InstanceImportCfZipMalformedManifest,
    InstanceImportCfZipNotMinecraftModpack,
    InstanceImportMrpackMalformed,
    InstanceImportMrpackMissingManifest,
    InstanceImportMrpackMalformedManifest,
    InstanceExport,
    InstanceExportScanningMods,
    InstanceExportCacheMods,
    InstanceExportCalculateSize,
    InstanceExportCreatingBundle,
}
