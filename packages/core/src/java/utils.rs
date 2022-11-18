use anyhow::{bail, Result};
use regex::Regex;

use super::JavaVersion;

#[cfg(target_os = "windows")]
pub fn get_path_separator() -> char {
    ';'
}

#[cfg(not(target_os = "windows"))]
pub fn get_path_separator() -> char {
    ':'
}

pub fn parse_java_version(cmd_output: &str) -> Result<JavaVersion> {
    for line in cmd_output.lines() {
        // I spent way too much time on this regex
        let regex = Regex::new(
            r#"build (?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*(?:\.[0-9]+)?)(?:_(?P<update_number>[0-9]+)?)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<build_metadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#,
        )?;

        if let Some(captures) = regex.captures(line) {
            let mut version = JavaVersion {
                major: 0,
                minor: None,
                patch: None,
                update_number: None,
                prerelease: None,
                build_metadata: None,
            };

            for name in regex.capture_names().filter_map(|n| n) {
                match name {
                    "major" => {
                        if let Some(major) = captures.name("major") {
                            version.major = major.as_str().parse()?;
                        }
                    }
                    "minor" => {
                        if let Some(minor) = captures.name("minor") {
                            version.minor = Some(minor.as_str().parse()?);
                        }
                    }
                    "patch" => {
                        if let Some(patch) = captures.name("patch") {
                            version.patch = Some(patch.as_str().parse()?);
                        }
                    }
                    "update_number" => {
                        if let Some(update_number) = captures.name("update_number") {
                            version.update_number = Some(update_number.as_str().parse()?);
                        }
                    }
                    "prerelease" => {
                        if let Some(prerelease) = captures.name("prerelease") {
                            version.prerelease = Some(prerelease.as_str().to_string());
                        }
                    }
                    "build_metadata" => {
                        if let Some(build_metadata) = captures.name("build_metadata") {
                            version.build_metadata = Some(build_metadata.as_str().to_string());
                        }
                    }
                    _ => {
                        bail!("Unknown capture group: {}", name);
                    }
                }
            }
            return Ok(version);
        }
    }
    bail!("Could not parse java version")
}

mod tests {
    use crate::java::JavaVersion;

    #[test]
    fn test_parse_java_version() {
        #[derive(Debug)]
        struct TestCase {
            output: &'static str,
            expected: Option<JavaVersion>,
        }

        let expected = [
            TestCase {
                output: r#"openjdk 19.0.1 2022-10-18
                        OpenJDK Runtime Environment Homebrew (build 19.0.1)
                        OpenJDK 64-Bit Server VM Homebrew (build 19.0.1, mixed mode, sharing)"#,
                expected: Some(JavaVersion {
                    major: 19,
                    minor: Some(0),
                    patch: Some("1".to_owned()),
                    update_number: None,
                    prerelease: None,
                    build_metadata: None,
                }),
            },
            TestCase {
                output: r#"Error: Could not create the Java Virtual Machine.
                        Error: A fatal exception has occurred. Program will exit.
                        "#,
                expected: None,
            },
            TestCase {
                output: r#"openjdk version "1.8.0_352"
                        OpenJDK Runtime Environment (Temurin)(build 1.8.0_352-b08)
                        OpenJDK 64-Bit Server VM (Temurin)(build 25.352-b08, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 1,
                    minor: Some(8),
                    patch: Some("0".to_owned()),
                    update_number: Some("352".to_owned()),
                    prerelease: Some("b08".to_owned()),
                    build_metadata: None,
                }),
            },
            TestCase {
                output: r#"openjdk version "19.0.1" 2022-10-18
                        OpenJDK Runtime Environment Temurin-19.0.1+10 (build 19.0.1+10)
                        OpenJDK 64-Bit Server VM Temurin-19.0.1+10 (build 19.0.1+10, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 19,
                    minor: Some(0),
                    patch: Some("1".to_owned()),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("10".to_owned()),
                }),
            },
            TestCase {
                output: r#"java version "1.4.0_03"
                        Java(TM) 2 Runtime Environment, Standard Edition (build 1.4.0_03-b04)
                        Java HotSpot(TM) Client VM (build 1.4.0_03-b04, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 1,
                    minor: Some(4),
                    patch: Some("0".to_owned()),
                    update_number: Some("03".to_owned()),
                    prerelease: Some("b04".to_owned()),
                    build_metadata: None,
                }),
            },
            TestCase {
                output: r#"openjdk version "17.0.6-beta" 2023-01-17
                        OpenJDK Runtime Environment Temurin-17.0.6+2-202211152348 (build 17.0.6-beta+2-202211152348)
                        OpenJDK 64-Bit Server VM Temurin-17.0.6+2-202211152348 (build 17.0.6-beta+2-202211152348, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 17,
                    minor: Some(0),
                    patch: Some("6".to_owned()),
                    update_number: None,
                    prerelease: Some("beta".to_owned()),
                    build_metadata: Some("2-202211152348".to_owned()),
                }),
            },
            TestCase {
                output: r#"openjdk version "1.8.0_362-beta"
                        OpenJDK Runtime Environment (Temurin)(build 1.8.0_362-beta-202211161809-b03+152)
                        OpenJDK 64-Bit Server VM (Temurin)(build 25.362-b03, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 1,
                    minor: Some(8),
                    patch: Some("0".to_owned()),
                    update_number: Some("362".to_owned()),
                    prerelease: Some("beta-202211161809-b03".to_owned()),
                    build_metadata: Some("152".to_owned()),
                }),
            },
            TestCase {
                output: r#"openjdk version "18.0.2.1" 2022-08-18
                        OpenJDK Runtime Environment Temurin-18.0.2.1+1 (build 18.0.2.1+1)
                        OpenJDK 64-Bit Server VM Temurin-18.0.2.1+1 (build 18.0.2.1+1, mixed mode)
                        "#,
                expected: Some(JavaVersion {
                    major: 18,
                    minor: Some(0),
                    patch: Some("2.1".to_owned()),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("1".to_owned()),
                }),
            },
            TestCase {
                output: r#"openjdk version "17.0.5" 2022-10-18
                        IBM Semeru Runtime Open Edition 17.0.5.0 (build 17.0.5+8)
                        Eclipse OpenJ9 VM 17.0.5.0 (build openj9-0.35.0, JRE 17 Mac OS X amd64-64-Bit Compressed References 20221018_304 (JIT enabled, AOT enabled)
                        OpenJ9   - e04a7f6c1
                        OMR      - 85a21674f
                        JCL      - 32d2c409a33 based on jdk-17.0.5+8)
                        "#,
                expected: Some(JavaVersion {
                    major: 17,
                    minor: Some(0),
                    patch: Some("5".to_owned()),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("8".to_owned()),
                }),
            },
            TestCase {
                output: r#"openjdk version "17.0.5" 2022-10-18 LTS
                        OpenJDK Runtime Environment Microsoft-6841604 (build 17.0.5+8-LTS)
                        OpenJDK 64-Bit Server VM Microsoft-6841604 (build 17.0.5+8-LTS, mixed mode, sharing)
                        "#,
                expected: Some(JavaVersion {
                    major: 17,
                    minor: Some(0),
                    patch: Some("5".to_owned()),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("8-LTS".to_owned()),
                }),
            },
        ];

        for test_case in expected.iter() {
            let actual = super::parse_java_version(test_case.output).ok();
            assert_eq!(actual, test_case.expected);
        }
    }
}
