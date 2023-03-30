use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

#[cfg(target_os = "windows")]
pub static PATH_SEPARATOR: &str = ";";

#[cfg(not(target_os = "windows"))]
pub static PATH_SEPARATOR: &str = ":";

const JAVA_CHECK_APP: &[u8; 1013] = include_bytes!("JavaCheck.class");
pub const JAVA_CHECK_APP_NAME: &str = "JavaCheck.class";

pub async fn locate_java_check_class() -> anyhow::Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let java_check_path = temp_dir.join(JAVA_CHECK_APP_NAME);
    if !java_check_path.exists() {
        let mut file = tokio::fs::File::create(&java_check_path).await?;

        file.write_all(JAVA_CHECK_APP).await?;
    }

    Ok(java_check_path)
}

/*
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
*/
