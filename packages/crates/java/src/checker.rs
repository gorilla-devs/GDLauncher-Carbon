use std::path::PathBuf;

use anyhow::{bail, Result};
use tokio::process::Command;

use crate::{utils::{PATH_SEPARATOR, locate_java_check_class, JAVA_CHECK_APP_NAME, parse_java_version, parse_java_arch}, JavaComponent};

async fn load_java_paths_from_env() -> Result<Vec<PathBuf>> {
    let env_path = std::env::var("PATH")?;
    let paths = env_path.split(PATH_SEPARATOR).collect::<Vec<&str>>();
    let mut java_paths = Vec::new();
    for path in paths {
        let path = path.to_string();
        if path.contains("java") {
            java_paths.extend(search_java_binary_in_path(PathBuf::from(path)));
        }
    }

    Ok(java_paths)
}

#[cfg(target_os = "macos")]
pub async fn find_java_paths() -> Vec<PathBuf> {
    let mut javas: Vec<PathBuf> = vec![];
    javas.extend(search_java_binary_in_path(
        PathBuf::from("/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java")
    ));
    javas.extend(search_java_binary_in_path(PathBuf::from(
        "/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home",
    )));
    javas.extend(search_java_binary_in_path(PathBuf::from(
        "/System/Library/Frameworks/JavaVM.framework/Versions/Current/Commands",
    )));
    javas.extend(search_java_binary_in_path(PathBuf::from(
        "/opt/homebrew/opt/openjdk/bin",
    )));
    javas.extend(search_java_binary_in_path(PathBuf::from("/usr/bin")));

    // Library JVM
    let library_jvm_dir = PathBuf::from("/Library/Java/JavaVirtualMachines");
    let library_jvm_javas = std::fs::read_dir(library_jvm_dir);
    if let Ok(library_jvm_javas) = library_jvm_javas {
        for library_jvm_java in library_jvm_javas {
            if let Ok(library_jvm_java) = library_jvm_java {
                let library_jvm_java = library_jvm_java.path();
                javas.extend(
                    vec![
                        search_java_binary_in_path(library_jvm_java.join("Contents/Home")),
                        search_java_binary_in_path(library_jvm_java.join("Contents/Home/jre")),
                    ]
                    .concat(),
                );
            }
        }
    }

    // System Library JVM
    let system_library_jvm_dir = PathBuf::from("/System/Library/Java/JavaVirtualMachines");
    let system_library_jvm_javas = std::fs::read_dir(system_library_jvm_dir);
    if let Ok(system_library_jvm_javas) = system_library_jvm_javas {
        for system_library_jvm_java in system_library_jvm_javas {
            if let Ok(system_library_jvm_java) = system_library_jvm_java {
                let system_library_jvm_java = system_library_jvm_java.path();

                javas.extend(
                    vec![
                        search_java_binary_in_path(system_library_jvm_java.join("Contents/Home")),
                        search_java_binary_in_path(
                            system_library_jvm_java.join("Contents/Commands"),
                        ),
                    ]
                    .concat(),
                );
            }
        }
    }

    let java_from_env = load_java_paths_from_env().await;
    if let Ok(java_from_env) = java_from_env {
        javas.extend(java_from_env);
    }

    // Remove duplicates
    javas.sort_by(|a, b| {
        a.to_string_lossy()
            .to_string()
            .to_lowercase()
            .cmp(&b.to_string_lossy().to_string().to_lowercase())
    });
    javas.dedup();
    javas.into_iter().filter(|java| java.exists()).collect()
}

fn search_java_binary_in_path(path: PathBuf) -> Vec<PathBuf> {
    let mut options = vec![];
    if cfg!(windows) {
        options.push(path.join("bin").join("java.exe"));
        options.push(path.join("java.exe"));
    } else {
        options.push(path.join("bin").join("java"));
        options.push(path.join("java"));
    }

    options
}

#[cfg(target_os = "windows")]
fn read_registry_key(key: &str, value: &str, subkey_suffix: Option<&str>) -> Result<Vec<PathBuf>> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let key_reg = hkcu.open_subkey(key)?;
    let mut results = vec![];

    if let Some(subkey_suffix) = subkey_suffix {
        let subkeys = key_reg.enum_keys();
        for subkey in subkeys {
            if let Ok(subkey) = subkey {
                let joined_subkey = format!("{}\\{}\\{}", key, subkey, subkey_suffix);
                let subkey_reg = hkcu.open_subkey(&joined_subkey)?;
                match subkey_reg.get_value(value) {
                    Ok(value) => {
                        let s_value: String = value;
                        results.extend(search_java_binary_in_path(PathBuf::from(s_value)));
                    }
                    Err(_) => continue,
                };
            }
        }
    } else {
        match key_reg.get_value(value) {
            Ok(value) => {
                let s_value: String = value;
                results.extend(search_java_binary_in_path(PathBuf::from(s_value)));
            }
            Err(_) => {}
        };
    }
    Ok(results)
}

