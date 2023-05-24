use anyhow::bail;

use crate::domain::java::{JavaArch, JavaVersion};

#[derive(Debug, PartialEq, Eq)]
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
            arch = Some(JavaArch::try_from(line.replace("os.arch=", "").trim())?);
        } else if line.trim().starts_with("java.vendor=") {
            vendor = Some(line.replace("java.vendor=", "").trim().to_string());
        }
    }

    let Some(version) = version else {
        bail!("Could not parse java version from output");
    };

    let Some(arch) = arch else {
        bail!("Could not parse java arch from output");
    };

    let Some(vendor) = vendor else {
        bail!("Could not parse java vendor from output");
    };

    Ok(JavaCmdParsedOutput {
        version,
        arch,
        vendor,
    })
}

#[cfg(test)]
mod test {
    use crate::{domain::java::JavaVersion, managers::java::parser::JavaCmdParsedOutput};

    #[test]
    fn test_parse_cmd_output_java_arch() {
        struct TestCase {
            output: &'static str,
            expected: Option<JavaCmdParsedOutput>,
        }

        let expected = [
            TestCase {
                output: "os.arch=amd64\njava.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: Some(JavaCmdParsedOutput {
                    version: JavaVersion::try_from("19.0.1").unwrap(),
                    arch: "amd64".try_into().unwrap(),
                    vendor: "AdoptOpenJDK".into(),
                }),
            },
            TestCase {
                output: "os.arch=x64\njava.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: Some(JavaCmdParsedOutput {
                    version: JavaVersion::try_from("19.0.1").unwrap(),
                    arch: super::JavaArch::X86_64,
                    vendor: "AdoptOpenJDK".to_string(),
                }),
            },
            TestCase {
                output: "os.arch=x86_64\njava.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: Some(JavaCmdParsedOutput {
                    version: JavaVersion::try_from("19.0.1").unwrap(),
                    arch: super::JavaArch::X86_64,
                    vendor: "AdoptOpenJDK".to_string(),
                }),
            },
            TestCase {
                output: "os.arch=x86\njava.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: Some(JavaCmdParsedOutput {
                    version: JavaVersion::try_from("19.0.1").unwrap(),
                    arch: super::JavaArch::X86_32,
                    vendor: "AdoptOpenJDK".to_string(),
                }),
            },
            TestCase {
                output: "os.arch=aarch64\njava.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: Some(JavaCmdParsedOutput {
                    version: JavaVersion::try_from("19.0.1").unwrap(),
                    arch: super::JavaArch::Arm64,
                    vendor: "AdoptOpenJDK".to_string(),
                }),
            },
            TestCase {
                output: "java.version=19.0.1\njava.vendor=AdoptOpenJDK",
                expected: None,
            },
            TestCase {
                output: "java.version=19.0.1\nos.arch=aarch64",
                expected: None,
            },
        ];

        for test_case in expected.iter() {
            let actual = super::parse_cmd_output_java(test_case.output).ok();
            assert_eq!(actual, test_case.expected);
        }
    }
}
