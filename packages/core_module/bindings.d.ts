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
        { key: "instance.getImportableEntities", input: never, result: FEEntity[] } | 
        { key: "instance.getImportableInstances", input: FEEntity, result: FEImportableInstance[] } | 
        { key: "instance.getInstanceDetails", input: FEInstanceId, result: InstanceDetails } | 
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
        { key: "modplatforms.modrinth.getProjectVersions", input: MRFEProjectID, result: MRFEVersionsResponse } | 
        { key: "modplatforms.modrinth.getProjects", input: MRFEProjectIDs, result: MRFEProjectsResponse } | 
        { key: "modplatforms.modrinth.getTeam", input: MRFETeamID, result: MRFETeamResponse } | 
        { key: "modplatforms.modrinth.getVersion", input: MRFEVersionID, result: MRFEVersion } | 
        { key: "modplatforms.modrinth.getVersions", input: MRFEVersionIDs, result: MRFEVersionsResponse } | 
        { key: "modplatforms.modrinth.search", input: MRFEProjectSearchParameters, result: MRFEProjectSearchResponse } | 
        { key: "modplatforms.unifiedSearch", input: FEUnifiedSearchParameters, result: FEUnifiedSearchResponse } | 
        { key: "settings.getSettings", input: never, result: FESettings } | 
        { key: "systeminfo.getTotalRAM", input: never, result: string } | 
        { key: "systeminfo.getUsedRAM", input: never, result: string } | 
        { key: "vtask.getTask", input: FETaskId, result: FETask | null } | 
        { key: "vtask.getTasks", input: never, result: FETask[] },
    mutations: 
        { key: "account.deleteAccount", input: string, result: null } | 
        { key: "account.enroll.begin", input: never, result: null } | 
        { key: "account.enroll.cancel", input: never, result: null } | 
        { key: "account.enroll.finalize", input: never, result: null } | 
        { key: "account.refreshAccount", input: string, result: null } | 
        { key: "account.setActiveUuid", input: string | null, result: null } | 
        { key: "instance.createGroup", input: string, result: FEGroupId } | 
        { key: "instance.createInstance", input: CreateInstance, result: FEInstanceId } | 
        { key: "instance.deleteGroup", input: FEGroupId, result: null } | 
        { key: "instance.deleteInstance", input: FEInstanceId, result: null } | 
        { key: "instance.deleteLog", input: GameLogId, result: null } | 
        { key: "instance.deleteMod", input: InstanceMod, result: null } | 
        { key: "instance.disableMod", input: InstanceMod, result: null } | 
        { key: "instance.duplicateInstance", input: DuplicateInstance, result: FEInstanceId } | 
        { key: "instance.enableMod", input: InstanceMod, result: null } | 
        { key: "instance.importInstance", input: FEImportInstance, result: FETaskId } | 
        { key: "instance.installMod", input: InstallMod, result: FETaskId } | 
        { key: "instance.killInstance", input: FEInstanceId, result: null } | 
        { key: "instance.launchInstance", input: FEInstanceId, result: null } | 
        { key: "instance.loadIconUrl", input: string, result: null } | 
        { key: "instance.moveGroup", input: MoveGroup, result: null } | 
        { key: "instance.moveInstance", input: MoveInstance, result: null } | 
        { key: "instance.openInstanceFolder", input: OpenInstanceFolder, result: null } | 
        { key: "instance.prepareInstance", input: FEInstanceId, result: null } | 
        { key: "instance.scanImportableInstances", input: FEEntity, result: null } | 
        { key: "instance.setFavorite", input: SetFavorite, result: null } | 
        { key: "instance.updateInstance", input: UpdateInstance, result: null } | 
        { key: "java.deleteJavaVersion", input: string, result: null } | 
        { key: "java.setupManagedJava", input: FEManagedJavaSetupArgs, result: string } | 
        { key: "java.updateSystemJavaProfilePath", input: FEUpdateSystemJavaProfileArgs, result: null } | 
        { key: "metrics.sendEvent", input: FEEvent, result: null } | 
        { key: "metrics.sendPageview", input: FEPageview, result: null } | 
        { key: "settings.setSettings", input: FESettingsUpdate, result: null } | 
        { key: "vtask.dismissTask", input: FETaskId, result: null },
    subscriptions: 
        { key: "invalidateQuery", input: never, result: InvalidationEvent }
};

