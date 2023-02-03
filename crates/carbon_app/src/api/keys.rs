macro_rules! keys {
    {$($group:ident { $($name:ident = $value:literal;)* })*} => {
        $(pub mod $group {
            pub const GROUP_PREFIX: &'static str = concat!(stringify!($group), ".");

            #[doc = "full key names"]
            pub mod full {
                $(pub const $name: &'static str = concat!(stringify!($group), ".", $value);)*
            }

            #[doc = "relative key names"]
            pub mod local {
                $(pub const $name: &'static str = $value;)*
            }
        })*
    }
}

keys! {
    java {
        GET_AVAILABLE                 = "getAvailable";
        SET_DEFAULT                   = "setDefault";
        SETUP_CONTROLLED              = "setupControlled";
        GET_CONTROLLED_INSTALL_STATUS = "getControlledInstallStatus";
        DELETE_CONTROLLED             = "deleteControlled";
    }

    mc {
        GET_INSTANCES                 = "getInstances";
        GET_INSTANCE_DETAILS          = "getInstanceDetails";
        OPEN_INSTANCE_FOLDER_PATH     = "openInstanceFolderPath";
        START_INSTANCE                = "startInstance";
        STOP_INSTANCE                 = "stopInstance";
        DELETE_INSTANCE               = "deleteInstance";
        ENABLE_MOD                    = "enableMod";
        DISABLE_MOD                   = "disableMod";
        REMOVE_MOD                    = "removeMod";
        SWITCH_MINECRAFT_VERSION      = "switchMinecraftVersion";
        SWITCH_MODLOADER              = "switchModloader";
        SWITCH_MODLOADER_VERSION      = "switchModloaderVersion";
        UPDATE_INSTANCE_NAME          = "updateInstanceName";
        GET_INSTANCE_MEMORY           = "getInstanceMemory";
        UPDATE_INSTANCE_MEMORY        = "updateInstanceMemory";
        GET_INSTANCE_JAVA_ARGS        = "getInstanceJavaArgs";
        UPDATE_INSTANCE_JAVA_ARGS     = "updateInstanceJavaArgs";
    }

    app {
        GET_THEME                     = "getTheme";
        SET_THEME                     = "setTheme";
    }
}
