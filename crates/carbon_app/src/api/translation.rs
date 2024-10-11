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
