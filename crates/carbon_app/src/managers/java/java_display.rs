//! How a Java is named in the UI, and which binary a file pick means.

use crate::domain::java::JavaOs;
use std::path::{Path, PathBuf};

/// Short names by what their builds' `java.vendor` contains, lowercase. The
/// first match wins.
const VENDOR_NAMES: &[(&str, &[&str])] = &[
    ("Azul Zulu", &["azul", "zulu"]),
    // Before Oracle: Oracle's own GraalVM reports "Oracle Corporation" and
    // shows as Oracle, the community one as GraalVM.
    ("GraalVM", &["graalvm"]),
    (
        "Temurin",
        &["adoptium", "adoptopenjdk", "eclipse", "temurin"],
    ),
    ("Oracle", &["oracle"]),
    ("Microsoft", &["microsoft"]),
    ("Corretto", &["amazon", "corretto"]),
    ("Liberica", &["bellsoft", "liberica"]),
    ("JetBrains", &["jetbrains"]),
    ("Red Hat", &["red hat", "redhat"]),
    ("SapMachine", &["sap se", "sapmachine"]),
];

/// The folder managed Javas install into, under the launcher's data folder.
const MANAGED_JAVAS_FOLDER: &str = "managed_javas";

/// A short name for the vendor the Java check reports (`java.vendor`). A Java
/// in a `mojang-…` install folder is Mojang's, whoever built it.
pub fn vendor_display_name(raw_vendor: &str, path: &str) -> String {
    if in_mojang_install(path) {
        return "Mojang".to_string();
    }

    let vendor = raw_vendor.to_lowercase();
    VENDOR_NAMES
        .iter()
        .find(|(_, needles)| needles.iter().any(|needle| vendor.contains(needle)))
        .map_or("OpenJDK", |(name, _)| name)
        .to_string()
}

fn in_mojang_install(path: &str) -> bool {
    let components: Vec<_> = Path::new(path)
        .components()
        .map(|component| component.as_os_str())
        .collect();

    components.windows(2).any(|pair| {
        pair[0] == MANAGED_JAVAS_FOLDER
            && pair[1]
                .to_str()
                .is_some_and(|folder| folder.starts_with("mojang-"))
    })
}

