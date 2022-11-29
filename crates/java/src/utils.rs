use anyhow::{bail, ensure, Result};
use regex::Regex;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use crate::JavaArch;

use super::JavaVersion;

#[cfg(target_os = "windows")]
pub static PATH_SEPARATOR: &'static str = ";";

#[cfg(not(target_os = "windows"))]
pub static PATH_SEPARATOR: &'static str = ":";

const JAVA_CHECK_APP: &'static [u8; 1013] = include_bytes!("JavaCheck.class");
pub const JAVA_CHECK_APP_NAME: &'static str = "JavaCheck.class";

pub async fn locate_java_check_class() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let java_check_path = temp_dir.join(JAVA_CHECK_APP_NAME);
    if !java_check_path.exists() {
        let mut file = tokio::fs::File::create(&java_check_path).await?;
        file.write_all(JAVA_CHECK_APP).await?;
    }

    Ok(java_check_path)
}

pub fn parse_java_version(cmd_output: &str) -> Result<JavaVersion> {
    for line in cmd_output.lines() {
        // I spent way too much time on this regex
        let regex = Regex::new(
            r#"^java.version=(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*(?:\.[0-9]+)?)(?:_(?P<update_number>[0-9]+)?)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<build_metadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#,
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

pub fn parse_java_arch(cmd_output: &str) -> Result<JavaArch> {
    for line in cmd_output.lines() {
        // I spent way too much time on this regex
        let regex = Regex::new(r#"^java.arch=(?P<arch>[[:alnum:]]*)"#)?;

        if let Some(captures) = regex.captures(line) {
            match captures.name("arch") {
                Some(arch) => {
                    return Ok(arch.as_str().into());
                }
                None => {
                    bail!("No arch found in output");
                }
            }
        }
    }
    bail!("Could not parse java arch")
}

#[cfg(test)]
mod test {
    use crate::JavaVersion;

    #[test]
    fn test_parse_java_version() {
        struct TestCase {
            output: &'static str,
            expected: Option<JavaVersion>,
        }

        let expected = [
            TestCase {
                output: "java.version=19.0.1",
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
                output: "os.arch=amd64",
                expected: None,
            },
            TestCase {
                output: "java.version=1.8.0_352-b08",
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
                output: "java.version=19.0.1+10",
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
                output: "java.version=1.4.0_03-b04",
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
                output: "java.version=17.0.6-beta+2-202211152348",
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
                output: "java.version=1.8.0_362-beta-202211161809-b03+152",
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
                output: "java.version=18.0.2.1+1",
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
                output: "java.version=17.0.5+8",
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
                output: "java.version=17.0.5+8-LTS",
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

    #[test]
    fn test_parse_java_arch() {
        struct TestCase {
            output: &'static str,
            expected: Option<super::JavaArch>,
        }

        let expected = [
            TestCase {
                output: "java.arch=amd64",
                expected: Some(super::JavaArch::Amd64),
            },
            TestCase {
                output: "java.arch=x86",
                expected: Some(super::JavaArch::X86),
            },
            TestCase {
                output: "java.arch=aarch64",
                expected: Some(super::JavaArch::Aarch64),
            },
            TestCase {
                output: "java.version=19.0.1",
                expected: None,
            },
        ];

        for test_case in expected.iter() {
            let actual = super::parse_java_arch(test_case.output).ok();
            assert_eq!(actual, test_case.expected);
        }
    }

    #[tokio::test]
    async fn test_locate_java_check_class_and_execute() {
        let temp_dir = std::env::temp_dir();
        let java_check_path_env = temp_dir.join(super::JAVA_CHECK_APP_NAME);
        let _ = std::fs::remove_file(&java_check_path_env);

        let java_check_path = super::locate_java_check_class().await.unwrap();
        assert!(
            java_check_path == java_check_path_env,
            "Java check path is unexpected"
        );
        assert!(java_check_path.exists(), "Java check path does not exist");

        let proc = tokio::process::Command::new("java")
            .current_dir(temp_dir)
            .arg(super::JAVA_CHECK_APP_NAME.strip_suffix(".class").unwrap())
            .output()
            .await
            .unwrap();

        assert!(
            proc.status.code() == Some(0),
            "Java check exit code is not 0"
        );
        let _ = std::fs::remove_file(&java_check_path_env);
    }
}
