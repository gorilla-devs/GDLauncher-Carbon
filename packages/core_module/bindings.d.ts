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
        { key: "instance.explore", input: ExploreQuery, result: ExploreEntry[] } | 
        { key: "instance.findModUpdate", input: UpdateMod, result: RemoteVersion | null } | 
        { key: "instance.getAllInstances", input: never, result: ListInstance[] } | 
        { key: "instance.getDefaultGroup", input: never, result: number } | 
        { key: "instance.getGroups", input: never, result: ListGroup[] } | 
        { key: "instance.getImportEntityDefaultPath", input: ImportEntity, result: string | null } | 
        { key: "instance.getImportScanStatus", input: never, result: FullImportScanStatus } | 
        { key: "instance.getImportableEntities", input: never, result: ImportEntityStatus[] } | 
        { key: "instance.getInstanceDetails", input: FEInstanceId | null, result: InstanceDetails | null } | 
        { key: "instance.getInstanceMods", input: FEInstanceId | null, result: Mod[] | null } | 
        { key: "instance.getLogs", input: FEInstanceId, result: GameLogEntry[] } | 
        { key: "instance.getModSources", input: FEInstanceId, result: ModSources } | 
        { key: "instance.getModpackInfo", input: FEInstanceId | null, result: FEInstanceModpackInfo | null } | 
        { key: "java.getAvailableJavas", input: never, result: { [key: number]: FEJavaComponent[] } } | 
        { key: "java.getJavaProfiles", input: never, result: FEJavaProfile[] } | 
        { key: "java.getManagedArch", input: never, result: FEManagedJavaArch[] } | 
        { key: "java.getManagedOS", input: never, result: FEManagedJavaOs[] } | 
        { key: "java.getManagedVendors", input: never, result: FEVendor[] } | 
        { key: "java.getManagedVersionsByVendor", input: FEVendor, result: FEManagedJavaOsMap } | 
        { key: "java.getSetupManagedJavaProgress", input: never, result: FEManagedJavaSetupProgress } | 
        { key: "java.systemJavaProfileAssignments", input: never, result: { [key: string]: string[] } } | 
        { key: "mc.getFabricVersions", input: never, result: FEModdedManifest } | 
        { key: "mc.getForgeVersions", input: never, result: FEModdedManifest } | 
        { key: "mc.getMinecraftVersions", input: never, result: ManifestVersion[] } | 
        { key: "mc.getNeoforgeVersions", input: never, result: FEModdedManifest } | 
        { key: "mc.getQuiltVersions", input: never, result: FEModdedManifest } | 
        { key: "modplatforms.curseforge.getCategories", input: never, result: FECategoriesResponse } | 
        { key: "modplatforms.curseforge.getFiles", input: CFFEFilesParameters, result: FEFilesResponse } | 
        { key: "modplatforms.curseforge.getMod", input: CFFEModParameters, result: FEModResponse } | 
        { key: "modplatforms.curseforge.getModDescription", input: CFFEModDescriptionParameters, result: FEModDescriptionResponse } | 
        { key: "modplatforms.curseforge.getModFile", input: CFFEModFileParameters, result: FEModFileResponse } | 
        { key: "modplatforms.curseforge.getModFileChangelog", input: CFFEModFileChangelogParameters, result: FEModFileChangelogResponse } | 
        { key: "modplatforms.curseforge.getModFiles", input: CFFEModFilesParameters, result: FEModFilesResponse } | 
        { key: "modplatforms.curseforge.getModloaders", input: never, result: CFFEModLoaderType[] } | 
        { key: "modplatforms.curseforge.getMods", input: CFFEModsParameters, result: FEModsResponse } | 
        { key: "modplatforms.curseforge.search", input: CFFEModSearchParameters, result: FEModSearchResponse } | 
        { key: "modplatforms.modrinth.getCategories", input: never, result: MRFECategoriesResponse } | 
        { key: "modplatforms.modrinth.getLoaders", input: never, result: MRFELoadersResponse } | 
        { key: "modplatforms.modrinth.getProject", input: MRFEProjectID, result: MRFEProject } | 
        { key: "modplatforms.modrinth.getProjectTeam", input: MRFEProjectID, result: MRFETeamResponse } | 
        { key: "modplatforms.modrinth.getProjectVersions", input: MRFEProjectVersionsFilters, result: MRFEVersionsResponse } | 
        { key: "modplatforms.modrinth.getProjects", input: MRFEProjectIDs, result: MRFEProjectsResponse } | 
        { key: "modplatforms.modrinth.getTeam", input: MRFETeamID, result: MRFETeamResponse } | 
        { key: "modplatforms.modrinth.getVersion", input: MRFEVersionID, result: MRFEVersion } | 
        { key: "modplatforms.modrinth.getVersions", input: MRFEVersionIDs, result: MRFEVersionsResponse } | 
        { key: "modplatforms.modrinth.search", input: MRFEProjectSearchParameters, result: MRFEProjectSearchResponse } | 
        { key: "modplatforms.unifiedSearch", input: FEUnifiedSearchParameters, result: FEUnifiedSearchResponse } | 
        { key: "settings.getPrivacyStatementBody", input: never, result: string } | 
        { key: "settings.getSettings", input: never, result: FESettings } | 
        { key: "settings.getTermsOfServiceBody", input: never, result: string } | 
        { key: "systeminfo.getTotalRAM", input: never, result: string } | 
        { key: "systeminfo.getUsedRAM", input: never, result: string } | 
        { key: "vtask.getTask", input: FETaskId | null, result: FETask | null } | 
        { key: "vtask.getTasks", input: never, result: FETask[] },
    mutations: 
        { key: "account.deleteAccount", input: string, result: null } | 
        { key: "account.enroll.begin", input: never, result: null } | 
        { key: "account.enroll.cancel", input: never, result: null } | 
        { key: "account.enroll.finalize", input: never, result: null } | 
        { key: "account.refreshAccount", input: string, result: null } | 
        { key: "account.setActiveUuid", input: string | null, result: null } | 
        { key: "instance.cancelImportScan", input: never, result: null } | 
        { key: "instance.changeModpack", input: ChangeModpack, result: FETaskId } | 
        { key: "instance.createGroup", input: string, result: FEGroupId } | 
        { key: "instance.createInstance", input: CreateInstance, result: FEInstanceId } | 
        { key: "instance.deleteGroup", input: FEGroupId, result: null } | 
        { key: "instance.deleteInstance", input: FEInstanceId, result: null } | 
        { key: "instance.deleteLog", input: GameLogId, result: null } | 
        { key: "instance.deleteMod", input: InstanceMod, result: null } | 
        { key: "instance.disableMod", input: InstanceMod, result: null } | 
        { key: "instance.duplicateInstance", input: DuplicateInstance, result: FEInstanceId } | 
        { key: "instance.enableMod", input: InstanceMod, result: null } | 
        { key: "instance.export", input: ExportArgs, result: FETaskId } | 
        { key: "instance.importInstance", input: ImportRequest, result: FETaskId } | 
        { key: "instance.installLatestMod", input: InstallLatestMod, result: FETaskId } | 
        { key: "instance.installMod", input: InstallMod, result: FETaskId } | 
        { key: "instance.killInstance", input: FEInstanceId, result: null } | 
        { key: "instance.launchInstance", input: FEInstanceId, result: null } | 
        { key: "instance.loadIconUrl", input: string, result: null } | 
        { key: "instance.moveGroup", input: MoveGroup, result: null } | 
        { key: "instance.moveInstance", input: MoveInstance, result: null } | 
        { key: "instance.openInstanceFolder", input: OpenInstanceFolder, result: null } | 
        { key: "instance.prepareInstance", input: FEInstanceId, result: FETaskId } | 
        { key: "instance.setFavorite", input: SetFavorite, result: null } | 
        { key: "instance.setImportScanTarget", input: [ImportEntity, string], result: null } | 
        { key: "instance.updateInstance", input: FEUpdateInstance, result: null } | 
        { key: "instance.updateMod", input: UpdateMod, result: FETaskId } | 
        { key: "java.createCustomJavaVersion", input: string, result: null } | 
        { key: "java.createJavaProfile", input: FECreateJavaProfileArgs, result: null } | 
        { key: "java.deleteJavaProfile", input: string, result: null } | 
        { key: "java.deleteJavaVersion", input: string, result: null } | 
        { key: "java.setupManagedJava", input: FEManagedJavaSetupArgs, result: string } | 
        { key: "java.updateJavaProfile", input: FEUpdateJavaProfileArgs, result: null } | 
        { key: "java.validateCustomJavaPath", input: string, result: boolean } | 
        { key: "longRunning", input: never, result: boolean } | 
        { key: "metrics.sendEvent", input: FEMetricsEvent, result: null } | 
        { key: "settings.setSettings", input: FESettingsUpdate, result: null } | 
        { key: "vtask.dismissTask", input: FETaskId, result: null },
    subscriptions: never
};