export type FEManagedJavaSetupArgs = { os: FEManagedJavaOs; arch: FEManagedJavaArch; vendor: FEVendor; id: string }

export type SetFavorite = { instance: FEInstanceId; favorite: boolean }

export type MRFEDependency = { version_id: string | null; project_id: string | null; file_name: string | null; dependency_type: MRFEDependencyType }

export type MRFESearchIndex = "relevance" | "downloads" | "follows" | "newest" | "updated"

export type ListGroup = { id: FEGroupId; name: string; instances: ListInstance[] }

export type MRFEVersionIDs = string[]

export type CurseforgeMod = { project_id: number; file_id: number }

export type FESubtask = { name: Translation; progress: FESubtaskProgress }

export type MRFEVersionType = "alpha" | "beta" | "release"

export type CFFEModStatus = "new" | "changesRequired" | "underSoftReview" | "approved" | "rejected" | "changesMade" | "inactive" | "abandoned" | "deleted" | "underReview"

export type ModLoader = { type_: FEInstanceModLoaderType; version: string }

export type FESystemJavaProfile = { name: FESystemJavaProfileName; javaId: string | null }

export type MRFEProject = { slug: string; title: string; description: string; categories: string[]; client_side: MRFEProjectSupportRange; server_side: MRFEProjectSupportRange; body: string; additional_categories: string[]; issues_url: string | null; source_url: string | null; wiki_url: string | null; discord_url: string | null; donation_urls: MRFEDonationLink[]; project_type: MRFEProjectType; downloads: number; icon_url: string | null; color: number | null; id: string; team: string; moderator_message: MRFEModeratorMessage | null; published: string; updated: string; approved: string | null; followers: number; status: MRFEProjectStatus; license: MRFELicense; versions: string[]; game_versions: string[]; loaders: string[]; gallery: MRFEGalleryItem[] }

export type FETask = { name: Translation; progress: Progress; downloaded: number; download_total: number; active_subtasks: FESubtask[] }

export type CurseforgeModpack = { project_id: number; file_id: number }

export type GameVersion = { Standard: StandardVersion }

export type FEUpdateSystemJavaProfileArgs = { profileName: FESystemJavaProfileName; javaId: string }

export type OpenInstanceFolder = { instance_id: FEInstanceId; folder: InstanceFolder }

export type ModSource = { Curseforge: CurseforgeMod } | { Modrinth: ModrinthMod }

export type FEUnifiedSearchType = "mod" | "modPack"

export type FESettings = { theme: string; language: string; reducedMotion: boolean; discordIntegration: boolean; releaseChannel: string; concurrentDownloads: number; showNews: boolean; xmx: number; xms: number; isFirstLaunch: boolean; startupResolution: string; javaCustomArgs: string; autoManageJava: boolean; isLegalAccepted: boolean; metricsLevel: number | null }

export type FEManagedJavaOsMap = { [key: FEManagedJavaOs]: FEManagedJavaArchMap }

export type MRFEModeratorMessage = { message: string; body: string | null }

export type FEManagedJavaArch = "x64" | "x86" | "arm32" | "arm64"

export type And<T> = Or<T>[]

export type FEManagedJavaOs = "windows" | "linux" | "macOs"

export type FEEntity = "legacyGDLauncher" | "mrpack" | "modrinth" | "curseForgeZip" | "curseForge" | "atlauncher" | "technic" | "ftb" | "multiMC" | "prismLauncher"

