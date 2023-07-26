use daedalus::minecraft::{
    Argument, ArgumentValue, AssetsIndex, Download, Library, Os, OsRule, Rule, RuleAction,
};
use std::path::PathBuf;
use sysinfo::SystemExt;

use crate::domain::{
    java::JavaArch,
    runtime_path::{AssetsPath, LibrariesPath, RuntimePath},
};

pub fn libraries_into_vec_downloadable(
    libraries: &[Library],
    base_path: &std::path::Path,
    java_arch: &JavaArch,
) -> Vec<carbon_net::Downloadable> {
    let mut files = vec![];

    for library in libraries {
        if !library_is_allowed(library, java_arch) {
            continue;
        }

        if let Some(downloadable) = library_into_lib_downloadable(library.clone(), base_path) {
            files.push(downloadable);
        }

        if let Some(downloadable) =
            library_into_natives_downloadable(library.clone(), base_path, java_arch)
        {
            files.push(downloadable);
        }

        // Forge special case where downloads is not present but `url` defines the base url
        if let Some(base_url) = &library.url {
            let checksum = None;

            let maven_path = library.name.into_path();
            let Ok(maven_url) = library.name.into_url(base_url) else {
                continue
            };

            files.push(carbon_net::Downloadable {
                url: maven_url.to_string(),
                path: PathBuf::from(base_path).join(maven_path),
                checksum,
                size: None,
            });
        }
    }

    files
}

// Use java arch instead of system arch
pub fn is_rule_allowed(rule: &Rule, java_arch: &JavaArch) -> bool {
    let res = match rule {
        Rule {
            os: Some(ref os), ..
        } => os_rule(os, java_arch),
        Rule {
            features: Some(_), ..
        } => false,
        _ => true,
    };

    match rule.action {
        RuleAction::Allow => res,
        RuleAction::Disallow => !res,
    }
}

pub fn os_rule(rule: &OsRule, java_arch: &JavaArch) -> bool {
    let mut rule_match = true;

    if let Some(ref arch) = rule.arch {
        rule_match &= !matches!(arch.as_str(), "x86" | "arm");
    }

    if let Some(name) = &rule.name {
        rule_match &= &Os::native_arch(java_arch) == name;
    }

    if let Some(version) = &rule.version {
        let system = sysinfo::System::new();
        let Some(os_version) = system.os_version() else {
            return true;
        };

        if let Ok(regex) = regex::Regex::new(version.as_str()) {
            rule_match &= regex.is_match(&os_version);
        }
    }

    rule_match
}

pub fn library_into_lib_downloadable(
    library: Library,
    base_path: &std::path::Path,
) -> Option<carbon_net::Downloadable> {
    let artifact = library.downloads.and_then(|v| v.artifact);

    if let Some(artifact) = artifact {
        let checksum = Some(carbon_net::Checksum::Sha1(artifact.sha1));

        return Some(carbon_net::Downloadable {
            url: artifact.url,
            path: PathBuf::from(base_path).join(artifact.path),
            checksum,
            size: Some(artifact.size as u64),
        });
    } else if let Some(base_url) = &library.url {
        return Some(carbon_net::Downloadable {
            url: format!("{}{}", base_url, library.name.path()),
            path: base_path.join(library.name.path()),
            checksum: None,
            size: None,
        });
    }
    None
}

pub fn library_into_natives_downloadable(
    library: Library,
    base_path: &std::path::Path,
    java_arch: &JavaArch,
) -> Option<carbon_net::Downloadable> {
    let Some(classifiers) = library.downloads.and_then(|v| v.classifiers) else {
        return None;
    };

    let Some(natives) = library.natives else {
        return None;
    };

    let Some(natives_name) = natives.get(&Os::native_arch(java_arch)) else {
        return None;
    };

    let Some(mapping_class) = classifiers.get(&natives_name.replace("${arch}", ARCH_WIDTH)) else {
        return None;
    };

    let checksum = Some(carbon_net::Checksum::Sha1(mapping_class.clone().sha1));

    Some(carbon_net::Downloadable {
        url: mapping_class.url.clone(),
        path: PathBuf::from(base_path).join(mapping_class.clone().path),
        checksum,
        size: Some(mapping_class.size as u64),
    })
}

pub fn version_download_into_downloadable(
    version_download: Download,
    version_id: &str,
    runtime_path: &RuntimePath,
) -> carbon_net::Downloadable {
    let jar_path = runtime_path.get_libraries().get_mc_client(version_id);

    carbon_net::Downloadable::new(version_download.url, jar_path)
        .with_checksum(Some(carbon_net::Checksum::Sha1(version_download.sha1)))
        .with_size(version_download.size as u64)
}

pub fn chain_lwjgl_libs_with_base_libs(
    lwjgl_libs: &[Library],
    all_libs: &[Library],
    java_component_arch: &JavaArch,
    libraries_path: &LibrariesPath,
    only_classpath_visible: bool,
) -> Vec<String> {
    let mut libraries = all_libs
        .iter()
        .chain(lwjgl_libs.iter())
        .filter_map(|library| {
            if !library_is_allowed(library, java_component_arch)
                || (only_classpath_visible && !library.include_in_classpath)
            {
                return None;
            }

            let path = libraries_path.get_library_path({
                if let Some(downloads) = library.downloads.as_ref() {
                    if let Some(artifact) = downloads.artifact.as_ref() {
                        artifact.path.clone()
                    } else if let Some(classifiers) = downloads.classifiers.as_ref() {
                        let Some(native_name) = library
                        .natives
                        .as_ref()
                        .and_then(|natives| natives.get(&Os::native())) else {
                            return None;
                        };

                        classifiers
                            .get(&native_name.replace("${arch}", ARCH_WIDTH))
                            .unwrap()
                            .path
                            .clone()
                    } else {
                        panic!("Library has no artifact or classifier");
                    }
                } else if library.url.is_some() {
                    library.name.into_path().to_string_lossy().to_string()
                } else {
                    panic!("Library has no method of retrieval");
                }
            });

            Some(path.display().to_string())
        })
        .collect::<Vec<String>>();

    libraries.dedup();

    libraries
}

