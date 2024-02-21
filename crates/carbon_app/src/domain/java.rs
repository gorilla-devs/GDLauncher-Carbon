use anyhow::bail;
use daedalus::minecraft::MinecraftJavaProfile;
use regex::Regex;
use serde::{Deserialize, Serialize};
use strum_macros::EnumIter;

// TODO: This does not handle the case where the same path still exists between executions but changes the version
pub struct Java {
    pub id: String,
    pub component: JavaComponent,
    pub is_valid: bool,
}

impl TryFrom<crate::db::java::Data> for Java {
    type Error = anyhow::Error;

    fn try_from(value: crate::db::java::Data) -> Result<Self, Self::Error> {
        let is_valid = value.is_valid;
        Ok(Self {
            id: value.id.clone(),
            component: JavaComponent::try_from(value)?,
            is_valid,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct JavaComponent {
    pub path: String,
    pub arch: JavaArch,
    pub os: JavaOs,
    /// Indicates whether the component has manually been added by the user
    #[serde(rename = "type")]
    pub _type: JavaComponentType,
    pub version: JavaVersion,
    pub vendor: String,
}

impl TryFrom<crate::db::java::Data> for JavaComponent {
    type Error = anyhow::Error;

    fn try_from(value: crate::db::java::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            path: value.path,
            arch: JavaArch::try_from(&*value.arch)?,
            _type: JavaComponentType::try_from(&*value.r#type)?,
            version: JavaVersion::try_from(&*value.full_version)?,
            os: JavaOs::try_from(value.os)?,
            vendor: value.vendor,
        })
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub enum JavaComponentType {
    Local,
    Managed,
    Custom,
}

impl TryFrom<&str> for JavaComponentType {
    type Error = anyhow::Error;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match &*s.to_lowercase() {
            "local" => Ok(Self::Local),
            "managed" => Ok(Self::Managed),
            "custom" => Ok(Self::Custom),
            _ => bail!("Uh oh, this shouldn't happen"),
        }
    }
}

impl ToString for JavaComponentType {
    fn to_string(&self) -> String {
        match self {
            Self::Local => "local",
            Self::Managed => "managed",
            Self::Custom => "custom",
        }
        .to_string()
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Hash, Copy, Clone, EnumIter)]
pub enum JavaArch {
    X86_64,
    X86_32,
    Arm32,
    Arm64,
}

impl JavaArch {
    pub fn get_current_arch() -> anyhow::Result<Self> {
        Self::try_from(std::env::consts::ARCH)
    }
}

impl ToString for JavaArch {
    fn to_string(&self) -> String {
        match self {
            JavaArch::X86_64 => "x64",
            JavaArch::X86_32 => "x86",
            JavaArch::Arm32 => "arm32",
            JavaArch::Arm64 => "arm64",
        }
        .to_string()
    }
}

impl<'a> TryFrom<&'a str> for JavaArch {
    type Error = anyhow::Error;

    fn try_from(s: &'a str) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "amd64" => Ok(JavaArch::X86_64),
            "x64" => Ok(JavaArch::X86_64),
            "x86" => Ok(JavaArch::X86_32),
            "x86_64" => Ok(JavaArch::X86_64),
            "x86_32" => Ok(JavaArch::X86_32),
            "arm32" => Ok(JavaArch::Arm32),
            "arm64" => Ok(JavaArch::Arm64),
            "aarch32" => Ok(JavaArch::Arm32),
            "aarch64" => Ok(JavaArch::Arm64),
            _ => bail!("Unknown JavaArch: {s}"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, EnumIter, Copy, Clone)]
pub enum JavaOs {
    Windows,
    Linux,
    MacOs,
}

impl TryFrom<&str> for JavaOs {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            _ => bail!("Unknown JavaOs: {}", value),
        }
    }
}

impl JavaOs {
    pub fn get_current_os() -> anyhow::Result<Self> {
        JavaOs::try_from(std::env::consts::OS)
    }
}

impl TryFrom<String> for JavaOs {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::MacOs),
            _ => Err(anyhow::anyhow!("Unknown OS: {}", value)),
        }
    }
}

impl ToString for JavaOs {
    fn to_string(&self) -> String {
        match self {
            JavaOs::Windows => "windows",
            JavaOs::Linux => "linux",
            JavaOs::MacOs => "macos",
        }
        .to_string()
    }
}

#[derive(Debug, EnumIter)]
pub enum JavaVendor {
    Azul,
}