export type MRFEHashes = ({ [key: string]: string }) & { sha512: string; sha1: string }

export type FEImportInstance = { entity: FEEntity; index: number }

export type LaunchState = { Inactive: { failed_task: FETaskId | null } } | { Preparing: FETaskId } | { Running: { start_time: string; log_id: number } }

export type EnrollmentError = "deviceCodeExpired" | { xboxAccount: XboxError } | "noGameOwnership" | "noGameProfile"

export type ModpackPlatform = "Curseforge" | "Modrinth"

export type MRFEUser = { username: string; name: string | null; email: string | null; bio: string | null; id: string; github_id: number | null; avatar_url: string; created: string; role: MRFEUserRole; badges: number }

export type MRFELicense = { id: string; name: string; url: string | null }

export type FEUnifiedPagination = { index: number; pageSize: number; resultCount: number; totalCount: number }

export type CFFEModLinks = { websiteUrl: string | null; wikiUrl: string | null; issuesUrl: string | null; sourceUrl: string | null }

export type MRFEProjectSearchResult = { slug: string; title: string; description: string; categories: string[] | null; client_side: MRFEProjectSupportRange; server_side: MRFEProjectSupportRange; project_type: MRFEProjectType; downloads: number; icon_url: string | null; color: number | null; project_id: string; author: string; display_categories: string[] | null; versions: string[]; follows: number; date_created: string; date_modified: string; latest_version: string | null; license: string; gallery: string[] | null; featured_gallery: string | null }

export type Progress = "Indeterminate" | { Known: number } | { Failed: FeError }

export type XboxError = "noAccount" | "xboxServicesBanned" | "adultVerificationRequired" | "childAccount" | { unknown: number }

export type Mod = { id: string; filename: string; enabled: boolean; modloaders: FEInstanceModLoaderType[]; metadata: ModFileMetadata }

export type CFFEModFileParameters = { modId: number; fileId: number }

export type CFFEFileRelationType = "embeddedLibrary" | "optionalDependency" | "requiredDependency" | "tool" | "incompatible" | "include"

export type CFFEModAsset = { id: number; modId: number; title: string; description: string; thumbnailUrl: string; url: string }

export type AccountEntry = { username: string; uuid: string; lastUsed: string; type: AccountType }

export type FEManagedJavaVersion = { id: string; name: string; downloadUrl: string; javaVersion: string }

export type InstallMod = { instance_id: FEInstanceId; mod_source: ModSource }

export type UpdateInstance = { instance: FEInstanceId; name?: Set<string> | null; use_loaded_icon?: Set<boolean> | null; notes?: Set<string> | null; version?: Set<string> | null; modloader?: Set<ModLoader | null> | null; global_java_args?: Set<boolean> | null; extra_java_args?: Set<string | null> | null; memory?: Set<MemoryRange | null> | null }

export type CFFEFilesParameters = { body: CFFEFilesParametersBody }

export type CreateInstanceVersion = { Version: GameVersion } | { Modpack: Modpack }

export type ListInstanceStatus = { Valid: ValidListInstance } | { Invalid: InvalidListInstance }

export type CFFEPagination = { index: number; pageSize: number; resultCount: number; totalCount: number }

export type CFFEModSearchParameters = { query: CFFEModSearchParametersQuery }

export type CFFECategory = { id: number; name: string; slug: string; url: string; iconUrl: string; dateModified: string; isClass: boolean | null; classId: number | null; parentCategoryId: number | null; displayIndex: number | null }

export type FEUnifiedModLoaderType = "forge" | "fabric" | "quilt" | "liteloader" | "cauldron" | "bukkit" | "bungeecord" | "canvas" | "datapack" | "folia" | "iris" | "minecraft" | "modloader" | "optifine" | "paper" | "purpur" | "rift" | "spigot" | "sponge" | "vanilla" | "velocity" | "waterfall" | { other: string }

