use std::fmt::Display;

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
        GET_AVAILABLE                               = "getAvailable";
        SET_DEFAULT                                 = "setDefault";
        SETUP_CONTROLLED                            = "setupControlled";
        GET_CONTROLLED_INSTALL_STATUS               = "getControlledInstallStatus";
        DELETE_CONTROLLED                           = "deleteControlled";
    }

    mc {
        GET_MINECRAFT_VERSIONS                      = "getMinecraftVersions";
        GET_FORGE_VERSIONS                          = "getForgeVersions";
    }

    instance {
        DEFAULT_GROUP                               = "getDefaultGroup";
        GET_GROUPS                                  = "getGroups";
        GET_INSTANCES_UNGROUPED                     = "getInstancesUngrouped";
        CREATE_GROUP                                = "createGroup";
        CREATE_INSTANCE                             = "createInstance";
        DELETE_GROUP                                = "deleteGroup";
        DELETE_INSTANCE                             = "deleteInstance";
        MOVE_GROUP                                  = "moveGroup";
        MOVE_INSTANCE                               = "moveInstance";
        UPDATE_INSTANCE                             = "updateInstance";
        SET_FAVORITE                                = "setFavorite";
        INSTANCE_DETAILS                            = "getInstanceDetails";
        PREPARE_INSTANCE                            = "prepareInstance";
        LAUNCH_INSTANCE                             = "launchInstance";
        KILL_INSTANCE                               = "killInstance";
        OPEN_INSTANCE_FOLDER                        = "openInstanceFolder";
        ENABLE_MOD                                  = "enableMod";
        DISABLE_MOD                                 = "disableMod";
        DELETE_MOD                                  = "deleteMod";
    }

    instance {
        DEFAULT_GROUP                               = "getDefaultGroup";
        GET_GROUPS                                  = "getGroups";
        GET_INSTANCES_UNGROUPED                     = "getInstancesUngrouped";
        CREATE_GROUP                                = "createGroup";
        CREATE_INSTANCE                             = "createInstance";
        DELETE_GROUP                                = "deleteGroup";
        DELETE_INSTANCE                             = "deleteInstance";
        MOVE_GROUP                                  = "moveGroup";
        MOVE_INSTANCE                               = "moveInstance";
        UPDATE_INSTANCE                             = "updateInstance";
        SET_FAVORITE                                = "setFavorite";
        INSTANCE_DETAILS                            = "getInstanceDetails";
        PREPARE_INSTANCE                            = "prepareInstance";
        LAUNCH_INSTANCE                             = "launchInstance";
        KILL_INSTANCE                               = "killInstance";
        OPEN_INSTANCE_FOLDER                        = "openInstanceFolder";
        ENABLE_MOD                                  = "enableMod";
        DISABLE_MOD                                 = "disableMod";
        DELETE_MOD                                  = "deleteMod";
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