export type InvalidListInstance = "JsonMissing" | { JsonError: ConfigurationParseError } | { Other: string }

export type FEManagedJavaSetupArgs = { os: FEManagedJavaOs; arch: FEManagedJavaArch; vendor: FEVendor; id: string }

export type Set<T> = { Set: T }

export type CFFEPagination = { index: number; pageSize: number; resultCount: number; totalCount: number }

export type FEJavaComponentType = "local" | "managed" | "custom"

export type ImportEntry = { Valid: ImportableInstance } | { Invalid: InvalidImportEntry }

export type EnrollmentStatus = "requestingCode" | { pollingCode: DeviceCode } | "queryingAccount" | { complete: AccountEntry } | { failed: EnrollmentError }

export type FEModdedManifestVersion = { id: string; stable: boolean; loaders: FEModdedManifestLoaderVersion[] }

export type FEUpdateJavaProfileArgs = { profileName: string; javaId?: string | null }

export type ExploreEntry = { name: string; type: ExploreEntryType }

export type MRFECategory = { icon: string; name: string; project_type: MRFEProjectType; header: string }

export type CFFEModLinks = { websiteUrl: string | null; wikiUrl: string | null; issuesUrl: string | null; sourceUrl: string | null }

export type MRFEHashes = ({ [key: string]: string }) & { sha512: string; sha1: string }

