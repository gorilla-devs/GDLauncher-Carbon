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
        GET_ACCOUNT_STATUS                          = "getAccountStatus";
        DELETE_ACCOUNT                              = "deleteAccount";
        ENROLL_BEGIN                                = "enroll.begin";
        ENROLL_CANCEL                               = "enroll.cancel";
        ENROLL_GET_STATUS                           = "enroll.getStatus";
        ENROLL_FINALIZE                             = "enroll.finalize";
        REFRESH_ACCOUNT                             = "refreshAccount";
        GET_HEAD                                    = "getHead";
    }

    java {
        GET_AVAILABLE_JAVAS                         = "getAvailableJavas";
        GET_MANAGED_VENDORS                         = "getManagedVendors";
        GET_MANAGED_OS                              = "getManagedOS";
        GET_MANAGED_ARCH                            = "getManagedArch";
        GET_MANAGED_VERSIONS_BY_VENDOR              = "getManagedVersionsByVendor";
        SETUP_MANAGED_JAVA                          = "setupManagedJava";
        GET_SETUP_MANAGED_JAVA_PROGRESS             = "getSetupManagedJavaProgress";
        GET_SYSTEM_JAVA_PROFILES                    = "getSystemJavaProfiles";
        UPDATE_SYSTEM_JAVA_PROFILE_PATH             = "updateSystemJavaProfilePath";
        DELETE_JAVA_VERSION                         = "deleteJavaVersion";
    }

    mc {
        GET_MINECRAFT_VERSIONS                      = "getMinecraftVersions";
        GET_FORGE_VERSIONS                          = "getForgeVersions";
        GET_FABRIC_VERSIONS                         = "getFabricVersions";
        GET_QUILT_VERSIONS                          = "getQuiltVersions";
    }

    instance {
        DEFAULT_GROUP                               = "getDefaultGroup";
        GET_GROUPS                                  = "getGroups";
        GET_INSTANCES_UNGROUPED                     = "getInstancesUngrouped";
        CREATE_GROUP                                = "createGroup";
        CREATE_INSTANCE                             = "createInstance";
        LOAD_ICON_URL                               = "loadIconUrl";
        DELETE_GROUP                                = "deleteGroup";
        DELETE_INSTANCE                             = "deleteInstance";
        MOVE_GROUP                                  = "moveGroup";
        MOVE_INSTANCE                               = "moveInstance";
        DUPLICATE_INSTANCE                          = "duplicateInstance";
        UPDATE_INSTANCE                             = "updateInstance";
        SET_FAVORITE                                = "setFavorite";
        INSTANCE_DETAILS                            = "getInstanceDetails";
        PREPARE_INSTANCE                            = "prepareInstance";
        LAUNCH_INSTANCE                             = "launchInstance";
        KILL_INSTANCE                               = "killInstance";
        GET_LOGS                                    = "getLogs";
        DELETE_LOG                                  = "deleteLog";
        OPEN_INSTANCE_FOLDER                        = "openInstanceFolder";
        ENABLE_MOD                                  = "enableMod";
        DISABLE_MOD                                 = "disableMod";
        DELETE_MOD                                  = "deleteMod";
        INSTALL_MOD                                 = "installMod";
        GET_IMPORTABLE_ENTITIES                     = "getImportableEntities";
        SCAN_IMPORTABLE_INSTANCES                   = "scanImportableInstances";
        GET_IMPORTABLE_INSTANCES                    = "getImportableInstances";
        IMPORT_INSTANCE                             = "importInstance";
    }

    vtask {
        GET_TASKS                                   = "getTasks";
        GET_TASK                                    = "getTask";
        DISMISS_TASK                                = "dismissTask";
    }

    settings {
        GET_SETTINGS                                = "getSettings";
        SET_SETTINGS                                = "setSettings";
        GET_IS_FIRST_LAUNCH                         = "getIsFirstLaunch";
        SET_IS_FIRST_LAUNCH                         = "setIsFirstLaunch";
    }

    metrics {
        SEND_EVENT                                  = "sendEvent";
        SEND_PAGEVIEW                               = "sendPageview";
    }

    systeminfo {
        GET_TOTAL_RAM                               = "getTotalRAM";
        GET_USED_RAM                                = "getUsedRAM";
    }

    modplatforms {
        CURSEFORGE_GET_MODLOADERS                   = "curseforge.getModloaders";
        CURSEFORGE_GET_CATEGORIES                   = "curseforge.getCategories";
        CURSEFORGE_SEARCH                           = "curseforge.search";
        CURSEFORGE_GET_MOD                          = "curseforge.getMod";
        CURSEFORGE_GET_MODS                         = "curseforge.getMods";
        CURSEFORGE_GET_MOD_DESCRIPTION              = "curseforge.getModDescription";
        CURSEFORGE_GET_MOD_FILE                     = "curseforge.getModFile";
        CURSEFORGE_GET_MOD_FILES                    = "curseforge.getModFiles";
        CURSEFORGE_GET_FILES                        = "curseforge.getFiles";
        CURSEFORGE_GET_MOD_FILE_CHANGELOG           = "curseforge.getModFileChangelog";

        MODRINTH_GET_LOADERS                        = "modrinth.getLoaders";
        MODRINTH_GET_CATEGORIES                     = "modrinth.getCategories";
        MODRINTH_SEARCH                             = "modrinthSearch";
        MODRINTH_GET_PROJECT                        = "modrinth.getProject";
        MODRINTH_GET_PROJECTS                       = "modrinth.getProjects";
        MODRINTH_GET_VERSION                        = "modrinth.getVersion";
        MODRINTH_GET_VERSIONS                       = "modrinth.getVersions";
        MODRINTH_GET_PROJECT_TEAM                   = "modrinth.getProjectTeam";
        MODRINTH_GET_TEAM                           = "modrinth.getTeam";

        UNIFIED_SEARCH                              = "unifiedSearch";
    }
}
