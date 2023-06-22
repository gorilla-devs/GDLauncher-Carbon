// This file was generated by [rspc](https://github.com/oscartbeaumont/rspc). Do not edit this file manually.

export type Procedures = {
    queries: 
        { key: "account.enroll.getStatus", input: never, result: EnrollmentStatus | null } | 
        { key: "account.getAccountStatus", input: string, result: AccountStatus | null } | 
        { key: "account.getAccounts", input: never, result: AccountEntry[] } | 
        { key: "account.getActiveUuid", input: never, result: string | null } | 
        { key: "account.getHead", input: string, result: null } | 
        { key: "echo", input: string, result: string } | 
        { key: "getAppVersion", input: never, result: string } | 
        { key: "instance.getDefaultGroup", input: never, result: number } | 
        { key: "instance.getGroups", input: never, result: ListGroup[] } | 
        { key: "instance.getInstanceDetails", input: InstanceId, result: InstanceDetails } | 
        { key: "instance.getInstancesUngrouped", input: never, result: UngroupedInstance[] } | 
        { key: "instance.getLogs", input: never, result: GameLogEntry[] } | 
        { key: "java.getAvailableJavas", input: never, result: { [key: number]: FEJavaComponent[] } } | 
        { key: "java.getManagedArch", input: never, result: FEManagedJavaArch[] } | 
        { key: "java.getManagedOS", input: never, result: FEManagedJavaOs[] } | 
        { key: "java.getManagedVendors", input: never, result: FEVendor[] } | 
        { key: "java.getManagedVersionsByVendor", input: FEVendor, result: FEManagedJavaOsMap } | 
        { key: "java.getSetupManagedJavaProgress", input: never, result: FEManagedJavaSetupProgress } | 
        { key: "java.getSystemJavaProfiles", input: never, result: FESystemJavaProfile[] } | 
        { key: "mc.getFabricVersions", input: never, result: FEModdedManifest } | 
        { key: "mc.getForgeVersions", input: never, result: FEModdedManifest } | 
        { key: "mc.getMinecraftVersions", input: never, result: ManifestVersion[] } | 
        { key: "mc.getQuiltVersions", input: never, result: FEModdedManifest } | 
        { key: "modplatforms.curseforgeGetCategories", input: never, result: FECategoriesResponse } | 
        { key: "modplatforms.curseforgeGetFiles", input: FEFilesParameters, result: FEFilesResponse } | 
        { key: "modplatforms.curseforgeGetMod", input: FEModParameters, result: FEModResponse } | 
        { key: "modplatforms.curseforgeGetModDescription", input: FEModDescriptionParameters, result: FEModDescriptionResponse } | 
        { key: "modplatforms.curseforgeGetModFile", input: FEModFileParameters, result: FEModFileResponse } | 
        { key: "modplatforms.curseforgeGetModFileChangelog", input: FEModFileChangelogParameters, result: FEModFileChangelogResponse } | 
        { key: "modplatforms.curseforgeGetModFiles", input: FEModFilesParameters, result: FEModFilesResponse } | 
        { key: "modplatforms.curseforgeGetMods", input: FEModsParameters, result: FEModsResponse } | 
        { key: "modplatforms.curseforgeSearch", input: FEModSearchParameters, result: FEModSearchResponse } | 
        { key: "settings.getSettings", input: never, result: FESettings } | 
        { key: "systeminfo.getTotalRAM", input: never, result: string } | 
        { key: "systeminfo.getUsedRAM", input: never, result: string } | 
        { key: "vtask.getTask", input: TaskId, result: Task | null } | 
        { key: "vtask.getTasks", input: never, result: Task[] },
    mutations: 
        { key: "account.deleteAccount", input: string, result: null } | 
        { key: "account.enroll.begin", input: never, result: null } | 
        { key: "account.enroll.cancel", input: never, result: null } | 
        { key: "account.enroll.finalize", input: never, result: null } | 
        { key: "account.refreshAccount", input: string, result: null } | 
        { key: "account.setActiveUuid", input: string | null, result: null } | 
        { key: "instance.createGroup", input: string, result: GroupId } | 
        { key: "instance.createInstance", input: CreateInstance, result: InstanceId } | 
        { key: "instance.deleteGroup", input: GroupId, result: null } | 
        { key: "instance.deleteInstance", input: InstanceId, result: null } | 
        { key: "instance.deleteLog", input: GameLogId, result: null } | 
        { key: "instance.deleteMod", input: InstanceMod, result: null } | 
        { key: "instance.disableMod", input: InstanceMod, result: null } | 
        { key: "instance.duplicateInstance", input: DuplicateInstance, result: InstanceId } | 
        { key: "instance.enableMod", input: InstanceMod, result: null } | 
        { key: "instance.installMod", input: InstallMod, result: TaskId } | 
        { key: "instance.killInstance", input: InstanceId, result: null } | 
        { key: "instance.launchInstance", input: InstanceId, result: null } | 
        { key: "instance.loadIconUrl", input: string, result: null } | 
        { key: "instance.moveGroup", input: MoveGroup, result: null } | 
        { key: "instance.moveInstance", input: MoveInstance, result: null } | 
        { key: "instance.openInstanceFolder", input: OpenInstanceFolder, result: null } | 
        { key: "instance.prepareInstance", input: InstanceId, result: null } | 
        { key: "instance.setFavorite", input: SetFavorite, result: null } | 
        { key: "instance.updateInstance", input: UpdateInstance, result: null } | 
        { key: "java.deleteJavaVersion", input: string, result: null } | 
        { key: "java.setupManagedJava", input: FEManagedJavaSetupArgs, result: string } | 
        { key: "java.updateSystemJavaProfilePath", input: FEUpdateSystemJavaProfileArgs, result: null } | 
        { key: "metrics.sendEvent", input: FEEvent, result: null } | 
        { key: "metrics.sendPageview", input: FEPageview, result: null } | 
        { key: "settings.setSettings", input: FESettingsUpdate, result: null } | 
        { key: "vtask.dismissTask", input: TaskId, result: null },
    subscriptions: 
        { key: "invalidateQuery", input: never, result: InvalidationEvent }
};