export type Modpack = { type: "curseforge"; value: CurseforgeModpack } | { type: "modrinth"; value: ModrinthModpack }

export type MRFEProject = { slug: string; title: string; description: string; categories: string[]; client_side: MRFEProjectSupportRange; server_side: MRFEProjectSupportRange; body: string; additional_categories: string[]; issues_url: string | null; source_url: string | null; wiki_url: string | null; discord_url: string | null; donation_urls: MRFEDonationLink[]; project_type: MRFEProjectType; downloads: number; icon_url: string | null; color: number | null; id: string; team: string; moderator_message: MRFEModeratorMessage | null; published: string; updated: string; approved: string | null; followers: number; status: MRFEProjectStatus; license: MRFELicense; versions: string[]; game_versions: string[]; loaders: string[]; gallery: MRFEGalleryItem[] }

export type UpdateMod = { instance_id: FEInstanceId; mod_id: string }

export type FEModsResponse = { data: CFFEMod[]; pagination: CFFEPagination | null }

export type MRFEAdditionalFileType = "requiredResourcePack" | "optionalResourcePack"

export type ModrinthModMetadata = { project_id: string; version_id: string; title: string; version: string; urlslug: string; description: string; authors: string; has_image: boolean }

export type MRFEStatus = "listed" | "archived" | "draft" | "unlisted" | "scheduled" | "unknown"

export type MRFETeamID = string

export type MRFESearchFacet = { Category: string } | { Version: string } | { License: string } | { ProjectType: string }

export type EnrollmentError = "deviceCodeExpired" | { xboxAccount: XboxError } | "noGameOwnership" | "noGameProfile"

export type FEJavaComponent = { id: string; path: string; version: string; type: FEJavaComponentType; isValid: boolean }

export type MRFEVersionID = string

export type MRFEProjectSearchResult = { slug: string; title: string; description: string; categories: string[] | null; client_side: MRFEProjectSupportRange; server_side: MRFEProjectSupportRange; project_type: MRFEProjectType; downloads: number; icon_url: string | null; color: number | null; project_id: string; author: string; display_categories: string[] | null; versions: string[]; follows: number; date_created: string; date_modified: string; latest_version: string | null; license: string; gallery: string[] | null; featured_gallery: string | null }

export type FEUnifiedSearchParameters = { searchQuery: string | null; categories: And<FEUnifiedSearchCategoryID> | null; gameVersions: Or<string> | null; modloaders: Or<FEUnifiedModLoaderType> | null; projectType: FEUnifiedSearchType | null; sortIndex: FEUnifiedModSortIndex | null; sortOrder: CFFEModSearchSortOrder | null; index: number | null; pageSize: number | null; searchApi: FESearchAPI }

export type InstanceDetails = { name: string; favorite: boolean; version: string | null; modpack: ModpackInfo | null; globalJavaArgs: boolean; extraJavaArgs: string | null; memory: MemoryRange | null; gameResolution: GameResolution | null; lastPlayed: string | null; secondsPlayed: number; modloaders: ModLoader[]; javaOverride: FEJavaOverride | null; requiredJavaProfile: string | null; preLaunchHook: string | null; postExitHook: string | null; wrapperCommand: string | null; notes: string; state: LaunchState; iconRevision: number | null; hasPackUpdate: boolean }

export type StandardVersion = { release: string; modloaders: ModLoader[] }

export type FEManagedJavaOsMap = { [key: FEManagedJavaOs]: FEManagedJavaArchMap }

export type FEManagedJavaArch = "x64" | "x86" | "arm32" | "arm64"

export type FEModFileChangelogResponse = { data: string; pagination: CFFEPagination | null }

/**
 * An image that have been uploaded to a project's gallery
 */
export type MRFEGalleryItem = { url: string; featured: boolean; title: string | null; description: string | null; created: string; ordering: string | null }

export type And<T> = Or<T>[]

export type ManifestVersion = { id: string; type: McType }

export type CreateInstance = { group: FEGroupId; name: string; use_loaded_icon: boolean; version: CreateInstanceVersion; notes: string }

export type FEManagedJavaOs = "windows" | "linux" | "macOs"

