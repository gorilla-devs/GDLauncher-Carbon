use anyhow::bail;

use crate::domain::java::{JavaArch, JavaVersion};

pub fn parse_cmd_output_java_version(cmd_output: &str) -> anyhow::Result<JavaVersion> {
    for line in cmd_output.lines() {
        if line.trim().starts_with("java.version=") {
            return JavaVersion::try_from(line.replace("java.version=", "").trim());
        }
    }

    bail!("Could not find java version in output: {}", cmd_output);
}

pub fn parse_cmd_output_java_arch(cmd_output: &str) -> anyhow::Result<JavaArch> {
    for line in cmd_output.lines() {
        if line.trim().starts_with("os.arch=") {
            return Ok(JavaArch::from(line.replace("os.arch=", "").trim()));
        }
    }

    bail!("Could not find java arch in output: {}", cmd_output);
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
            let actual = super::parse_cmd_output_java_version(test_case.output).ok();
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
                expected: Some(super::JavaArch::Amd64),
            },
            TestCase {
                output: "os.arch=x86",
                expected: Some(super::JavaArch::X86_64),
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
            let actual = super::parse_cmd_output_java_arch(test_case.output).ok();
            assert_eq!(actual, test_case.expected);
        }
    }
}