export type FETaskId = number

export type MRFEProjectType = "mod" | "shader" | "modpack" | "resourcepack"

export type CFFEClassId = "mods" | "modpacks"

export type MRFEVersion = { name: string; version_number: string; changelog: string | null; dependencies: MRFEDependency[]; game_versions: string[]; version_type: MRFEVersionType; loaders: string[]; featured: boolean; status: MRFEStatus | null; requested_status: MRFERequestedVersionStatus | null; id: string; project_id: string; author_id: string; date_published: string; downloads: number; files: MRFEVersionFile[] }

export type FEUnifiedSearchResponse = { searchApi: FESearchAPI; data: FEUnifiedSearchResult[]; pagination: FEUnifiedPagination | null }

export type MRFEVersionsResponse = MRFEVersion[]

export type FEInstanceModLoaderType = "forge" | "fabric" | "quilt" | "unknown"

export type MRFESearchFacetOr = MRFESearchFacet[]

export type ModFileMetadata = { modid: string; name: string | null; version: string | null; description: string | null; authors: string | null; modloaders: FEInstanceModLoaderType[] | null }

export type ConfigurationParseErrorType = "Syntax" | "Data" | "Eof"

export type FEManagedJavaArchMap = { [key: FEManagedJavaArch]: FEManagedJavaVersion[] }

export type InstanceDetails = { name: string; favorite: boolean; version: string | null; modpack: Modpack | null; global_java_args: boolean; extra_java_args: string | null; memory: MemoryRange | null; last_played: string; seconds_played: number; modloaders: ModLoader[]; notes: string; state: LaunchState; mods: Mod[]; icon_revision: number }

export type CFFEModAuthor = { id: number; name: string; url: string }

export type InvalidListInstance = "JsonMissing" | { JsonError: ConfigurationParseError } | { Other: string }

export type McType = "old_alpha" | "old_beta" | "release" | "snapshot"

export type CFFEFileModule = { name: string; fingerprint: string }

export type CFFEFilesParametersBody = { fileIds: number[] }

/**
 * An image that have been uploaded to a project's gallery
 */
export type MRFEGalleryItem = { url: string; featured: boolean; title: string | null; description: string | null; created: string; ordering: number }

export type FEUnifiedSearchCategoryID = { curseforge: number } | { modrinth: string }

export type FESearchAPI = "curseforge" | "modrinth"

export type MoveInstanceTarget = { BeforeInstance: FEInstanceId } | { BeginningOfGroup: FEGroupId } | { EndOfGroup: FEGroupId }

export type UngroupedInstance = ({ id: FEInstanceId; name: string; favorite: boolean; status: ListInstanceStatus; icon_revision: number }) & { favorite: boolean }

export type Translation = { translation: "ModCacheTaskUpdate" } | { translation: "ModCacheTaskUpdateScanFiles" } | { translation: "ModCacheTaskUpdateQueryApis" } | { translation: "InstanceTaskLaunch"; args: { name: string } } | { translation: "InstanceTaskPrepare"; args: { name: string } } | { translation: "InstanceTaskLaunchWaiting" } | { translation: "InstanceTaskLaunchRequestVersions" } | { translation: "InstanceTaskLaunchRequestModpack" } | { translation: "InstanceTaskLaunchDownloadModpackFiles" } | { translation: "InstanceTaskLaunchExtractModpackFiles" } | { translation: "InstanceTaskLaunchDownloadAddonMetadata" } | { translation: "InstanceTaskLaunchInstallJava" } | { translation: "InstanceTaskLaunchDownloadFiles" } | { translation: "InstanceTaskLaunchExtractNatives" } | { translation: "InstanceTaskLaunchRunForgeProcessors" } | { translation: "InstanceTaskInstallMod"; args: { mod_name: string; instance_name: string } } | { translation: "InstanceTaskInstallModDownloadFile" } | { translation: "FinalizingImport" }