export type MRFEDependency = { version_id: string | null; project_id: string | null; file_name: string | null; dependency_type: MRFEDependencyType }

export type AccountEntry = { username: string; uuid: string; lastUsed: string; type: AccountType }

export type ImportEntityStatus = { entity: ImportEntity; supported: boolean; selection_type: ImportEntitySelectionType }

export type FEInstanceModloaderType = "neoforge" | "forge" | "fabric" | "quilt"

export type ModFileMetadata = { id: string; modid: string | null; name: string | null; version: string | null; description: string | null; authors: string | null; modloaders: FEInstanceModloaderType[]; sha_1: string; sha_512: string; murmur_2: string; has_image: boolean }

export type CauseSegment = { display: string; debug: string }

export type FEUnifiedPagination = { index: number; pageSize: number; resultCount: number; totalCount: number }

export type ModSources = { channels: ModChannelWithUsage[]; platform_blacklist: ModPlatform[] }

export type FESettings = { theme: string; language: string; reducedMotion: boolean; discordIntegration: boolean; releaseChannel: FEReleaseChannel; concurrentDownloads: number; downloadDependencies: boolean; launcherActionOnGameLaunch: FELauncherActionOnGameLaunch; showNews: boolean; instancesSortBy: InstancesSortBy; instancesSortByAsc: boolean; instancesGroupBy: InstancesGroupBy; instancesGroupByAsc: boolean; instancesTileSize: number; deletionThroughRecycleBin: boolean; xmx: number; xms: number; preLaunchHook: string | null; wrapperCommand: string | null; postExitHook: string | null; isFirstLaunch: boolean; gameResolution: GameResolution | null; javaCustomArgs: string; autoManageJavaSystemProfiles: boolean; modSources: ModSources; termsAndPrivacyAccepted: boolean; metricsEnabled: boolean; randomUserUuid: string }

export type ModSource = { Curseforge: CurseforgeMod } | { Modrinth: ModrinthMod }

export type ModpackInfo = { modpack: Modpack; locked: boolean }

export type ListInstanceStatus = { status: "valid"; value: ValidListInstance } | { status: "invalid"; value: InvalidListInstance }

export type CFFEFileModule = { name: string; fingerprint: string }

export type CFFEModFileParameters = { modId: number; fileId: number }

export type ImportEntitySelectionType = "file" | "directory"

export type MRFESearchFacetOr = MRFESearchFacet[]

export type FEModDescriptionResponse = { data: string; pagination: CFFEPagination | null }

export type FEModResponse = { data: CFFEMod; pagination: CFFEPagination | null }

export type CFFEModStatus = "new" | "changesRequired" | "underSoftReview" | "approved" | "rejected" | "changesMade" | "inactive" | "abandoned" | "deleted" | "underReview"

export type FEManagedJavaVersion = { id: string; name: string; downloadUrl: string; javaVersion: string }

export type CFFEFileIndex = { gameVersion: string; fileId: number; filename: string; releaseType: CFFEFileReleaseType; gameVersionTypeId: number | null; modLoader: CFFEModLoaderType | null }

export type FEGroupId = number

export type CFFEFilesParameters = { body: CFFEFilesParametersBody }

export type ModrinthMod = { project_id: string; version_id: string }

export type AccountType = "microsoft" | "offline"

export type DeviceCode = { userCode: string; verificationUri: string; expiresAt: string }

export type CFFEModSearchParameters = { query: CFFEModSearchParametersQuery }

export type DuplicateInstance = { instance: FEInstanceId; new_name: string }

export type ImportEntity = "LegacyGDLauncher" | "MRPack" | "Modrinth" | "CurseForgeZip" | "CurseForge" | "ATLauncher" | "Technic" | "FTB" | "MultiMC" | "PrismLauncher"

export type FEUnifiedModLoaderType = "forge" | "neoforge" | "fabric" | "quilt" | "liteloader" | "unknown" | "cauldron" | "bukkit" | "bungeecord" | "canvas" | "datapack" | "folia" | "iris" | "minecraft" | "modloader" | "optifine" | "paper" | "purpur" | "rift" | "spigot" | "sponge" | "vanilla" | "velocity" | "waterfall"

export type MRFEProjectType = "mod" | "shader" | "modpack" | "resourcepack" | "plugin" | "project" | "datapack"

export type MRFEUserRole = "developer" | "moderator" | "admin"

export type FEUnifiedSearchResponse = { searchApi: FESearchAPI; data: FEUnifiedSearchResult[]; pagination: FEUnifiedPagination | null }

export type MRFEVersionsResponse = MRFEVersion[]

export type MoveInstanceTarget = { BeforeInstance: FEInstanceId } | { BeginningOfGroup: FEGroupId } | { EndOfGroup: FEGroupId }

export type ValidListInstance = { mc_version: string | null; modloader: FEInstanceModloaderType | null; modpack: Modpack | null; state: LaunchState }

