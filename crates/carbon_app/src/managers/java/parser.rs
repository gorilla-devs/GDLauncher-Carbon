use anyhow::bail;

use crate::domain::java::{JavaArch, JavaVersion};

pub struct JavaCmdParsedOutput {
    pub version: JavaVersion,
    pub arch: JavaArch,
    pub vendor: String,
}

pub fn parse_cmd_output_java(cmd_output: &str) -> anyhow::Result<JavaCmdParsedOutput> {
    let mut version = None;
    let mut arch = None;
    let mut vendor = None;

    for line in cmd_output.lines() {
        if line.trim().starts_with("java.version=") {
            version = Some(JavaVersion::try_from(
                line.replace("java.version=", "").trim(),
            )?);
        } else if line.trim().starts_with("os.arch=") {
            arch = Some(JavaArch::from(line.replace("os.arch=", "").trim()));
        } else if line.trim().starts_with("java.vendor=") {
            vendor = Some(line.replace("java.vendor=", "").trim().to_string());
        }
    }

    if version.is_none() {
        bail!("Could not parse java version from output");
    }

    if arch.is_none() {
        bail!("Could not parse java arch from output");
    }

    if vendor.is_none() {
        bail!("Could not parse java vendor from output");
    }

    Ok(JavaCmdParsedOutput {
        version: version.unwrap(),
        arch: arch.unwrap(),
        vendor: vendor.unwrap(),
    })
}

#[cfg(test)]
mod test {
    use crate::domain::java::JavaVersion;

    #[test]
    fn test_parse_cmd_output_java_version() {
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
                output: "java.version=",
                expected: None,
            },
        ];

        for test_case in expected.iter() {
            let actual = super::parse_cmd_output_java(test_case.output)
                .map(|x| x.version)
                .ok();
            assert_eq!(actual, test_case.expected);
        }
    }

    #[test]
    fn test_parse_cmd_output_java_arch() {
        struct TestCase {
            output: &'static str,
            expected: Option<super::JavaArch>,
        }

        let expected = [
            TestCase {
                output: "os.arch=amd64",
                expected: Some(super::JavaArch::X64),
            },
            TestCase {
                output: "os.arch=x64",
                expected: Some(super::JavaArch::X64),
            },
            TestCase {
                output: "os.arch=x86_64",
                expected: Some(super::JavaArch::X64),
            },
            TestCase {
                output: "os.arch=x86",
                expected: Some(super::JavaArch::X86),
            },
            TestCase {
                output: "os.arch=aarch64",
                expected: Some(super::JavaArch::Aarch64),
            },
            TestCase {
                output: "java.version=19.0.1",
                expected: None,
            },
        ];

        for test_case in expected.iter() {
            let actual = super::parse_cmd_output_java(test_case.output)
                .map(|x| x.arch)
                .ok();
            assert_eq!(actual, test_case.expected);
        }
    }
}