export type Task = { name: Translation; progress: Progress; downloaded: number; download_total: number; active_subtasks: Subtask[] }

export type MoveGroup = { group: GroupId; before: GroupId | null }

export type DeviceCode = { userCode: string; verificationUri: string; expiresAt: string }

export type MemoryRange = { min_mb: number; max_mb: number }

export type ModLoader = { type_: ModLoaderType; version: string }

export type MoveInstanceTarget = { BeforeInstance: InstanceId } | { BeginningOfGroup: GroupId } | { EndOfGroup: GroupId }

export type SubtaskProgress = { download: { downloaded: number; total: number } } | { item: { current: number; total: number } } | "opaque"

export type GameVersion = { Standard: StandardVersion }

export type GroupId = number

export type AccountStatus = "ok" | "expired" | "refreshing" | "invalid"

export type FEModSearchParametersQuery = { gameId: number; searchFilter: string | null; gameVersion: string | null; categoryId: number | null; sortOrder: FEModSearchSortOrder | null; sortField: FEModSearchSortField | null; classId: FEClassId | null; modLoaderType: FEModLoaderType | null; gameVersionTypeId: number | null; authorId: number | null; slug: string | null; index: number | null; pageSize: number | null }

export type InstanceDetails = { name: string; favorite: boolean; version: string | null; modpack: Modpack | null; global_java_args: boolean; extra_java_args: string | null; memory: MemoryRange | null; last_played: string; seconds_played: number; modloaders: ModLoader[]; notes: string; state: LaunchState; mods: Mod[] }

export type FEJavaComponent = { id: string; path: string; version: string; type: FEJavaComponentType; isValid: boolean }

export type FEManagedJavaArchMap = { [key: FEManagedJavaArch]: FEManagedJavaVersion[] }

export type ManifestVersion = { id: string; type: McType }

export type ListGroup = { id: GroupId; name: string; instances: ListInstance[] }

export type Mod = { id: string; filename: string; enabled: boolean; modloaders: ModLoaderType[]; metadata: ModFileMetadata }

export type Set<T> = { Set: T }