export type CFFEFileStatus = "processing" | "changesRequired" | "underReview" | "approved" | "rejected" | "malwareDetected" | "deleted" | "archived" | "testing" | "released" | "readyForReview" | "deprecated" | "baking" | "awaitingPublishing" | "failedPublishing"

export type FEUpdateInstance = { instance: FEInstanceId; name?: Set<string> | null; useLoadedIcon?: Set<boolean> | null; notes?: Set<string> | null; version?: Set<string> | null; modloader?: Set<ModLoader | null> | null; javaOverride?: Set<FEJavaOverride | null> | null; globalJavaArgs?: Set<boolean> | null; extraJavaArgs?: Set<string | null> | null; memory?: Set<MemoryRange | null> | null; preLaunchHook?: Set<string | null> | null; postExitHook?: Set<string | null> | null; wrapperCommand?: Set<string | null> | null; gameResolution?: Set<GameResolution | null> | null; modSources?: Set<ModSources | null> | null; modpackLocked?: Set<boolean | null> | null }

export type ModPlatform = "Curseforge" | "Modrinth"

export type ModChannel = "Alpha" | "Beta" | "Stable"

export type InstallMod = { instance_id: FEInstanceId; mod_source: ModSource; install_deps: boolean; replaces_mod: string | null }

export type ImportRequest = { index: number; name: string | null }

export type CFFEModLoaderType = "forge" | "neoforge" | "cauldron" | "liteloader" | "fabric" | "quilt" | "unknown"

export type InstanceFolder = "Root" | "Data" | "Mods" | "Configs" | "Screenshots" | "Saves" | "Logs" | "CrashReports" | "ResourcePacks" | "TexturePacks" | "ShaderPacks"

export type FEManagedJavaArchMap = { [key: FEManagedJavaArch]: FEManagedJavaVersion[] }

export type ModLoader = { type_: FEInstanceModloaderType; version: string }

export type FELauncherActionOnGameLaunch = "quitApp" | "closeWindow" | "minimizeWindow" | "hideWindow" | "none"

export type CFFEFilesParametersBody = { fileIds: number[] }

export type FEModdedManifestLoaderVersion = { id: string }

export type ExportTarget = "Curseforge" | "Modrinth"

export type FEModSearchResponse = { data: CFFEMod[]; pagination: CFFEPagination | null }

export type MRFELoader = { icon: string; name: MRFELoaderType; supported_project_types: MRFEProjectType[] }

export type MRFEProjectID = string

export type XboxError = "noAccount" | "xboxServicesBanned" | "adultVerificationRequired" | "childAccount" | { unknown: number }

export type Translation = { translation: "InstanceTaskDeleting" } | { translation: "ModCacheTaskUpdate" } | { translation: "ModCacheTaskUpdateScanFiles" } | { translation: "ModCacheTaskUpdateQueryApis" } | { translation: "InstanceTaskLaunch"; args: { name: string } } | { translation: "InstanceTaskPrepare"; args: { name: string } } | { translation: "InstanceTaskLaunchRequestVersions" } | { translation: "InstanceTaskLaunchRequestModpack" } | { translation: "InstanceTaskLaunchDownloadModpack" } | { translation: "InstanceTaskLaunchDownloadModpackFiles" } | { translation: "InstanceTaskLaunchExtractModpackFiles" } | { translation: "InstanceTaskLaunchDownloadAddonMetadata" } | { translation: "InstanceTaskLaunchApplyStagedPatches" } | { translation: "InstanceTaskLaunchDownloadJava" } | { translation: "InstanceTaskLaunchExtractJava" } | { translation: "InstanceTaskLaunchWaitDownloadFiles" } | { translation: "InstanceTaskLaunchDownloadFiles" } | { translation: "InstanceTaskGeneratingPackInfo" } | { translation: "InstanceTaskFillCache" } | { translation: "InstanceTaskLaunchExtractNatives" } | { translation: "InstanceTaskReconstructAssets" } | { translation: "InstanceTaskLaunchRunForgeProcessors" } | { translation: "InstanceTaskLaunchRunNeoforgeProcessors" } | { translation: "InstanceTaskInstallMod"; args: { mod_name: string; instance_name: string } } | { translation: "InstanceTaskInstallModDownloadFile" } | { translation: "FinalizingImport" } | { translation: "InstanceImportLegacyBadConfigFile" } | { translation: "InstanceImportCfZipMalformed" } | { translation: "InstanceImportCfZipMissingManifest" } | { translation: "InstanceImportCfZipMalformedManifest" } | { translation: "InstanceImportCfZipNotMinecraftModpack" } | { translation: "InstanceImportMrpackMalformed" } | { translation: "InstanceImportMrpackMissingManifest" } | { translation: "InstanceImportMrpackMalformedManifest" } | { translation: "InstanceExport" } | { translation: "InstanceExportScanningMods" } | { translation: "InstanceExportCacheMods" } | { translation: "InstanceExportCalculateSize" } | { translation: "InstanceExportCreatingBundle" }

