macro_rules! keys {
    {$($group:ident { $($name:ident = $value:literal;)* })*} => {
        $(pub mod $group {
            pub const GROUP_PREFIX: &'static str = concat!(stringify!($group), ".");

            $(
                pub const $name: $crate::api::keys::Key = $crate::api::keys::Key {
                    local: $value,
                    full: concat!(stringify!($group), ".", $value),
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
    }

    mc {
        GET_FORGE_VERSIONS                          = "getForgeVersions";
        GET_INSTANCES                               = "getInstances";
        GET_MINECRAFT_VERSIONS                      = "getMinecraftVersions";
        GET_INSTANCE_DETAILS                        = "getInstanceDetails";
        OPEN_INSTANCE_FOLDER_PATH                   = "openInstanceFolderPath";
        START_INSTANCE                              = "startInstance";
        STOP_INSTANCE                               = "stopInstance";
        DELETE_INSTANCE                             = "deleteInstance";
        ENABLE_MOD                                  = "enableMod";
        DISABLE_MOD                                 = "disableMod";
        REMOVE_MOD                                  = "removeMod";
        REMOVE_MODS                                 = "removeMods";
        SWITCH_MINECRAFT_VERSION                    = "switchMinecraftVersion";
        SWITCH_MODLOADER                            = "switchModloader";
        SWITCH_MODLOADER_VERSION                    = "switchModloaderVersion";
        UPDATE_INSTANCE_NAME                        = "updateInstanceName";
        GET_INSTANCE_MEMORY                         = "getInstanceMemory";
        UPDATE_INSTANCE_MEMORY                      = "updateInstanceMemory";
        GET_INSTANCE_JAVA_ARGS                      = "getInstanceJavaArgs";
        UPDATE_INSTANCE_JAVA_ARGS                   = "updateInstanceJavaArgs";
    }

    vtask {
        GET_TASKS                                   = "getTasks";
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
        CURSEFORGE_GET_CATEGORIES                   = "curseforgeGetCategories";
        CURSEFORGE_SEARCH                           = "curseforgeSearch";
        CURSEFORGE_GET_MOD                          = "curseforgeGetMod";
        CURSEFORGE_GET_MODS                         = "curseforgeGetMods";
        CURSEFORGE_GET_MOD_DESCRIPTION              = "curseforgeGetModDescription";
        CURSEFORGE_GET_MOD_FILE                     = "curseforgeGetModFile";
        CURSEFORGE_GET_MOD_FILES                    = "curseforgeGetModFiles";
        CURSEFORGE_GET_FILES                        = "curseforgeGetFiles";
        CURSEFORGE_GET_MOD_FILE_CHANGELOG           = "curseforgeGetModFileChangelog";
    }
}