export type FEManagedJavaOsMap = { [key: FEManagedJavaOs]: FEManagedJavaArchMap }

export type FECategoriesResponse = { data: FECategory[]; pagination: FEPagination | null }

export type FEModDescriptionResponse = { data: string; pagination: FEPagination | null }

export type FEEventName = "AppClosed"

export type ConfigurationParseError = { type_: ConfigurationParseErrorType; message: string; line: number; config_text: string }

export type Modpack = { Curseforge: CurseforgeModpack }

export type FESystemJavaProfile = { name: FESystemJavaProfileName; javaId: string | null }

export type FEModFilesParametersQuery = { gameVersion?: string | null; modLoaderType?: FEModLoaderType | null; gameVersionTypeId?: number | null; index?: number | null; pageSize?: number | null }

export type FEFileReleaseType = "stable" | "beta" | "alpha"

export type InvalidationEvent = { key: string; args: any | null }

export type FEManagedJavaVersion = { id: string; name: string; downloadUrl: string; javaVersion: string }

export type FEModsParameters = { body: FEModsParametersBody }

export type FEModFileChangelogParameters = { modId: number; fileId: number }

export type FEFileIndex = { gameVersion: string; fileId: number; filename: string; releaseType: FEFileReleaseType; gameVersionTypeId: number | null; modLoader: FEModLoaderType | null }

export type FEModResponse = { data: FEMod; pagination: FEPagination | null }

export type FEMod = { id: number; gameId: number; name: string; slug: string; links: FEModLinks; summary: string; status: FEModStatus; downloadCount: number; isFeatured: boolean; primaryCategoryId: number; categories: FECategory[]; classId: number | null; authors: FEModAuthor[]; logo: FEModAsset; screenshots: FEModAsset[]; mainFileId: number; latestFiles: FEFile[]; latestFilesIndexes: FEFileIndex[]; dateCreated: string; dateModified: string; dateReleased: string; allowModDistribution: boolean | null; gamePopularityRank: number; isAvailable: boolean; thumbsUpCount: number }

export type FEVendor = "azul"

export type UpdateInstance = { instance: InstanceId; name?: Set<string> | null; use_loaded_icon?: Set<boolean> | null; notes?: Set<string> | null; version?: Set<string> | null; modloader?: Set<ModLoader | null> | null; global_java_args?: Set<boolean> | null; extra_java_args?: Set<string | null> | null; memory?: Set<MemoryRange | null> | null }

export type XboxError = "noAccount" | "xboxServicesBanned" | "adultVerificationRequired" | "childAccount" | { unknown: number }

export type UngroupedInstance = ({ id: InstanceId; name: string; favorite: boolean; status: ListInstanceStatus }) & { favorite: boolean }

export type StandardVersion = { release: string; modloaders: ModLoader[] }

export type FEPageview = { path: string }

export type FEManagedJavaOs = "windows" | "linux" | "macOs"

export type FEUpdateSystemJavaProfileArgs = { profileName: FESystemJavaProfileName; javaId: string }

export type CreateInstance = { group: GroupId; name: string; use_loaded_icon: boolean; version: CreateInstanceVersion; notes: string }

export type FEModFileChangelogResponse = { data: string; pagination: FEPagination | null }

export type FEFile = { id: number; gameId: number; modId: number; isAvailable: boolean; displayName: string; fileName: string; releaseType: FEFileReleaseType; fileStatus: FEFileStatus; hashes: FEFileHash[]; fileDate: string; fileLength: number; downloadCount: number; downloadUrl: string | null; gameVersions: string[]; sortableGameVersions: FESortableGameVersion[]; dependencies: FEFileDependency[]; exposeAsAlternative: boolean | null; parentProjectFileId: number | null; alternateFileId: number | null; isServerPack: boolean | null; serverPackFileId: number | null; isEarlyAccessContent: boolean | null; earlyAccessEndDate: string | null; fileFingerprint: string; modules: FEFileModule[] }

export type InstallMod = { instance_id: InstanceId; project_id: number; file_id: number }

export type TaskId = number

export type FEModLoaderType = "forge" | "cauldron" | "liteLoader" | "fabric" | "quilt"