export type MRFEProjectSearchResponse = { hits: MRFEProjectSearchResult[]; offset: number; limit: number; total_hits: number }

export type CFFEModDescriptionParameters = { modId: number }

export type ImportScanStatus = "NoResults" | { SingleResult: ImportEntry } | { MultiResult: ImportEntry[] }

export type FEMetricsEvent = { event_name: "page_view"; data: string } | { event_name: "featured_modpack_installed"; data: { campaign_id: string; item_id: string } }

export type GameVersion = { Standard: StandardVersion }

export type FETask = { name: Translation; progress: Progress; downloaded: number; download_total: number; active_subtasks: FESubtask[] }

export type MemoryRange = { min_mb: number; max_mb: number }

export type GameResolution = { type: "Standard"; value: [number, number] } | { type: "Custom"; value: [number, number] }

export type FEModFilesResponse = { data: CFFEFile[]; pagination: CFFEPagination | null }

export type MRFERequestedVersionStatus = "listed" | "archived" | "draft" | "unlisted"

export type CFFEFileHash = { value: string; algo: CFFEHashAlgo }

export type CFFEModSearchSortField = "featured" | "popularity" | "lastUpdated" | "name" | "author" | "totalDownloads" | "category" | "gameVersion"

export type FEModdedManifest = { gameVersions: FEModdedManifestVersion[] }

export type MRFEVersionType = "alpha" | "beta" | "release"

export type CFFEModSearchSortOrder = "ascending" | "descending"

export type CFFEModFilesParametersQuery = { gameVersion?: string | null; modLoaderType?: CFFEModLoaderType | null; gameVersionTypeId?: number | null; index?: number | null; pageSize?: number | null }

export type CurseforgeModpack = { project_id: number; file_id: number }

export type ExploreEntryType = { File: { size: number } } | "Directory"

export type CFFEHashAlgo = "sha1" | "md5"

export type LatestModSource = { Curseforge: number } | { Modrinth: string }

export type GameLogEntry = { id: GameLogId; instance_id: FEInstanceId; active: boolean }

export type FEUnifiedModSortIndex = { curseForge: CFFEModSearchSortField } | { modrinth: MRFESearchIndex }

export type FETaskId = number

export type CFFEFileDependency = { modId: number; relationType: CFFEFileRelationType }

export type CFFEFileReleaseType = "stable" | "beta" | "alpha"

export type FEUnifiedSearchType = "mod" | "modPack"

export type CFFESortableGameVersion = { gameVersionName: string; gameVersionPadded: string; gameVersion: string; gameVersionReleaseDate: string; gameVersionTypeId: number | null }

export type MRFEProjectSearchParameters = { query: string | null; facets: MRFESearchFacetAnd | null; index: MRFESearchIndex | null; offset: number | null; limit: number | null; filters: string | null }

export type MRFEProjectStatus = "approved" | "rejected" | "draft" | "unlisted" | "archived" | "processing" | "withheld" | "unknown"

export type MoveGroup = { group: FEGroupId; before: FEGroupId | null }

export type CurseforgeMod = { project_id: number; file_id: number }

export type MRFESearchIndex = "relevance" | "downloads" | "follows" | "newest" | "updated"

export type CFFEModParameters = { modId: number }

export type MRFEDonationLink = { id: string; platform: string; url: string }

export type CFFEFileRelationType = "embeddedLibrary" | "optionalDependency" | "requiredDependency" | "tool" | "incompatible" | "include"

export type GameResolution = { type: "Standard"; value: [number, number] } | { type: "Custom"; value: [number, number] }

export type FECategoriesResponse = { data: CFFECategory[]; pagination: CFFEPagination | null }

export type GameLogId = number

export type RemoteVersion = ({ platform: "Curseforge" } & CFFEFile) | ({ platform: "Modrinth" } & MRFEVersion)

export type InstanceMod = { instance_id: FEInstanceId; mod_id: string }

export type FEInstanceId = number

export type Mod = { id: string; filename: string; enabled: boolean; metadata: ModFileMetadata | null; curseforge: CurseForgeModMetadata | null; modrinth: ModrinthModMetadata | null; has_update: boolean }

export type CurseForgeModMetadata = { project_id: number; file_id: number; name: string; version: string; urlslug: string; summary: string; authors: string; has_image: boolean }

export type MRFEProjectVersionsFilters = { project_id: MRFEProjectID; game_versions?: string[] | null; loaders?: string[] | null; limit?: number | null; offset?: number | null }