/// The Java binary for what the user picked in a file dialog:
/// - a `java` binary as is, and on Windows `javaw.exe`'s `java.exe`;
/// - a `bin` folder's `java`;
/// - a Java home folder's `bin/java`;
/// - on macOS, a `.jdk`, `.jre` or `jre.bundle` bundle's
///   `Contents/Home/bin/java`, or its `Contents` folder's `Home/bin/java`.
pub fn java_binary_from_selection(selected: &Path, os: JavaOs) -> PathBuf {
    let binary = match os {
        JavaOs::Windows => "java.exe",
        JavaOs::Linux | JavaOs::MacOs => "java",
    };
    let name = selected
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    // Windows file names ignore case.
    let is = |expected: &str| match os {
        JavaOs::Windows => name.eq_ignore_ascii_case(expected),
        JavaOs::Linux | JavaOs::MacOs => name == expected,
    };

    if is(binary) {
        return selected.to_path_buf();
    }
    // The launcher needs the console binary next to it.
    if os == JavaOs::Windows && is("javaw.exe") {
        return selected.with_file_name(binary);
    }
    if is("bin") {
        return selected.join(binary);
    }

    let home = match os {
        JavaOs::MacOs if is("Contents") => selected.join("Home"),
        JavaOs::MacOs
            if name.ends_with(".jdk") || name.ends_with(".jre") || name == "jre.bundle" =>
        {
            selected.join("Contents").join("Home")
        }
        _ => selected.to_path_buf(),
    };

    home.join("bin").join(binary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vendor_display_name_maps_known_vendors() {
        let name = |vendor| vendor_display_name(vendor, "/usr/lib/jvm/java/bin/java");

        assert_eq!(name("Azul Systems, Inc."), "Azul Zulu");
        assert_eq!(name("Eclipse Adoptium"), "Temurin");
        assert_eq!(name("AdoptOpenJDK"), "Temurin");
        assert_eq!(name("GraalVM Community"), "GraalVM");
        assert_eq!(name("Oracle Corporation"), "Oracle");
        assert_eq!(name("Microsoft"), "Microsoft");
        assert_eq!(name("Amazon.com Inc."), "Corretto");
        assert_eq!(name("BellSoft"), "Liberica");
        assert_eq!(name("JetBrains s.r.o."), "JetBrains");
        assert_eq!(name("Red Hat, Inc."), "Red Hat");
        assert_eq!(name("SAP SE"), "SapMachine");
        assert_eq!(name("Private Build"), "OpenJDK");
        assert_eq!(name("Debian"), "OpenJDK");
        assert_eq!(name(""), "OpenJDK");

        // Mojang ships Microsoft's build; its install folder says where it came from.
        assert_eq!(
            vendor_display_name(
                "Microsoft",
                "/home/u/.gdl/managed_javas/mojang-java-runtime-delta-21.0.7/bin/java"
            ),
            "Mojang"
        );
        assert_eq!(
            vendor_display_name(
                "Oracle Corporation",
                "/x/managed_javas/mojang-java-runtime-delta-21.0.7/bin/java"
            ),
            "Mojang"
        );
        // A folder that merely looks like one elsewhere is not Mojang's.
        assert_eq!(
            vendor_display_name("Eclipse Adoptium", "/opt/mojang-tools/jdk/bin/java"),
            "Temurin"
        );
    }

    #[test]
    fn java_binary_from_selection_handles_binary_home_and_bundle() {
        let pick = |path: &str, os| java_binary_from_selection(Path::new(path), os);

        // A binary is taken as is.
        assert_eq!(
            pick("/usr/lib/jvm/java-21/bin/java", JavaOs::Linux),
            PathBuf::from("/usr/lib/jvm/java-21/bin/java")
        );
        assert_eq!(
            pick(
                "C:/Program Files/Zulu/zulu-21/bin/java.exe",
                JavaOs::Windows
            ),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/bin/java.exe")
        );
        assert_eq!(
            pick(
                "C:/Program Files/Zulu/zulu-21/bin/JAVA.EXE",
                JavaOs::Windows
            ),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/bin/JAVA.EXE")
        );

        // A Java home folder.
        assert_eq!(
            pick("/usr/lib/jvm/java-21", JavaOs::Linux),
            PathBuf::from("/usr/lib/jvm/java-21/bin/java")
        );
        assert_eq!(
            pick("/usr/lib/jvm/java-21/", JavaOs::Linux),
            PathBuf::from("/usr/lib/jvm/java-21/bin/java")
        );
        assert_eq!(
            pick("C:/Program Files/Zulu/zulu-21", JavaOs::Windows),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/bin/java.exe")
        );
        assert_eq!(
            pick(
                "/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home",
                JavaOs::MacOs
            ),
            PathBuf::from("/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java")
        );

        // macOS bundles.
        assert_eq!(
            pick(
                "/Library/Java/JavaVirtualMachines/zulu-21.jdk",
                JavaOs::MacOs
            ),
            PathBuf::from("/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java")
        );
        assert_eq!(
            pick("/x/mojang-java-runtime-delta/jre.bundle", JavaOs::MacOs),
            PathBuf::from("/x/mojang-java-runtime-delta/jre.bundle/Contents/Home/bin/java")
        );
        assert_eq!(
            pick("/x/mojang-java-runtime-delta/jre.bundle/", JavaOs::MacOs),
            PathBuf::from("/x/mojang-java-runtime-delta/jre.bundle/Contents/Home/bin/java")
        );
        // Only macOS has bundles.
        assert_eq!(
            pick("/opt/odd.jdk", JavaOs::Linux),
            PathBuf::from("/opt/odd.jdk/bin/java")
        );
    }

    #[test]
    fn java_binary_from_selection_handles_javaw_bin_and_contents() {
        let pick = |path: &str, os| java_binary_from_selection(Path::new(path), os);

        // javaw.exe has no console; the launcher runs java.exe next to it.
        assert_eq!(
            pick(
                "C:/Program Files/Zulu/zulu-21/bin/javaw.exe",
                JavaOs::Windows
            ),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/bin/java.exe")
        );
        assert_eq!(
            pick(
                "C:/Program Files/Zulu/zulu-21/bin/JAVAW.EXE",
                JavaOs::Windows
            ),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/bin/java.exe")
        );

        // The bin folder itself.
        assert_eq!(
            pick("/usr/lib/jvm/java-21/bin", JavaOs::Linux),
            PathBuf::from("/usr/lib/jvm/java-21/bin/java")
        );
        assert_eq!(
            pick("/usr/lib/jvm/java-21/bin/", JavaOs::Linux),
            PathBuf::from("/usr/lib/jvm/java-21/bin/java")
        );
        assert_eq!(
            pick("C:/Program Files/Zulu/zulu-21/Bin", JavaOs::Windows),
            PathBuf::from("C:/Program Files/Zulu/zulu-21/Bin/java.exe")
        );
        assert_eq!(
            pick(
                "/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin",
                JavaOs::MacOs
            ),
            PathBuf::from("/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java")
        );

        // A macOS bundle's Contents folder, and JRE bundles.
        assert_eq!(
            pick(
                "/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents",
                JavaOs::MacOs
            ),
            PathBuf::from("/Library/Java/JavaVirtualMachines/zulu-21.jdk/Contents/Home/bin/java")
        );
        assert_eq!(
            pick(
                "/x/mojang-java-runtime-delta/jre.bundle/Contents/",
                JavaOs::MacOs
            ),
            PathBuf::from("/x/mojang-java-runtime-delta/jre.bundle/Contents/Home/bin/java")
        );
        assert_eq!(
            pick(
                "/Library/Java/JavaVirtualMachines/zulu-21.jre",
                JavaOs::MacOs
            ),
            PathBuf::from("/Library/Java/JavaVirtualMachines/zulu-21.jre/Contents/Home/bin/java")
        );
    }
}