export type FESettingsUpdate = { theme?: string | null; language?: string | null; reducedMotion?: boolean | null; discordIntegration?: boolean | null; releaseChannel?: string | null; concurrentDownloads?: number | null; showNews?: boolean | null; xmx?: number | null; xms?: number | null; isFirstLaunch?: boolean | null; startupResolution?: string | null; javaCustomArgs?: string | null; autoManageJava?: boolean | null; isLegalAccepted?: boolean | null; metricsLevel?: number | null }

export type MRFEProjectSearchResponse = { hits: MRFEProjectSearchResult[]; offset: number; limit: number; total_hits: number }

export type FEEvent = { name: FEEventName; properties: { [key: string]: string } }

export type CFFEModDescriptionParameters = { modId: number }

export type Set<T> = { Set: T }

export type MRFETeamID = string

export type FEGroupId = number

export type ListInstance = { id: FEInstanceId; name: string; favorite: boolean; status: ListInstanceStatus; icon_revision: number }

export type CFFEFileHash = { value: string; algo: CFFEHashAlgo }

export type FEModdedManifest = { gameVersions: FEModdedManifestVersion[] }

export type CFFEModSearchSortField = "featured" | "popularity" | "lastUpdated" | "name" | "author" | "totalDownloads" | "category" | "gameVersion"

export type MRFEAdditionalFileType = "requiredResourcePack" | "optionalResourcePack"

export type CFFEModSearchSortOrder = "ascending" | "descending"

export type CFFEModFilesParametersQuery = { gameVersion?: string | null; modLoaderType?: CFFEModLoaderType | null; gameVersionTypeId?: number | null; index?: number | null; pageSize?: number | null }

export type FEEventName = "AppClosed"

export type FEModFileResponse = { data: CFFEFile; pagination: CFFEPagination | null }

export type MRFEUserRole = "developer" | "moderator" | "admin"

export type Modpack = { Curseforge: CurseforgeModpack } | { Modrinth: ModrinthModpack }

export type AccountStatus = "ok" | "expired" | "refreshing" | "invalid"

export type FEUnifiedModSortIndex = { curseForge: CFFEModSearchSortField } | { modrinth: MRFESearchIndex }

export type FEModResponse = { data: CFFEMod; pagination: CFFEPagination | null }

export type MRFECategory = { icon: string; name: string; project_type: MRFEProjectType; header: string }

export type CFFEFileDependency = { modId: number; relationType: CFFEFileRelationType }

export type MRFERequestedVersionStatus = "listed" | "archived" | "draft" | "unlisted"

export type CFFEFileReleaseType = "stable" | "beta" | "alpha"

export type FEModFilesResponse = { data: CFFEFile[]; pagination: CFFEPagination | null }

export type CreateInstance = { group: FEGroupId; name: string; use_loaded_icon: boolean; version: CreateInstanceVersion; notes: string }

export type MoveInstance = { instance: FEInstanceId; target: MoveInstanceTarget }

export type ValidListInstance = { mc_version: string | null; modloader: FEInstanceModLoaderType | null; modpack_platform: ModpackPlatform | null; state: LaunchState }

export type InstanceMod = { instance_id: FEInstanceId; mod_id: string }

export type MRFEDependencyType = "required" | "optional" | "incompatible" | "embedded"

export type CFFEModParameters = { modId: number }

export type GameLogId = number

export type ManifestVersion = { id: string; type: McType }

export type MRFEProjectSearchParameters = { query: string | null; facets: MRFESearchFacetAnd | null; index: MRFESearchIndex | null; offset: number | null; limit: number | null; filters: string | null }

export type MRFETeamMember = { team_id: string; user: MRFEUser; role: string; ordering: number | null }

export type FEPageview = { path: string }

export type InvalidationEvent = { key: string; args: any | null }

export type MRFEDonationLink = { id: string; platform: string; url: string }