export type MRFEVersion = { name: string; version_number: string; changelog: string | null; dependencies: MRFEDependency[]; game_versions: string[]; version_type: MRFEVersionType; loaders: string[]; featured: boolean; status: MRFEStatus | null; requested_status: MRFERequestedVersionStatus | null; id: string; project_id: string; author_id: string; date_published: string; downloads: number; files: MRFEVersionFile[] }

export type CFFEModAsset = { id: number; modId: number; title: string; description: string; thumbnailUrl: string; url: string }

export type InvalidImportEntry = { name: string; reason: Translation }

export type FECreateJavaProfileArgs = { profileName: string; javaId: string | null }

export type FEJavaProfile = { name: string; javaId: string | null; isSystem: boolean }

export type MRFEProjectsResponse = MRFEProject[]

export type CFFECategory = { id: number; name: string; slug: string; url: string; iconUrl: string | null; dateModified: string; isClass: boolean | null; classId: number | null; parentCategoryId: number | null; displayIndex: number | null }

export type FullImportScanStatus = { scanning: boolean; status: ImportScanStatus }

export type ConfigurationParseError = { type_: ConfigurationParseErrorType; message: string; line: number; config_text: string }

export type FESubtaskProgress = { download: { downloaded: number; total: number } } | { item: { current: number; total: number } } | "opaque"

export type FEModFileResponse = { data: CFFEFile; pagination: CFFEPagination | null }

export type CFFEModsParameters = { body: CFFEModsParametersBody }

export type OpenInstanceFolder = { instance_id: FEInstanceId; folder: InstanceFolder }

export type FESubtask = { name: Translation; progress: FESubtaskProgress }

export type CreateInstanceVersion = { Version: GameVersion } | { Modpack: Modpack }

export type FEReleaseChannel = "stable" | "alpha" | "beta"

export type MRFELoadersResponse = MRFELoader[]

export type ModrinthModpack = { project_id: string; version_id: string }

export type MRFELoaderType = "bukkit" | "bungeecord" | "canvas" | "datapack" | "fabric" | "folia" | "forge" | "neoforge" | "iris" | "liteloader" | "minecraft" | "modloader" | "optifine" | "paper" | "purpur" | "quilt" | "rift" | "spigot" | "sponge" | "vanilla" | "velocity" | "waterfall" | "other"

export type FEFilesResponse = { data: CFFEFile[]; pagination: CFFEPagination | null }

export type InstancesGroupBy = "group" | "modloader" | "gameVersion" | "modplatform"

export type CFFEModFilesParameters = { modId: number; query: CFFEModFilesParametersQuery }

export type FEJavaOverride = { Profile: string | null } | { Path: string | null }

export type MRFEModeratorMessage = { message: string; body: string | null }

export type MRFECategoriesResponse = MRFECategory[]

export type MRFEVersionFile = { hashes: MRFEHashes; url: string; filename: string; primary: boolean; size: number; file_type: MRFEAdditionalFileType | null }

export type CFFEClassId = "mods" | "resourcePacks" | "modpacks" | "customizations" | "bukkitPlugins" | "worlds" | "addons" | "shaders" | { other: number }

export type ModChannelWithUsage = { channel: ModChannel; allow_updates: boolean }

export type ExploreQuery = { instance_id: FEInstanceId; path: string[] }

export type FEVendor = "azul"

export type FEInstanceModpackInfo = { name: string; version_name: string; url_slug: string; has_image: boolean }

export type FEUnifiedSearchCategoryID = { curseforge: number } | { modrinth: string }

export type MRFEProjectIDs = string[]

export type ExportEntry = { entries: { [key: string]: ExportEntry | null } }

export type MRFETeamMember = { team_id: string; user: MRFEUser; role: string; ordering: string | null }

export type ListGroup = { id: FEGroupId; name: string }

export type MRFEVersionIDs = string[]

export type ConfigurationParseErrorType = "Syntax" | "Data" | "Eof" | "Unknown"

export type MRFETeamResponse = MRFETeamMember[]

export type SetFavorite = { instance: FEInstanceId; favorite: boolean }

export type CFFEModsParametersBody = { modIds: number[] }

export type InstallLatestMod = { instance_id: FEInstanceId; mod_source: LatestModSource }

export type AccountStatus = "ok" | "expired" | "refreshing" | "invalid"

export type ListInstance = { id: FEInstanceId; group_id: FEGroupId; name: string; favorite: boolean; status: ListInstanceStatus; icon_revision: number | null; last_played: string | null; date_created: string; date_updated: string; seconds_played: number }

export type CFFEFile = { id: number; gameId: number; modId: number; isAvailable: boolean; displayName: string; fileName: string; releaseType: CFFEFileReleaseType; fileStatus: CFFEFileStatus; hashes: CFFEFileHash[]; fileDate: string; fileLength: number; downloadCount: number; downloadUrl: string | null; gameVersions: string[]; sortableGameVersions: CFFESortableGameVersion[]; dependencies: CFFEFileDependency[]; exposeAsAlternative: boolean | null; parentProjectFileId: number | null; alternateFileId: number | null; isServerPack: boolean | null; serverPackFileId: number | null; isEarlyAccessContent: boolean | null; earlyAccessEndDate: string | null; fileFingerprint: string; modules: CFFEFileModule[] }

