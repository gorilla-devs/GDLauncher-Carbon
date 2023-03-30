use anyhow::bail;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum JavaArch {
    Amd64,
    X86,
    Aarch64,
}

impl<'a> From<&JavaArch> for &'a str {
    fn from(arch: &JavaArch) -> &'a str {
        match arch {
            JavaArch::Amd64 => "amd64",
            JavaArch::X86 => "x86",
            JavaArch::Aarch64 => "aarch64",
        }
    }
}

impl<'a> From<&'a str> for JavaArch {
    fn from(s: &'a str) -> Self {
        match s {
            "amd64" => JavaArch::Amd64,
            "x86" => JavaArch::X86,
            "aarch64" => JavaArch::Aarch64,
            _ => panic!("Unknown JavaArch: {s}"),
        }
    }
}

pub fn parse_java_arch(arch_string: &str) -> anyhow::Result<JavaArch> {
    // I spent way too much time on this regex
    let regex = Regex::new(r#"^(?P<arch>[[:alnum:]]*)"#)?;

    if let Some(captures) = regex.captures(arch_string) {
        match captures.name("arch") {
            Some(arch) => {
                return Ok(arch.as_str().into());
            }
            None => bail!("Not a valid arch. Cannot parse: {}", arch_string),
        }
    }
    bail!("Could not parse java arch from output: {}", arch_string);
}

pub fn parse_cmd_output_java_arch(cmd_output: &str) -> anyhow::Result<JavaArch> {
    for line in cmd_output.lines() {
        if line.starts_with("java.arch=") {
            return parse_java_arch(line.replace("java.arch=", "").trim());
        }
    }

    bail!("Could not find java version in output: {}", cmd_output);
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JavaVersion {
    pub major: u8,
    pub minor: Option<u8>,
    pub patch: Option<String>,
    pub update_number: Option<String>,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

impl TryFrom<&str> for JavaVersion {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        parse_java_version(s)
    }
}

impl From<JavaVersion> for String {
    fn from(v: JavaVersion) -> Self {
        format!(
            "{}.{}.{}{}{}{}",
            v.major,
            v.minor.unwrap_or(0),
            v.patch.unwrap_or_default(),
            v.update_number
                .map(|u| format!("_{}", u))
                .unwrap_or_default(),
            v.prerelease.map(|p| format!("-{}", p)).unwrap_or_default(),
            v.build_metadata
                .map(|b| format!("+{}", b))
                .unwrap_or_default()
        )
    }
}

impl JavaVersion {
    pub fn from_major(major: u8) -> Self {
        Self {
            major,
            minor: None,
            patch: None,
            update_number: None,
            prerelease: None,
            build_metadata: None,
        }
    }
}

pub fn parse_cmd_output_java_version(cmd_output: &str) -> anyhow::Result<JavaVersion> {
    for line in cmd_output.lines() {
        if line.starts_with("java.version=") {
            return parse_java_version(line.replace("java.version=", "").trim());
        }
    }

    bail!("Could not find java version in output: {}", cmd_output);
}

fn parse_java_version(version_string: &str) -> anyhow::Result<JavaVersion> {
    // I spent way too much time on this regex
    let regex = Regex::new(
        r#"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*(?:\.[0-9]+)?)(?:_(?P<update_number>[0-9]+)?)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<build_metadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#,
    )?;

    if let Some(captures) = regex.captures(version_string) {
        let mut version = JavaVersion {
            major: 0,
            minor: None,
            patch: None,
            update_number: None,
            prerelease: None,
            build_metadata: None,
        };

        for name in regex.capture_names().flatten() {
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
                    unreachable!("Regex capture group not handled: {}", name)
                }
            }
        }
        return Ok(version);
    }

    bail!("Could not parse java version in string: {}", version_string);
}