export type MRFEVersionID = string

export type MRFESearchFacetAnd = MRFESearchFacetOr[]

export type FEJavaComponent = { id: string; path: string; version: string; type: FEJavaComponentType; isValid: boolean }

export type InstanceFolder = "Root" | "Data" | "Mods" | "Configs" | "Screenshots" | "Saves" | "Logs" | "CrashReports" | "ResourcePacks" | "TexturePacks" | "ShaderPacks"

export type MRFEProjectsResponse = MRFEProject[]

export type FESubtaskProgress = { download: { downloaded: number; total: number } } | { item: { current: number; total: number } } | "opaque"

export type CauseSegment = { display: string; debug: string }

export type EnrollmentStatus = "requestingCode" | { pollingCode: DeviceCode } | "queryingAccount" | { complete: AccountEntry } | { failed: EnrollmentError }

export type FEModDescriptionResponse = { data: string; pagination: CFFEPagination | null }

export type CFFEModsParameters = { body: CFFEModsParametersBody }

export type MRFEProjectIDs = string[]

export type DeviceCode = { userCode: string; verificationUri: string; expiresAt: string }

export type MRFEStatus = "listed" | "archived" | "draft" | "unlisted" | "scheduled" | "unknown"

export type MRFELoadersResponse = MRFELoader[]

export type MRFEProjectID = string

export type FEModSearchResponse = { data: CFFEMod[]; pagination: CFFEPagination | null }

export type CFFEModFilesParameters = { modId: number; query: CFFEModFilesParametersQuery }

export type ConfigurationParseError = { type_: ConfigurationParseErrorType; message: string; line: number; config_text: string }

export type CFFEHashAlgo = "sha1" | "md5"

export type MoveGroup = { group: FEGroupId; before: FEGroupId | null }

export type MRFECategoriesResponse = MRFECategory[]

export type StandardVersion = { release: string; modloaders: ModLoader[] }

export type ModrinthModpack = { project_id: string; version_id: string }

export type CFFEFileIndex = { gameVersion: string; fileId: number; filename: string; releaseType: CFFEFileReleaseType; gameVersionTypeId: number | null; modLoader: CFFEModLoaderType | null }

export type FECategoriesResponse = { data: CFFECategory[]; pagination: CFFEPagination | null }

export type FEUnifiedSearchParameters = { searchQuery: string | null; categories: And<FEUnifiedSearchCategoryID> | null; gameVersions: Or<string> | null; modloaders: Or<FEUnifiedModLoaderType> | null; projectType: FEUnifiedSearchType | null; sortIndex: FEUnifiedModSortIndex | null; sortOrder: CFFEModSearchSortOrder | null; index: number | null; pageSize: number | null; searchApi: FESearchAPI }

export type CFFEMod = { id: number; gameId: number; name: string; slug: string; links: CFFEModLinks; summary: string; status: CFFEModStatus; downloadCount: number; isFeatured: boolean; primaryCategoryId: number; categories: CFFECategory[]; classId: number | null; authors: CFFEModAuthor[]; logo: CFFEModAsset; screenshots: CFFEModAsset[]; mainFileId: number; latestFiles: CFFEFile[]; latestFilesIndexes: CFFEFileIndex[]; dateCreated: string; dateModified: string; dateReleased: string; allowModDistribution: boolean | null; gamePopularityRank: number; isAvailable: boolean; thumbsUpCount: number }

export type CFFESortableGameVersion = { gameVersionName: string; gameVersionPadded: string; gameVersion: string; gameVersionReleaseDate: string; gameVersionTypeId: number | null }

export type FeError = { cause: CauseSegment[]; backtrace: string }

export type FEJavaComponentType = "local" | "managed" | "custom"

export type CFFEFileStatus = "processing" | "changesRequired" | "underReview" | "approved" | "rejected" | "malwareDetected" | "deleted" | "archived" | "testing" | "released" | "readyForReview" | "deprecated" | "baking" | "awaitingPublishing" | "failedPublishing"