impl JavaVendor {
    pub fn from_java_dot_vendor(vendor: &str) -> Option<Self> {
        match vendor {
            "Azul Systems, Inc." => Some(Self::Azul),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct JavaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: String,
    pub update_number: Option<String>,
    pub prerelease: Option<String>,
    pub build_metadata: Option<String>,
}

impl TryFrom<&str> for JavaVersion {
    type Error = anyhow::Error;

    fn try_from(version_string: &str) -> Result<Self, Self::Error> {
        let regex = Regex::new(
            r#"^(?P<major>0|[1-9]\d*)(?:\.(?P<minor>0|[1-9]\d*))?(?:\.(?P<patch>0|[1-9]\d*(?:\.[0-9]+)?))?(?:_(?P<update_number>[0-9]+)?)?(?:-(?P<prerelease>(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*)(?:\.(?:0|[1-9]\d*|\d*[a-zA-Z-][0-9a-zA-Z-]*))*))?(?:\+(?P<build_metadata>[0-9a-zA-Z-]+(?:\.[0-9a-zA-Z-]+)*))?"#,
        )?;

        if let Some(captures) = regex.captures(version_string) {
            let mut version = JavaVersion {
                major: 0,
                minor: 0,
                patch: "0".to_string(),
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
                            version.minor = minor.as_str().parse()?;
                        }
                    }
                    "patch" => {
                        if let Some(patch) = captures.name("patch") {
                            version.patch = patch.as_str().parse()?;
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

            // 1.8.0_832 -> 8.0.832
            if version.major == 1 {
                version.major = version.minor;
                version.minor = version.patch.parse()?;
                version.patch = version.update_number.unwrap_or("0".to_string());
                version.update_number = None;
            }

            return Ok(version);
        }

        bail!("Could not parse java version in string: {}", version_string);
    }
}

impl From<JavaVersion> for String {
    fn from(v: JavaVersion) -> Self {
        format!(
            "{}.{}.{}{}{}{}",
            v.major,
            v.minor,
            v.patch,
            v.update_number.map(|u| format!("_{u}")).unwrap_or_default(),
            v.prerelease.map(|p| format!("-{p}")).unwrap_or_default(),
            v.build_metadata
                .map(|b| format!("+{b}"))
                .unwrap_or_default()
        )
    }
}

impl ToString for JavaVersion {
    fn to_string(&self) -> String {
        String::from(self.clone())
    }
}

impl JavaVersion {
    pub fn from_major(major: u16) -> Self {
        Self {
            major,
            minor: 0,
            patch: "0".to_string(),
            update_number: None,
            prerelease: None,
            build_metadata: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone, EnumIter, Eq, PartialEq)]
pub enum SystemJavaProfileName {
    Legacy,
    // LegacyFixed1 doesn't natively exist in metadata, it's only used to fix forge on 1.16.5 and patched at run.rs level
    LegacyFixed1,
    Alpha,
    Beta,
    Gamma,
    GammaSnapshot,
    MinecraftJavaExe,
}

impl SystemJavaProfileName {
    pub fn is_java_version_compatible(&self, java_version: &JavaVersion) -> bool {
        match self {
            Self::Legacy => java_version.major == 8,
            Self::LegacyFixed1 => {
                java_version.major == 8 && java_version.patch.parse().unwrap_or(0) < 312
            } // newer versions will break sometimes,
            Self::Alpha => java_version.major == 16,
            Self::Beta => java_version.major == 17,
            Self::Gamma => java_version.major == 17,
            Self::GammaSnapshot => java_version.major == 17,
            Self::MinecraftJavaExe => java_version.major == 14,
        }
    }
}

impl From<MinecraftJavaProfile> for SystemJavaProfileName {
    fn from(value: MinecraftJavaProfile) -> Self {
        match value {
            MinecraftJavaProfile::JreLegacy => Self::Legacy,
            MinecraftJavaProfile::JavaRuntimeAlpha => Self::Alpha,
            MinecraftJavaProfile::JavaRuntimeBeta => Self::Beta,
            MinecraftJavaProfile::JavaRuntimeGamma => Self::Gamma,
            MinecraftJavaProfile::JavaRuntimeGammaSnapshot => Self::GammaSnapshot,
            MinecraftJavaProfile::MinecraftJavaExe => Self::MinecraftJavaExe,
        }
    }
}

pub const SYSTEM_JAVA_PROFILE_NAME_PREFIX: &str = "__gdl_system_java_profile__";

impl ToString for SystemJavaProfileName {
    fn to_string(&self) -> String {
        let name = match self {
            Self::Legacy => "legacy",
            Self::LegacyFixed1 => "legacy_fixed_1",
            Self::Alpha => "alpha",
            Self::Beta => "beta",
            Self::Gamma => "gamma",
            Self::GammaSnapshot => "gamma_snapshot",
            Self::MinecraftJavaExe => "mc_java_exe",
        };

        format!("{}{}", SYSTEM_JAVA_PROFILE_NAME_PREFIX, name)
    }
}

impl TryFrom<&str> for SystemJavaProfileName {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if !value.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX) {
            bail!("Invalid system java profile name: {}", value);
        }

        let name = value.strip_prefix(SYSTEM_JAVA_PROFILE_NAME_PREFIX).unwrap();

        match name {
            "legacy" => Ok(Self::Legacy),
            "legacy_fixed_1" => Ok(Self::LegacyFixed1),
            "alpha" => Ok(Self::Alpha),
            "beta" => Ok(Self::Beta),
            "gamma" => Ok(Self::Gamma),
            "gamma_snapshot" => Ok(Self::GammaSnapshot),
            "mc_java_exe" => Ok(Self::MinecraftJavaExe),
            _ => bail!("Invalid system java profile name: {}", value),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JavaProfile {
    pub name: String,
    pub java_id: Option<String>,
    pub is_system: bool,
}

impl TryFrom<crate::db::java_profile::Data> for JavaProfile {
    type Error = anyhow::Error;

    fn try_from(data: crate::db::java_profile::Data) -> Result<Self, Self::Error> {
        Ok(Self {
            name: data.name,
            java_id: data.java_id,
            is_system: data.is_system_profile,
        })
    }
}

#[cfg(test)]
mod test {
    use crate::domain::java::JavaVersion;

    #[test]
    fn test_parse_java_version() {
        struct TestCase {
            output: &'static str,
            expected: Option<JavaVersion>,
        }

        let expected = [
            TestCase {
                output: "19.0.1",
                expected: Some(JavaVersion {
                    major: 19,
                    minor: 0,
                    patch: "1".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: None,
                }),
            },
            TestCase {
                output: "19.0",
                expected: Some(JavaVersion {
                    major: 19,
                    minor: 0,
                    patch: "0".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: None,
                }),
            },
            TestCase {
                output: "19",
                expected: Some(JavaVersion {
                    major: 19,
                    minor: 0,
                    patch: "0".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: None,
                }),
            },
            TestCase {
                output: "amd64",
                expected: None,
            },
            TestCase {
                output: "1.8.0_352-b08",
                expected: Some(JavaVersion {
                    major: 8,
                    minor: 0,
                    patch: "352".to_owned(),
                    update_number: None,
                    prerelease: Some("b08".to_owned()),
                    build_metadata: None,
                }),
            },
            TestCase {
                output: "19.0.1+10",
                expected: Some(JavaVersion {
                    major: 19,
                    minor: 0,
                    patch: "1".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("10".to_owned()),
                }),
            },
            TestCase {
                output: "1.4.0_03-b04",
                expected: Some(JavaVersion {
                    major: 4,
                    minor: 0,
                    patch: "03".to_owned(),
                    update_number: None,
                    prerelease: Some("b04".to_owned()),
                    build_metadata: None,
                }),
            },
            TestCase {
                output: "17.0.6-beta+2-202211152348",
                expected: Some(JavaVersion {
                    major: 17,
                    minor: 0,
                    patch: "6".to_owned(),
                    update_number: None,
                    prerelease: Some("beta".to_owned()),
                    build_metadata: Some("2-202211152348".to_owned()),
                }),
            },
            TestCase {
                output: "1.8.0_362-beta-202211161809-b03+152",
                expected: Some(JavaVersion {
                    major: 8,
                    minor: 0,
                    patch: "362".to_owned(),
                    update_number: None,
                    prerelease: Some("beta-202211161809-b03".to_owned()),
                    build_metadata: Some("152".to_owned()),
                }),
            },
            TestCase {
                output: "18.0.2.1+1",
                expected: Some(JavaVersion {
                    major: 18,
                    minor: 0,
                    patch: "2.1".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("1".to_owned()),
                }),
            },
            TestCase {
                output: "17.0.5+8",
                expected: Some(JavaVersion {
                    major: 17,
                    minor: 0,
                    patch: "5".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("8".to_owned()),
                }),
            },
            TestCase {
                output: "17.0.5+8-LTS",
                expected: Some(JavaVersion {
                    major: 17,
                    minor: 0,
                    patch: "5".to_owned(),
                    update_number: None,
                    prerelease: None,
                    build_metadata: Some("8-LTS".to_owned()),
                }),
            },
        ];

        for test_case in expected.iter() {
            let actual = JavaVersion::try_from(test_case.output).ok();
            assert_eq!(actual, test_case.expected);
        }
    }
}