export type ListInstanceStatus = { Valid: ValidListInstance } | { Invalid: InvalidListInstance }

export type FEModParameters = { modId: number }

export type FESettingsUpdate = { theme?: string | null; language?: string | null; reducedMotion?: boolean | null; discordIntegration?: boolean | null; releaseChannel?: string | null; concurrentDownloads?: number | null; showNews?: boolean | null; xmx?: number | null; xms?: number | null; isFirstLaunch?: boolean | null; startupResolution?: string | null; javaCustomArgs?: string | null; autoManageJava?: boolean | null }

export type FEJavaComponentType = "local" | "managed" | "custom"

export type EnrollmentError = "deviceCodeExpired" | { xboxAccount: XboxError } | "noGameOwnership" | "noGameProfile"

export type McType = "old_alpha" | "old_beta" | "release" | "snapshot"

export type FEFileRelationType = "embeddedLibrary" | "optionalDependency" | "requiredDependency" | "tool" | "incompatible" | "include"

export type FEModStatus = "new" | "changesRequired" | "underSoftReview" | "approved" | "rejected" | "changesMade" | "inactive" | "abandoned" | "deleted" | "underReview"

export type AccountEntry = { username: string; uuid: string; lastUsed: string; type: AccountType }

export type FEModSearchSortOrder = "ascending" | "descending"

export type FEModSearchSortField = "featured" | "popularity" | "lastUpdated" | "name" | "author" | "totalDownloads" | "category" | "gameVersion"

export type FEModDescriptionParameters = { modId: number }

export type EnrollmentStatus = "requestingCode" | { pollingCode: DeviceCode } | "queryingAccount" | { complete: AccountEntry } | { failed: EnrollmentError }

export type Translation = { translation: "ModCacheTaskUpdate" } | { translation: "ModCacheTaskUpdateScanFiles" } | { translation: "ModCacheTaskUpdateQueryApis" } | { translation: "InstanceTaskLaunch"; args: { name: string } } | { translation: "InstanceTaskPrepare"; args: { name: string } } | { translation: "InstanceTaskLaunchWaiting" } | { translation: "InstanceTaskLaunchRequestVersions" } | { translation: "InstanceTaskLaunchRequestModpack" } | { translation: "InstanceTaskLaunchDownloadModpackFiles" } | { translation: "InstanceTaskLaunchExtractModpackFiles" } | { translation: "InstanceTaskLaunchDownloadAddonMetadata" } | { translation: "InstanceTaskLaunchInstallJava" } | { translation: "InstanceTaskLaunchDownloadFiles" } | { translation: "InstanceTaskLaunchExtractNatives" } | { translation: "InstanceTaskLaunchRunForgeProcessors" } | { translation: "InstanceTaskInstallMod"; args: { mod_name: string; instance_name: string } } | { translation: "InstanceTaskInstallModDownloadFile" }

export type CurseforgeModpack = { project_id: number; file_id: number }

export type FEFileModule = { name: string; fingerprint: string }

export type InstanceMod = { instance_id: InstanceId; mod_id: string }

export type FEFileDependency = { modId: number; relationType: FEFileRelationType }

export type InvalidListInstance = "JsonMissing" | { JsonError: ConfigurationParseError } | { Other: string }

export type FEPagination = { index: number; pageSize: number; resultCount: number; totalCount: number }

export type ConfigurationParseErrorType = "Syntax" | "Data" | "Eof"

export type ModpackPlatform = "Curseforge"

export type AccountType = "microsoft" | "offline"

export type FEFilesParameters = { body: FEFilesParametersBody }

export type ValidListInstance = { mc_version: string | null; modloader: ModLoaderType | null; modpack_platform: ModpackPlatform | null; state: LaunchState }

export type SetFavorite = { instance: InstanceId; favorite: boolean }

export type FEModFileParameters = { modId: number; fileId: number }

export type ModFileMetadata = { modid: string; name: string | null; version: string | null; description: string | null; authors: string | null; modloaders: ModLoaderType[] | null }

export type FEModdedManifestLoaderVersion = { id: string }

