use std::path::PathBuf;

use anyhow::{Context, Result};

use super::{utils::get_path_separator, JavaComponent};

async fn load_java_paths_from_env() -> Result<Option<Vec<PathBuf>>> {
    let env_path = std::env::var("PATH").context("Could not find PATH env")?;
    let paths = env_path.split(get_path_separator()).collect::<Vec<&str>>();
    let mut java_paths = Vec::new();
    for path in paths {
        let path = path.to_string();
        if path.contains("java") {
            java_paths.push(PathBuf::from(path));
        }
    }

    Ok(Some(java_paths))
}

fn get_default_java_path() -> String {
    "java".to_owned()
}

#[cfg(target_os = "macos")]
pub async fn find_java_paths() -> Vec<PathBuf> {
    let mut javas: Vec<PathBuf> = vec![];
    javas.push(PathBuf::from(get_default_java_path()));
    javas.push(PathBuf::from("/Applications/Xcode.app/Contents/Applications/Application Loader.app/Contents/MacOS/itms/java/bin/java"));
    javas.push(PathBuf::from(
        "/Library/Internet Plug-Ins/JavaAppletPlugin.plugin/Contents/Home/bin/java",
    ));
    javas.push(PathBuf::from(
        "/System/Library/Frameworks/JavaVM.framework/Versions/Current/Commands/java",
    ));

    // Library JVM
    let library_jvm_dir = PathBuf::from("/Library/Java/JavaVirtualMachines");
    let library_jvm_javas = std::fs::read_dir(library_jvm_dir);
    if let Ok(library_jvm_javas) = library_jvm_javas {
        for library_jvm_java in library_jvm_javas {
            if let Ok(library_jvm_java) = library_jvm_java {
                let library_jvm_java = library_jvm_java.path();
                javas.push(library_jvm_java.join("Contents/Home/bin/java"));
                javas.push(library_jvm_java.join("/Contents/Home/jre/bin/java"));
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
                javas.push(system_library_jvm_java.join("/Contents/Home/bin/java"));
                javas.push(system_library_jvm_java.join("/Contents/Commands/java"));
            }
        }
    }

    let java_from_env = load_java_paths_from_env().await;
    if let Ok(Some(java_from_env)) = java_from_env {
        javas.extend(java_from_env);
    }

    // Remove duplicates
    javas = javas
        .into_iter()
        .filter(|java_path| java_path.to_str().is_some())
        .collect();
    javas.sort_by(|a, b| {
        a.to_str()
            .unwrap()
            .to_lowercase()
            .cmp(&b.to_str().unwrap().to_lowercase())
    });
    javas.dedup();

    javas
}

#[cfg(target_os = "windows")]
pub fn read_registry_key(key: &str, value: &str) -> Result<String> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE);
    let key = hkcu.open_subkey(key)?;
    let res: String = key.get_value(value)?;
    Ok(res)
}

#[cfg(target_os = "windows")]
pub async fn find_java_paths() -> Vec<PathBuf> {
    use winreg::enums::*;
    use winreg::RegKey;
    let mut javas: Vec<PathBuf> = vec![];
    javas.push(PathBuf::from(get_default_java_path()));
    let reg_paths = vec![
        (r"SOFTWARE\JavaSoft\Java Runtime Environment", "JavaHome"),
        (r"SOFTWARE\JavaSoft\Java Development Kit", "JavaHome"),
        (r"SOFTWARE\JavaSoft\JRE", "JavaHome"),
        (r"SOFTWARE\JavaSoft\JDK", "JavaHome"),
    ];

    let java_from_env = load_java_paths_from_env().await;
    if let Ok(Some(java_from_env)) = java_from_env {
        javas.extend(java_from_env);
    }

    // Remove duplicates
    javas = javas
        .into_iter()
        .filter(|java_path| java_path.to_str().is_some())
        .collect();
    javas.sort_by(|a, b| {
        a.to_str()
            .unwrap()
            .to_lowercase()
            .cmp(&b.to_str().unwrap().to_lowercase())
    });
    javas.dedup();

    javas
}
