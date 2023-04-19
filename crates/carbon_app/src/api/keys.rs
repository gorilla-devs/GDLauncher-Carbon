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
        GET_ACTIVE_UUID                  = "getActiveUuid";
        SET_ACTIVE_UUID                  = "setActiveUuid";
        GET_ACCOUNTS                     = "getAccounts";
        GET_ACCOUNT_STATUS               = "getAccountStatus";
        DELETE_ACCOUNT                   = "deleteAccount";
        ENROLL_BEGIN                     = "enroll.begin";
        ENROLL_CANCEL                    = "enroll.cancel";
        ENROLL_GET_STATUS                = "enroll.getStatus";
        ENROLL_FINALIZE                  = "enroll.finalize";
        REFRESH_ACCOUNT                  = "refreshAccount";
        GET_HEAD                         = "getHead";
    }

    java {
        GET_AVAILABLE                    = "getAvailable";
        SET_DEFAULT                      = "setDefault";
        SETUP_CONTROLLED                 = "setupControlled";
        GET_CONTROLLED_INSTALL_STATUS    = "getControlledInstallStatus";
        DELETE_CONTROLLED                = "deleteControlled";
    }

    mc {
        GET_INSTANCES                    = "getInstances";
        GET_MINECRAFT_VERSIONS           = "getMinecraftVersions";
        GET_INSTANCE_DETAILS             = "getInstanceDetails";
        OPEN_INSTANCE_FOLDER_PATH        = "openInstanceFolderPath";
        START_INSTANCE                   = "startInstance";
        STOP_INSTANCE                    = "stopInstance";
        DELETE_INSTANCE                  = "deleteInstance";
        ENABLE_MOD                       = "enableMod";
        DISABLE_MOD                      = "disableMod";
        REMOVE_MOD                       = "removeMod";
        REMOVE_MODS                      = "removeMods";
        SWITCH_MINECRAFT_VERSION         = "switchMinecraftVersion";
        SWITCH_MODLOADER                 = "switchModloader";
        SWITCH_MODLOADER_VERSION         = "switchModloaderVersion";
        UPDATE_INSTANCE_NAME             = "updateInstanceName";
        GET_INSTANCE_MEMORY              = "getInstanceMemory";
        UPDATE_INSTANCE_MEMORY           = "updateInstanceMemory";
        GET_INSTANCE_JAVA_ARGS           = "getInstanceJavaArgs";
        UPDATE_INSTANCE_JAVA_ARGS        = "updateInstanceJavaArgs";
    }

    vtask {
        GET_TASKS                        = "getTasks";
    }

    settings {
        GET_SETTINGS                     = "getSettings";
        SET_SETTINGS                     = "setSettings";
    }

    metrics {
        SEND_EVENT                       = "sendEvent";
        SEND_PAGEVIEW                    = "sendPageview";
    }

    modplatforms {
        TEST_QUERY                      = "testQuery";
    }
}