export type FEFileStatus = "processing" | "changesRequired" | "underReview" | "approved" | "rejected" | "malwareDetected" | "deleted" | "archived" | "testing" | "released" | "readyForReview" | "deprecated" | "baking" | "awaitingPublishing" | "failedPublishing"

export type FEModFilesParameters = { modId: number; query: FEModFilesParametersQuery }

export type FEHashAlgo = "sha1" | "md5"

export type FEEvent = { name: FEEventName; properties: { [key: string]: string } }

export type InstanceId = number

export type FEModsResponse = { data: FEMod[]; pagination: FEPagination | null }

export type FEModdedManifest = { gameVersions: FEModdedManifestVersion[] }

export type LaunchState = { Inactive: { failed_task: TaskId | null } } | { Preparing: TaskId } | { Running: { start_time: string; log_id: number } }

export type FEFileHash = { value: string; algo: FEHashAlgo }

export type OpenInstanceFolder = { instance_id: InstanceId; folder: InstanceFolder }

export type Progress = "Indeterminate" | { Known: number } | { Failed: FeError }

export type FESettings = { theme: string; language: string; reducedMotion: boolean; discordIntegration: boolean; releaseChannel: string; concurrentDownloads: number; showNews: boolean; xmx: number; xms: number; isFirstLaunch: boolean; startupResolution: string; javaCustomArgs: string; autoManageJava: boolean }

export type FEModsParametersBody = { modIds: number[] }

export type DuplicateInstance = { instance: InstanceId; new_name: string }

export type MoveInstance = { instance: InstanceId; target: MoveInstanceTarget }

export type FEModAuthor = { id: number; name: string; url: string }

export type FEFilesParametersBody = { fileIds: number[] }

export type FESystemJavaProfileName = "legacy" | "alpha" | "beta" | "gamma" | "minecraftJavaExe"

export type FEModAsset = { id: number; modId: number; title: string; description: string; thumbnailUrl: string; url: string }

export type ListInstance = { id: InstanceId; name: string; favorite: boolean; status: ListInstanceStatus }

export type GameLogEntry = { id: GameLogId; instance_id: InstanceId; active: boolean }

export type FEClassId = "mods" | "modpacks"

export type InstanceFolder = "Root" | "Data" | "Mods" | "Configs" | "Screenshots" | "Saves" | "Logs" | "CrashReports" | "ResourcePacks" | "TexturePacks" | "ShaderPacks"

export type CauseSegment = { display: string; debug: string }

export type Subtask = { name: Translation; progress: SubtaskProgress }

export type FEModdedManifestVersion = { id: string; stable: boolean; loaders: FEModdedManifestLoaderVersion[] }

export type ModLoaderType = "Forge" | "Fabric" | "Quilt" | "Unknown"

export type FEModLinks = { websiteUrl: string | null; wikiUrl: string | null; issuesUrl: string | null; sourceUrl: string | null }

export type FESortableGameVersion = { gameVersionName: string; gameVersionPadded: string; gameVersion: string; gameVersionReleaseDate: string; gameVersionTypeId: number | null }

export type FEModFileResponse = { data: FEFile; pagination: FEPagination | null }

export type GameLogId = number

export type CreateInstanceVersion = { Version: GameVersion } | { Modpack: Modpack }

export type FEManagedJavaArch = "x64" | "x86" | "arm32" | "arm64"

export type FEModSearchParameters = { query: FEModSearchParametersQuery }

export type FEManagedJavaSetupArgs = { os: FEManagedJavaOs; arch: FEManagedJavaArch; vendor: FEVendor; id: string }

export type FEModSearchResponse = { data: FEMod[]; pagination: FEPagination | null }

export type FEFilesResponse = { data: FEFile[]; pagination: FEPagination | null }

export type FEManagedJavaSetupProgress = "idle" | { downloading: [string, string] } | { extracting: [string, string] } | "done"

export type FeError = { cause: CauseSegment[]; backtrace: string }

export type FECategory = { id: number; name: string; slug: string; url: string; iconUrl: string; dateModified: string; isClass: boolean | null; classId: number | null; parentCategoryId: number | null; displayIndex: number | null }

export type FEModFilesResponse = { data: FEFile[]; pagination: FEPagination | null }
