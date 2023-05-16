use daedalus::minecraft::{
    Argument, ArgumentValue, AssetsIndex, Download, Library, Os, OsRule, Rule, RuleAction,
};
use std::path::{Path, PathBuf};

use crate::domain::maven::MavenCoordinates;

pub fn libraries_into_vec_downloadable(
    libraries: Vec<Library>,
    base_path: &std::path::Path,
) -> Vec<carbon_net::Downloadable> {
    let mut files = vec![];

    for library in libraries {
        if !library_is_allowed(library.clone()) {
            continue;
        }

        if let Some(downloadable) = library_into_lib_downloadable(library.clone(), base_path) {
            files.push(downloadable);
        }

        if let Some(downloadable) = library_into_natives_downloadable(library.clone(), base_path) {
            files.push(downloadable);
        }

        // Forge special case where downloads is not present but `url` defines the base url
        if let Some(base_url) = &library.url {
            let checksum = None;

            // It's ok here to use MavenCoordinates::try_from, since it's the only way to get the path
            let Ok(maven_path) = MavenCoordinates::try_from(library.name, None) else {
                continue
            };

            let maven_path = maven_path.into_path();

            files.push(carbon_net::Downloadable {
                url: format!("{}/{}", base_url, maven_path.to_string_lossy()),
                path: PathBuf::from(base_path).join(maven_path),
                checksum,
                size: None,
            });
        }
    }

    files
}

pub fn is_rule_allowed(rule: Rule) -> bool {
    let current_arch = std::env::consts::ARCH;

    let os = rule.os.as_ref().unwrap_or(&OsRule {
        name: None,
        version: None,
        arch: None,
    });

    let is_os_allowed = os.name.clone().unwrap_or(get_current_os()) == get_current_os()
        || os.name.clone().unwrap_or(get_current_os_base_arch()) == get_current_os_base_arch();

    let is_arch_allowed = os.arch.clone().unwrap_or(current_arch.to_string()) == current_arch;
    let is_feature_allowed = rule.features.is_none();
    // TODO: Check version

    match rule.action {
        RuleAction::Allow => is_os_allowed && is_arch_allowed && is_feature_allowed,
        RuleAction::Disallow => !(is_os_allowed && is_arch_allowed && is_feature_allowed),
    }
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
    }

    None
}

pub fn library_into_natives_downloadable(
    library: Library,
    base_path: &std::path::Path,
) -> Option<carbon_net::Downloadable> {
    let Some(classifiers) = library.downloads.and_then(|v| v.classifiers) else {
        return None;
    };

    let Some(natives) = library.natives else {
        return None;
    };

    let Some(natives_name) = natives.get(&get_current_os()) else {
        return None;
    };

    let Some(mapping_class) = classifiers.get(&natives_name.clone()) else {
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
    base_path: &std::path::Path,
) -> carbon_net::Downloadable {
    let jar_path = base_path.join(format!("{}.jar", &version_download.sha1));

    carbon_net::Downloadable::new(version_download.url, jar_path)
        .with_checksum(Some(carbon_net::Checksum::Sha1(version_download.sha1)))
        .with_size(version_download.size as u64)
}

pub fn assets_index_into_vec_downloadable(
    assets_index: AssetsIndex,
    base_path: &Path,
) -> Vec<carbon_net::Downloadable> {
    let mut files: Vec<carbon_net::Downloadable> = vec![];

    for (_, object) in assets_index.objects.iter() {
        // TODO: handle directories for different versions (virtual legacy)
        let asset_path = base_path.join(&object.hash[0..2]).join(&object.hash);

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
    }

    files
}

pub fn library_is_allowed(library: Library) -> bool {
    let Some(rules) = library.rules else {
        return true;
    };

    for rule in rules {
        if !is_rule_allowed(rule) {
            return false;
        }
    }

    true
}

pub fn get_current_os() -> Os {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "x86_64") {
            Os::Osx
        } else {
            Os::OsxArm64
        }
    }
    #[cfg(target_os = "windows")]
    {
        if cfg!(target_arch = "x86_64") {
            Os::Windows
        } else {
            Os::WindowsArm64
        }
    }
    #[cfg(target_os = "linux")]
    {
        if cfg!(target_arch = "x86_64") {
            Os::Linux
        } else if cfg!(target_arch = "arm64") {
            Os::LinuxArm64
        } else if cfg!(target_arch = "arm") {
            Os::LinuxArm32
        } else {
            panic!("Unsupported architecture")
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        panic!("Unsupported OS")
    }
}

pub fn get_current_os_base_arch() -> Os {
    #[cfg(target_os = "macos")]
    {
        Os::Osx
    }
    #[cfg(target_os = "windows")]
    {
        Os::Windows
    }
    #[cfg(target_os = "linux")]
    {
        Os::Linux
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        panic!("Unsupported OS")
    }
}

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