pub fn assets_index_into_vec_downloadable(
    assets_index: AssetsIndex,
    assets_path: &AssetsPath,
) -> Vec<carbon_net::Downloadable> {
    let mut files: Vec<carbon_net::Downloadable> = vec![];

    for (key, object) in assets_index.objects.iter() {
        // TODO: handle directories for different versions (virtual legacy)
        let asset_path = assets_path
            .get_objects_path()
            .join(&object.hash[0..2])
            .join(&object.hash);
        let _virtual_asset_path = assets_path.get_legacy_path().join(key);

        files.push(
            carbon_net::Downloadable::new(
                format!(
                    "https://resources.download.minecraft.net/{}/{}",
                    &object.hash[0..2],
                    &object.hash
                ),
                asset_path,
            )
            .with_checksum(Some(carbon_net::Checksum::Sha1(object.hash.clone())))
            .with_size(object.size as u64),
        );
        // files.push(
        //     carbon_net::Downloadable::new(
        //         format!(
        //             "https://resources.download.minecraft.net/{}/{}",
        //             &object.hash[0..2],
        //             &object.hash
        //         ),
        //         virtual_asset_path,
        //     )
        //     .with_checksum(Some(carbon_net::Checksum::Sha1(object.hash.clone())))
        //     .with_size(object.size as u64),
        // );
    }

    files
}

pub fn library_is_allowed(library: &Library, java_arch: &JavaArch) -> bool {
    let Some(rules) = library.rules.as_ref() else {
        return true;
    };

    let mut is_allowed = false;
    for rule in rules {
        match is_allowed {
            true if rule.action == RuleAction::Disallow => {
                is_allowed = is_rule_allowed(rule, java_arch)
            }
            true => continue,
            false => is_allowed = is_rule_allowed(rule, java_arch),
        }
    }

    is_allowed
}

#[cfg(target_pointer_width = "64")]
pub const ARCH_WIDTH: &str = "64";

#[cfg(target_pointer_width = "32")]
pub const ARCH_WIDTH: &str = "32";

pub fn get_default_jvm_args() -> Vec<Argument> {
    vec![
        Argument::Ruled {
            rules: vec![Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::Osx),
                    version: None,
                    arch: None,
                }),
                features: None,
            }, Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::OsxArm64),
                    version: None,
                    arch: None,
                }),
                features: None,
            }],
            value: ArgumentValue::Single("-XstartOnFirstThread".to_string()),
        },
        Argument::Ruled {
            rules: vec![Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::Windows),
                    version: None,
                    arch: None,
                }),
                features: None,
            }, Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::WindowsArm64),
                    version: None,
                    arch: None,
                }),
                features: None,
            }],
            value: ArgumentValue::Single("-XX:HeapDumpPath=MojangTricksIntelDriversForPerformance_javaw.exe_minecraft.exe.heapdump".to_string()),
        },
        Argument::Ruled {
            rules: vec![Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::Windows),
                    version: Some(r#"^10\\."#.to_string()),
                    arch: None,
                }),
                features: None,
            }, Rule {
                action: RuleAction::Allow,
                os: Some(OsRule {
                    name: Some(Os::WindowsArm64),
                    version: Some(r#"^10\\."#.to_string()),
                    arch: None,
                }),
                features: None,
            }],
            value: ArgumentValue::Many(vec![
                "-Dos.name=Windows 10".to_string(),
                "-Dos.version=10.0".to_string(),
            ]),
        },
        Argument::Normal("-Dfml.ignoreInvalidMinecraftCertificates=true".to_string()),
        Argument::Normal("-Djava.library.path=${natives_directory}".to_string()),
        Argument::Normal("-Dminecraft.launcher.brand=${launcher_name}".to_string()),
        Argument::Normal("-Dminecraft.launcher.version=${launcher_version}".to_string()),

        // Apparently this "hack" is only needed for launcherVersion < 18
        // Argument::String(
        //     "-Dminecraft.applet.TargetDirectory=${game_directory}".to_string(),
        // ),
        Argument::Normal("-cp".to_string()),
        Argument::Normal("${classpath}".to_string()),
    ]
}

pub trait OsExt {
    fn native() -> Self;
    fn native_arch(java_arch: &JavaArch) -> Self;
}

impl OsExt for Os {
    fn native_arch(java_arch: &JavaArch) -> Self {
        if std::env::consts::OS == "windows" {
            if java_arch == &JavaArch::Arm64 {
                Os::WindowsArm64
            } else {
                Os::Windows
            }
        } else if std::env::consts::OS == "linux" {
            if java_arch == &JavaArch::Arm64 {
                Os::LinuxArm64
            } else if java_arch == &JavaArch::Arm32 {
                Os::LinuxArm32
            } else {
                Os::Linux
            }
        } else if std::env::consts::OS == "macos" {
            if java_arch == &JavaArch::Arm64 {
                Os::OsxArm64
            } else {
                Os::Osx
            }
        } else {
            Os::Unknown
        }
    }

    fn native() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "macos" => Self::Osx,
            "linux" => Self::Linux,
            _ => Self::Unknown,
        }
    }
}