export type MRFELicense = { id: string; name: string; url: string | null }

export type Progress = { type: "Indeterminate" } | { type: "Known"; value: number } | { type: "Failed"; value: FeError }

export type MRFESearchFacetAnd = MRFESearchFacetOr[]

export type LaunchState = { state: "inactive"; value: { failed_task: FETaskId | null } } | { state: "preparing"; value: FETaskId } | { state: "running"; value: { start_time: string; log_id: number } } | { state: "deleting" }

export type FeError = { cause: CauseSegment[]; backtrace: string }

export type ChangeModpack = { instance: FEInstanceId; modpack: Modpack }

export type MRFEDependencyType = "required" | "optional" | "incompatible" | "embedded"

export type FESearchAPI = "curseforge" | "modrinth"

export type MoveInstance = { instance: FEInstanceId; target: MoveInstanceTarget }

export type CFFEMod = { id: number; gameId: number; name: string; slug: string; links: CFFEModLinks; summary: string; status: CFFEModStatus; downloadCount: number; isFeatured: boolean; primaryCategoryId: number; categories: CFFECategory[]; classId: CFFEClassId | null; authors: CFFEModAuthor[]; logo: CFFEModAsset | null; screenshots: CFFEModAsset[]; mainFileId: number; latestFiles: CFFEFile[]; latestFilesIndexes: CFFEFileIndex[]; dateCreated: string; dateModified: string; dateReleased: string; allowModDistribution: boolean | null; gamePopularityRank: number; isAvailable: boolean; thumbsUpCount: number }

export type Or<T> = T[]

export type MRFEUser = { username: string; name: string | null; email: string | null; bio: string | null; id: string; github_id: number | null; avatar_url: string | null; created: string; role: MRFEUserRole; badges: number }

export type InstancesSortBy = "name" | "lastPlayed" | "lastUpdated" | "created" | "gameVersion" | "mostPlayed"

export type CFFEModAuthor = { id: number; name: string; url: string }

export type CFFEModSearchParametersQuery = { gameId: number; searchFilter: string | null; gameVersion: string | null; categoryIds: number[] | null; sortOrder: CFFEModSearchSortOrder | null; sortField: CFFEModSearchSortField | null; classId: CFFEClassId | null; modLoaderTypes: CFFEModLoaderType[] | null; gameVersionTypeId: number | null; authorId: number | null; slug: string | null; index: number | null; pageSize: number | null }

export type FEUnifiedSearchResult = { curseforge: CFFEMod } | { modrinth: MRFEProjectSearchResult }

export type CFFEModFileChangelogParameters = { modId: number; fileId: number }

export type FESettingsUpdate = { theme?: Set<string> | null; language?: Set<string> | null; reducedMotion?: Set<boolean> | null; discordIntegration?: Set<boolean> | null; releaseChannel?: Set<FEReleaseChannel> | null; concurrentDownloads?: Set<number> | null; downloadDependencies?: Set<boolean> | null; instancesSortBy?: Set<InstancesSortBy> | null; instancesSortByAsc?: Set<boolean> | null; instancesGroupBy?: Set<InstancesGroupBy> | null; instancesGroupByAsc?: Set<boolean> | null; instancesTileSize?: Set<number> | null; deletionThroughRecycleBin?: Set<boolean> | null; showNews?: Set<boolean> | null; xmx?: Set<number> | null; xms?: Set<number> | null; preLaunchHook?: Set<string | null> | null; wrapperCommand?: Set<string | null> | null; postExitHook?: Set<string | null> | null; isFirstLaunch?: Set<boolean> | null; launcherActionOnGameLaunch?: Set<FELauncherActionOnGameLaunch> | null; gameResolution?: Set<GameResolution | null> | null; javaCustomArgs?: Set<string> | null; autoManageJavaSystemProfiles?: Set<boolean> | null; modSources?: Set<ModSources> | null; termsAndPrivacyAccepted?: Set<boolean> | null; metricsEnabled?: Set<boolean> | null }

export type ExportArgs = { instance_id: FEInstanceId; target: ExportTarget; save_path: string; self_contained_addons_bundling: boolean; filter: ExportEntry }

export type McType = "old_alpha" | "old_beta" | "release" | "snapshot"

export type MRFEProjectSupportRange = "required" | "optional" | "unsupported" | "unknown"

export type FEManagedJavaSetupProgress = "idle" | { downloading: [string, string] } | { extracting: [string, string] } | "done"

export type ImportableInstance = { filename: string; instance_name: string }
