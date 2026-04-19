use std::fmt::Display;

macro_rules! keys {
    {$($group:ident { $($name:ident = $value:literal;)* })*} => {
        $(pub mod $group {
            pub const GROUP_PREFIX: &'static str = concat!(stringify!($group), ".");

            $(
                pub const $name: $crate::api::keys::Key = $crate::api::keys::Key {
                    local: $value,
                    full: concat!(stringify!($group), ".", $value),
                    span_key: concat!("api::", stringify!($group), ".", $value)
                };
            )*
        })*
    }
}

/// Api endpoint keys
#[derive(Copy, Clone)]
pub struct Key {
    /// local keypoath `mykey`
    pub local: &'static str,
    /// full keypath `mygroup.mykey`
    pub full: &'static str,
    /// span key `api::mygroup.mykey`
    pub span_key: &'static str,
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full)
    }
}

keys! {
    account {
        GET_ACTIVE_UUID                             = "getActiveUuid";
        SET_ACTIVE_UUID                             = "setActiveUuid";
        GET_ACCOUNTS                                = "getAccounts";
        DELETE_ACCOUNT                              = "deleteAccount";
        ENROLL_BEGIN                                = "enroll.begin";
        ENROLL_BEGIN_BROWSER                        = "enroll.beginBrowser";
        ENROLL_PROTOCOL_CALLBACK                    = "enroll.protocolCallback";
        ENROLL_CANCEL                               = "enroll.cancel";
        ENROLL_GET_STATUS                           = "enroll.getStatus";
        ENROLL_FINALIZE                             = "enroll.finalize";
        ENROLL_RESUME                               = "enroll.resume";
        REFRESH_ACCOUNT                             = "refreshAccount";
        GET_HEAD                                    = "getHead";

        PEEK_GDL_ACCOUNT                            = "peekGdlAccount";
        GET_GDL_ACCOUNT                             = "getGdlAccount";
        REGISTER_GDL_ACCOUNT                        = "registerGdlAccount";
        REQUEST_NEW_VERIFICATION_TOKEN              = "requestNewVerificationToken";
        REQUEST_EMAIL_CHANGE                        = "requestEmailChange";
        REMOVE_GDL_ACCOUNT                          = "removeGdlAccount";
        SAVE_GDL_ACCOUNT                            = "saveGdlAccount";
        REQUEST_GDL_ACCOUNT_DELETION                = "requestGdlAccountDeletion";
        CHANGE_GDL_ACCOUNT_DISPLAY_NAME             = "changeGdlAccountDisplayName";
        UPLOAD_PROFILE_ICON                         = "uploadProfileIcon";
        DELETE_PROFILE_ICON                         = "deleteProfileIcon";
        CHECK_USERNAME_AVAILABLE                    = "checkUsernameAvailable";
        CREATE_PROFILE                              = "createProfile";
        GET_DISPLAY_NAME_HISTORY                    = "getDisplayNameHistory";
        CLEAR_DISPLAY_NAME_HISTORY                  = "clearDisplayNameHistory";
    }

    java {
        GET_AVAILABLE_JAVAS                         = "getAvailableJavas";
        GET_MANAGED_VENDORS                         = "getManagedVendors";
        GET_MANAGED_OS                              = "getManagedOS";
        GET_MANAGED_ARCH                            = "getManagedArch";
        GET_MANAGED_VERSIONS_BY_VENDOR              = "getManagedVersionsByVendor";
        SETUP_MANAGED_JAVA                          = "setupManagedJava";
        GET_SETUP_MANAGED_JAVA_PROGRESS             = "getSetupManagedJavaProgress";
        GET_JAVA_PROFILES                           = "getJavaProfiles";
        UPDATE_JAVA_PROFILE                         = "updateJavaProfile";
        CREATE_JAVA_PROFILE                         = "createJavaProfile";
        DELETE_JAVA_PROFILE                         = "deleteJavaProfile";
        VALIDATE_CUSTOM_JAVA_PATH                   = "validateCustomJavaPath";
        CREATE_CUSTOM_JAVA_VERSION                  = "createCustomJavaVersion";
        DELETE_JAVA_VERSION                         = "deleteJavaVersion";
        SYSTEM_JAVA_PROFILE_ASSIGNMENTS             = "systemJavaProfileAssignments";
    }

    mc {
        GET_MINECRAFT_VERSIONS                      = "getMinecraftVersions";
        GET_FORGE_VERSIONS                          = "getForgeVersions";
        GET_NEOFORGE_VERSIONS                       = "getNeoforgeVersions";
        GET_FABRIC_VERSIONS                         = "getFabricVersions";
        GET_QUILT_VERSIONS                          = "getQuiltVersions";
    }

    instance {
        DEFAULT_GROUP                               = "getDefaultGroup";
        GET_GROUPS                                  = "getGroups";
        GET_ALL_INSTANCES                           = "getAllInstances";
        CREATE_INSTANCE                             = "createInstance";
        CHANGE_MODPACK                              = "changeModpack";
        LOAD_ICON_URL                               = "loadIconUrl";
        DELETE_GROUP                                = "deleteGroup";
        DELETE_GROUP_WITH_INSTANCES                 = "deleteGroupWithInstances";
        RENAME_GROUP                                = "renameGroup";
        DELETE_INSTANCE                             = "deleteInstance";
        MOVE_GROUP                                  = "moveGroup";
        MOVE_INSTANCE                               = "moveInstance";
        CREATE_FOLDER_FROM_INSTANCES                = "createFolderFromInstances";
        ARRANGE_LIBRARY                             = "arrangeLibrary";
        ARRANGE_GROUP                               = "arrangeGroup";
        DUPLICATE_INSTANCE                          = "duplicateInstance";
        UPDATE_INSTANCE                             = "updateInstance";
        SET_FAVORITE                                = "setFavorite";
        INSTANCE_DETAILS                            = "getInstanceDetails";
        INSTANCE_MODS                               = "getInstanceMods";
        PRIORITIZE_INSTANCE_CACHE                   = "prioritizeInstanceCache";
        PREPARE_INSTANCE                            = "prepareInstance";
        LAUNCH_INSTANCE                             = "launchInstance";
        KILL_INSTANCE                               = "killInstance";
        GET_LOGS                                    = "getLogs";
        SEARCH_LOGS                                 = "searchLogs";
        DELETE_LOG                                  = "deleteLog";
        OPEN_LOG_IN_FOLDER                          = "openLogInFolder";
        OPEN_INSTANCE_FOLDER                        = "openInstanceFolder";
        ENABLE_MOD                                  = "enableMod";
        DISABLE_MOD                                 = "disableMod";
        DELETE_MOD                                  = "deleteMod";
        INSTALL_MOD                                 = "installMod";
        INSTALL_LATEST_MOD                          = "installLatestMod";
        UPDATE_MOD                                  = "updateMod";
        FIND_MOD_UPDATE                             = "findModUpdate";
        GET_MOD_SOURCES                             = "getModSources";
        IMPORT_INSTANCE_SHARE_CODE                  = "importInstanceShareCode";
        VALIDATE_SHARE_CODE                         = "validateShareCode";
        GET_SHARE_PREVIEW                           = "getSharePreview";
        GET_IMPORTABLE_ENTITIES                     = "getImportableEntities";
        GET_IMPORT_ENTITY_DEFAULT_PATH              = "getImportEntityDefaultPath";
        SET_IMPORT_SCAN_TARGET                      = "setImportScanTarget";
        CANCEL_IMPORT_SCAN                          = "cancelImportScan";
        GET_IMPORT_SCAN_STATUS                      = "getImportScanStatus";
        IMPORT_INSTANCE                             = "importInstance";
        EXPLORE                                     = "explore";
        EXPORT                                      = "export";
        WAIT_FOR_SHARE_INSTANCE                     = "waitForShareInstance";
        GET_USER_SHARES                             = "getUserShares";
        GET_USER_QUOTA                              = "getUserQuota";
        DELETE_SHARE                                = "deleteShare";
        UPDATE_SHARE                                = "updateShare";
        REGENERATE_SHARE_CODE                       = "regenerateShareCode";
        GET_MODPACK_INFO                            = "getModpackInfo";
    }

    vtask {
        GET_TASKS                                   = "getTasks";
        GET_TASK                                    = "getTask";
        DISMISS_TASK                                = "dismissTask";
    }

    settings {
        GET_SETTINGS                                = "getSettings";
        SET_SETTINGS                                = "setSettings";
        GET_TERMS_OF_SERVICE_BODY                   = "getTermsOfServiceBody";
        GET_PRIVACY_STATEMENT_BODY                  = "getPrivacyStatementBody";
        // First launch
        IS_FIRST_LAUNCH                             = "isFirstLaunch";
        COMPLETE_FIRST_LAUNCH                       = "completeFirstLaunch";
        // Changelog
        SHOULD_SHOW_CHANGELOG                       = "shouldShowChangelog";
        MARK_CHANGELOG_SEEN                         = "markChangelogSeen";
        // Beta prompt
        SHOULD_SHOW_BETA_PROMPT                     = "shouldShowBetaPrompt";
        DISMISS_BETA_PROMPT_PERMANENTLY             = "dismissBetaPromptPermanently";
        REMIND_BETA_PROMPT_LATER                    = "remindBetaPromptLater";
        // Onboarding tips
        GET_SEEN_ONBOARDING_TIPS                    = "getSeenOnboardingTips";
        MARK_ONBOARDING_TIP_SEEN                    = "markOnboardingTipSeen";
        RESET_ONBOARDING_TIPS                       = "resetOnboardingTips";
        // Search sidebar
        GET_SEARCH_SIDEBAR_DOCKED                   = "getSearchSidebarDocked";
        SET_SEARCH_SIDEBAR_DOCKED                   = "setSearchSidebarDocked";
        // Cache cleanup
        GET_TOTAL_CACHE_SIZE                        = "getTotalCacheSize";
        GET_DB_SIZE                                 = "getDbSize";
        GET_CACHE_BREAKDOWN                         = "getCacheBreakdown";
        CLEANUP_CACHES                              = "cleanupCaches";
    }

    metrics {
        SEND_EVENT                                  = "sendEvent";
        SEND_PAGEVIEW                               = "sendPageview";
    }

    systeminfo {
        GET_TOTAL_RAM                               = "getTotalRAM";
        GET_USED_RAM                                = "getUsedRAM";
        GET_AVAILABLE_RAM                           = "getAvailableRAM";
    }

    server {
        GET_DEFAULT_GROUP                           = "getDefaultGroup";
        GET_GROUPS                                  = "getGroups";
        GET_ALL_SERVERS                             = "getAllServers";
        CREATE_SERVER                               = "createServer";
        CREATE_SERVER_FROM_MODPACK                  = "createServerFromModpack";
        DELETE_SERVER                               = "deleteServer";
        ACCEPT_EULA                                 = "acceptEula";
        START_SERVER                                = "startServer";
        STOP_SERVER                                 = "stopServer";
        KILL_SERVER                                 = "killServer";
        SEND_CONSOLE_COMMAND                        = "sendConsoleCommand";
        GET_SERVER_DETAILS                          = "getServerDetails";
        GET_SERVER_METRICS                          = "getServerMetrics";
        SET_FAVORITE                                = "setFavorite";
        UPDATE_SERVER                               = "updateServer";
        SET_SERVER_ICON                             = "setServerIcon";
        MOVE_SERVER                                 = "moveServer";
        MOVE_SERVER_GROUP                           = "moveServerGroup";
        CREATE_FOLDER_FROM_SERVERS                  = "createFolderFromServers";
        ARRANGE_SERVER_LIBRARY                      = "arrangeServerLibrary";
        RENAME_SERVER_GROUP                         = "renameServerGroup";
        DELETE_SERVER_GROUP                         = "deleteServerGroup";
        GET_SERVER_PROPERTIES                       = "getServerProperties";
        UPDATE_SERVER_PROPERTIES                    = "updateServerProperties";
        GET_WHITELIST                               = "getWhitelist";
        ADD_TO_WHITELIST                            = "addToWhitelist";
        REMOVE_FROM_WHITELIST                       = "removeFromWhitelist";
        GET_OPS                                     = "getOps";
        ADD_OP                                      = "addOp";
        REMOVE_OP                                   = "removeOp";
        GET_BANNED_PLAYERS                          = "getBannedPlayers";
        BAN_PLAYER                                  = "banPlayer";
        UNBAN_PLAYER                                = "unbanPlayer";
        GET_BANNED_IPS                              = "getBannedIps";
        BAN_IP                                      = "banIp";
        UNBAN_IP                                    = "unbanIp";
        GET_SERVER_ADDONS                           = "getServerAddons";
        ENABLE_SERVER_ADDON                         = "enableServerAddon";
        DELETE_SERVER_ADDON                         = "deleteServerAddon";
        INSTALL_SERVER_MOD                          = "installServerMod";
        INSTALL_LATEST_SERVER_MOD                   = "installLatestServerMod";
        OPEN_SERVER_FOLDER                          = "openServerFolder";
        PRIORITIZE_SERVER_CACHE                     = "prioritizeServerCache";
    }

    modplatforms {
        CURSEFORGE_SEARCH                           = "curseforge.search";
        CURSEFORGE_GET_MOD                          = "curseforge.getMod";
        CURSEFORGE_GET_MODS                         = "curseforge.getMods";
        CURSEFORGE_GET_MOD_DESCRIPTION              = "curseforge.getModDescription";
        CURSEFORGE_GET_MOD_FILE                     = "curseforge.getModFile";
        CURSEFORGE_GET_MOD_FILES                    = "curseforge.getModFiles";
        CURSEFORGE_GET_FILES                        = "curseforge.getFiles";
        CURSEFORGE_GET_MOD_FILE_CHANGELOG           = "curseforge.getModFileChangelog";
        MODRINTH_SEARCH                             = "modrinth.search";
        MODRINTH_GET_PROJECT                        = "modrinth.getProject";
        MODRINTH_GET_PROJECTS                       = "modrinth.getProjects";
        MODRINTH_GET_PROJECT_VERSIONS               = "modrinth.getProjectVersions";
        MODRINTH_GET_VERSION                        = "modrinth.getVersion";
        MODRINTH_GET_VERSIONS                       = "modrinth.getVersions";
        MODRINTH_GET_PROJECT_TEAM                   = "modrinth.getProjectTeam";
        MODRINTH_GET_TEAM                           = "modrinth.getTeam";

        UNIFIED_SEARCH                              = "unifiedSearch";
        UNIFIED_GET_PROJECT                         = "unifiedGetProject";
        UNIFIED_GET_PROJECTS_BY_IDS                 = "unifiedGetProjectsByIds";
        UNIFIED_GET_PROJECT_VERSIONS                = "unifiedGetProjectVersions";
        UNIFIED_SEARCH_PROJECT_TYPE                 = "unifiedSearchProjectType";
        GET_UNIFIED_CATEGORIES                      = "getUnifiedCategories";
        GET_UNIFIED_MODLOADERS                      = "getUnifiedModloaders";
    }
}