#[cfg(target_os = "windows")]
pub async fn find_java_paths() -> Vec<PathBuf> {
    let mut javas: Vec<PathBuf> = vec![];

    // Load from env
    let java_from_env = load_java_paths_from_env().await;
    if let Ok(java_from_env) = java_from_env {
        javas.extend(java_from_env);
    }

    // Load from registry
    let reg_paths = vec![
        // Oracle
        (
            r"SOFTWARE\JavaSoft\Java Runtime Environment",
            "JavaHome",
            None,
        ),
        (r"SOFTWARE\JavaSoft\Java Development Kit", "JavaHome", None),
        // Oracle for Java 9 and newer
        (r"SOFTWARE\JavaSoft\JRE", "JavaHome", None),
        (r"SOFTWARE\JavaSoft\JDK", "JavaHome", None),
        (r"SOFTWARE\JavaSoft\JDK", "JavaHome", Some(r#"\\"#)),
        // AdoptOpenJDK
        (r"SOFTWARE\AdoptOpenJDK\JRE", "Path", Some(r#"hotspot\MSI"#)),
        (r"SOFTWARE\AdoptOpenJDK\JDK", "Path", Some(r#"hotspot\MSI"#)),
        // Eclipse Foundation
        (
            r"SOFTWARE\Eclipse Foundation\JDK",
            "Path",
            Some(r#"hotspot\MSI"#),
        ),
        // Eclipse Adoptium
        (
            r"SOFTWARE\Eclipse Adoptium\JRE",
            "Path",
            Some(r#"hotspot\MSI"#),
        ),
        (
            r"SOFTWARE\Eclipse Adoptium\JDK",
            "Path",
            Some(r#"hotspot\MSI"#),
        ),
        // Microsoft
        (r"SOFTWARE\Microsoft\JDK", "Path", Some(r#"hotspot\MSI"#)),
        // Azul Zulu
        (r"SOFTWARE\Azul Systems\Zulu", "InstallationPath", None),
        // BellSoft Liberica
        (r"SOFTWARE\BellSoft\Liberica", "InstallationPath", None),
    ];

    for (key, value, subkey_suffix) in reg_paths {
        match read_registry_key(key, value, subkey_suffix) {
            Ok(paths) => {
                javas.extend(paths.into_iter().map(|path| PathBuf::from(path)));
            }
            Err(_) => continue,
        }
    }

    // Load from disk options
    let potential_parent_dirs = vec!["C:/Program Files/Java", "C:/Program Files (x86)/Java"];
    for potential_parent_dir in potential_parent_dirs {
        let parent_dir = PathBuf::from(potential_parent_dir);
        if parent_dir.exists() {
            let children = std::fs::read_dir(parent_dir);
            if let Ok(mut children) = children {
                while let Some(child) = children.next() {
                    if let Ok(child) = child {
                        let child = child.path();
                        if child.is_dir() {
                            javas.extend(search_java_binary_in_path(child));
                        }
                    }
                }
            }
        }
    }

    // Remove duplicates
    javas.sort_by(|a, b| {
        a.to_string_lossy()
            .to_string()
            .to_lowercase()
            .cmp(&b.to_string_lossy().to_string().to_lowercase())
    });
    javas.dedup();

    javas.into_iter().filter(|java| java.exists()).collect()
}

#[cfg(target_os = "linux")]
pub async fn find_java_paths() -> Vec<PathBuf> {
    let folders = [
        "/usr/java",
        "/usr/lib/jvm",
        "/usr/lib64/jvm",
        "/usr/lib32/jvm",
        "/opt/jdk",
        "/opt/jdks",
        "/app/jdk",
    ];

    let mut javas: Vec<PathBuf> = vec![];

    for file in folders {
        let directories = scan_java_dirs(file).await;
        for dir in directories {
            javas.push(dir);
        }
    }

    let java_from_env = load_java_paths_from_env().await;
    if let Ok(java_from_env) = java_from_env {
        javas.extend(java_from_env);
    }

    // Remove duplicates
    javas.sort_by(|a, b| {
        a.to_string_lossy()
            .to_string()
            .to_lowercase()
            .cmp(&b.to_string_lossy().to_string().to_lowercase())
    });
    javas.dedup();

    javas.into_iter().filter(|java| java.exists()).collect()
}

#[cfg(target_os = "linux")]
async fn scan_java_dirs(dir_path: &str) -> Vec<PathBuf> {
    let mut result: Vec<PathBuf> = Vec::new();
    let mut entries = match tokio::fs::read_dir(dir_path).await {
        Ok(directories) => directories,
        Err(_) => return result,
    };

    while let Ok(child) = entries.next_entry().await {
        match child {
            Some(child) => {
                let path = PathBuf::from(child.path());
                result.push(path.join("jre/bin/java"));
                result.push(path.join("bin/java"));
            }
            None => break,
        }
    }

    result
}

pub async fn gather_java_bin_info(java_bin_path: &PathBuf) -> Result<JavaComponent> {
    let java_checker_path = locate_java_check_class().await?;
    if java_bin_path.to_string_lossy().to_string() != "java" && !java_bin_path.exists() {
        bail!("Java binary does not exist");
    }

    // Run java
    let output = Command::new(java_bin_path)
        .current_dir(&java_checker_path.parent().expect("This should never fail"))
        .arg(
            JAVA_CHECK_APP_NAME
                .strip_suffix(".class")
                .expect("This should never fail"),
        )
        .output()
        .await?;

    // print
    let output = String::from_utf8(output.stdout)?;
    let java_version = parse_java_version(&output)?;
    let java_arch = parse_java_arch(&output)?;

    Ok(JavaComponent {
        path: java_bin_path.to_string_lossy().to_string(),
        version: java_version,
        arch: java_arch,
        is_custom: false,
    })
}