export type FEVendor = "azul"

export type MRFETeamResponse = MRFETeamMember[]

export type MRFESearchFacet = { Category: string } | { Version: string } | { License: string } | { ProjectType: string }

export type MRFELoader = { icon: string; name: MRFELoaderType; supported_project_types: MRFEProjectType[] }

export type CFFEModsParametersBody = { modIds: number[] }

export type FEFilesResponse = { data: CFFEFile[]; pagination: CFFEPagination | null }

export type FEInstanceId = number

export type CFFEFile = { id: number; gameId: number; modId: number; isAvailable: boolean; displayName: string; fileName: string; releaseType: CFFEFileReleaseType; fileStatus: CFFEFileStatus; hashes: CFFEFileHash[]; fileDate: string; fileLength: number; downloadCount: number; downloadUrl: string | null; gameVersions: string[]; sortableGameVersions: CFFESortableGameVersion[]; dependencies: CFFEFileDependency[]; exposeAsAlternative: boolean | null; parentProjectFileId: number | null; alternateFileId: number | null; isServerPack: boolean | null; serverPackFileId: number | null; isEarlyAccessContent: boolean | null; earlyAccessEndDate: string | null; fileFingerprint: string; modules: CFFEFileModule[] }

export type FEModFileChangelogResponse = { data: string; pagination: CFFEPagination | null }

export type FESystemJavaProfileName = "legacy" | "alpha" | "beta" | "gamma" | "minecraftJavaExe"

export type FEImportableInstance = { name: string }

export type MRFEProjectStatus = "approved" | "rejected" | "draft" | "unlisted" | "archived" | "processing" | "unknown"

export type Or<T> = T[]

export type ModrinthMod = { project_id: string; version_id: string }

export type FEModdedManifestLoaderVersion = { id: string }

export type MRFELoaderType = "bukkit" | "bungeecord" | "canvas" | "datapack" | "fabric" | "folia" | "forge" | "iris" | "liteloader" | "minecraft" | "modloader" | "optifine" | "paper" | "purpur" | "quilt" | "rift" | "spigot" | "sponge" | "vanilla" | "velocity" | "waterfall" | { other: string }

export type DuplicateInstance = { instance: FEInstanceId; new_name: string }

export type FEModdedManifestVersion = { id: string; stable: boolean; loaders: FEModdedManifestLoaderVersion[] }

export type MRFEVersionFile = { hashes: MRFEHashes; url: string; filename: string; primary: boolean; size: number; file_type: MRFEAdditionalFileType | null }

export type FEModsResponse = { data: CFFEMod[]; pagination: CFFEPagination | null }

export type CFFEModSearchParametersQuery = { gameId: number; searchFilter: string | null; gameVersion: string | null; categoryIds: number[] | null; sortOrder: CFFEModSearchSortOrder | null; sortField: CFFEModSearchSortField | null; classId: CFFEClassId | null; modLoaderTypes: CFFEModLoaderType[] | null; gameVersionTypeId: number | null; authorId: number | null; slug: string | null; index: number | null; pageSize: number | null }

export type FEUnifiedSearchResult = { curseforge: CFFEMod } | { modrinth: MRFEProjectSearchResult }

export type CFFEModLoaderType = "forge" | "cauldron" | "liteLoader" | "fabric" | "quilt"

export type CFFEModFileChangelogParameters = { modId: number; fileId: number }

export type GameLogEntry = { id: GameLogId; instance_id: FEInstanceId; active: boolean }

export type MRFEProjectSupportRange = "required" | "optional" | "unsupported" | "unknown"

export type AccountType = "microsoft" | "offline"

export type FEManagedJavaSetupProgress = "idle" | { downloading: [string, string] } | { extracting: [string, string] } | "done"

export type MemoryRange = { min_mb: number; max_mb: number }
