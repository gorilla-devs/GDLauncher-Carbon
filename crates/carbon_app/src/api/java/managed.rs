use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEVendor {
    Azul,
    Mojang,
}

impl From<crate::domain::java::JavaVendor> for FEVendor {
    fn from(v: crate::domain::java::JavaVendor) -> Self {
        use crate::domain::java::JavaVendor;
        match v {
            JavaVendor::Azul => Self::Azul,
            JavaVendor::Mojang => Self::Mojang,
        }
    }
}

impl From<FEVendor> for crate::domain::java::JavaVendor {
    fn from(v: FEVendor) -> Self {
        match v {
            FEVendor::Azul => Self::Azul,
            FEVendor::Mojang => Self::Mojang,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEManagedJavaOs {
    Windows,
    Linux,
    MacOs,
}

impl From<crate::domain::java::JavaOs> for FEManagedJavaOs {
    fn from(v: crate::domain::java::JavaOs) -> Self {
        use crate::domain::java::JavaOs;
        match v {
            JavaOs::Windows => Self::Windows,
            JavaOs::Linux => Self::Linux,
            JavaOs::MacOs => Self::MacOs,
        }
    }
}

impl From<FEManagedJavaOs> for crate::domain::java::JavaOs {
    fn from(v: FEManagedJavaOs) -> Self {
        match v {
            FEManagedJavaOs::Windows => Self::Windows,
            FEManagedJavaOs::Linux => Self::Linux,
            FEManagedJavaOs::MacOs => Self::MacOs,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEManagedJavaArch {
    X64,
    X86,
    Arm32,
    Arm64,
}

impl From<crate::domain::java::JavaArch> for FEManagedJavaArch {
    fn from(v: crate::domain::java::JavaArch) -> Self {
        use crate::domain::java::JavaArch;
        match v {
            JavaArch::X86_64 => Self::X64,
            JavaArch::X86_32 => Self::X86,
            JavaArch::Arm32 => Self::Arm32,
            JavaArch::Arm64 => Self::Arm64,
        }
    }
}

impl From<FEManagedJavaArch> for crate::domain::java::JavaArch {
    fn from(v: FEManagedJavaArch) -> Self {
        match v {
            FEManagedJavaArch::X64 => Self::X86_64,
            FEManagedJavaArch::X86 => Self::X86_32,
            FEManagedJavaArch::Arm32 => Self::Arm32,
            FEManagedJavaArch::Arm64 => Self::Arm64,
        }
    }
}

#[derive(Type, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaBuild {
    pub id: String,
    pub name: String,
    pub major: u8,
    pub version: String,
    pub release_date: Option<String>,
    /// `u32` because specta refuses `u64`.
    pub size_bytes: Option<u32>,
    /// Names of the advisories that affect this build on this computer.
    pub known_issues: Vec<String>,
}

impl FEManagedJavaBuild {
    fn new(v: crate::managers::java::managed::ManagedJavaBuild, known_issues: Vec<String>) -> Self {
        Self {
            id: v.id,
            name: v.name,
            major: v.major,
            version: v.version.to_string(),
            release_date: v.release_date,
            size_bytes: v.size_bytes.and_then(|b| u32::try_from(b).ok()),
            known_issues,
        }
    }
}

#[derive(Type, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaMajor {
    pub major: u8,
    pub lts: bool,
    pub builds: Vec<FEManagedJavaBuild>,
}

#[derive(Type, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaCatalog {
    pub majors: Vec<FEManagedJavaMajor>,
}

impl FEManagedJavaCatalog {
    pub fn new(
        catalog: crate::managers::java::managed::ManagedJavaCatalog,
        advisories: &[crate::managers::java::advisories::JavaAdvisory],
        arch: &str,
        cpu: &dyn crate::managers::java::advisories::HostCpu,
    ) -> Self {
        Self {
            majors: catalog
                .majors
                .into_iter()
                .map(|m| FEManagedJavaMajor {
                    major: m.major,
                    lts: m.lts,
                    builds: m
                        .builds
                        .into_iter()
                        .map(|b| {
                            let issues = crate::managers::java::advisories::known_issues_for_build(
                                &b.version, arch, advisories, cpu,
                            );
                            FEManagedJavaBuild::new(b, issues)
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

#[derive(Type, Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEJavaAdvisoryMatch {
    pub java_id: String,
    pub advisory_id: String,
    pub name: String,
    pub severity: String,
    pub url: String,
    pub fixed_version: Option<String>,
    /// Auto-update will fix this by itself.
    pub auto_fixable: bool,
    /// An instance uses the Java, directly or through Automatic.
    pub in_use: bool,
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaSetupArgs {
    pub vendor: FEVendor,
    pub id: String,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEManagedJavaSetupProgress {
    Idle,
    Downloading(String, String),
    Extracting(String, String),
    Done,
}

impl From<crate::managers::java::managed::Step> for FEManagedJavaSetupProgress {
    fn from(v: crate::managers::java::managed::Step) -> Self {
        use crate::managers::java::managed::Step;

        match v {
            Step::Idle => Self::Idle,
            Step::Downloading(a, b) => Self::Downloading(a.to_string(), b.to_string()),
            Step::Extracting(a, b) => Self::Extracting(a.to_string(), b.to_string()),
            Step::Done => Self::Done,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaInstall {
    pub build_id: String,
    pub vendor: FEVendor,
    pub major: u8,
    pub version: String,
    pub progress: FEManagedJavaSetupProgress,
}

impl FEManagedJavaInstall {
    pub fn new(build_id: String, p: crate::managers::java::managed::InstallProgress) -> Self {
        Self {
            build_id,
            vendor: p.vendor.into(),
            major: p.major,
            version: p.version,
            progress: p.step.into(),
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaUpdate {
    pub java_id: String,
    pub current_version: String,
    pub latest_version: String,
    pub pinned: bool,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FEMinecraftJavaMajor {
    pub major: u8,
    pub from_version: String,
    pub to_version: String,
    pub newest: bool,
    pub oldest: bool,
}

impl From<crate::managers::java::minecraft_majors::MinecraftJavaMajor> for FEMinecraftJavaMajor {
    fn from(m: crate::managers::java::minecraft_majors::MinecraftJavaMajor) -> Self {
        Self {
            major: m.major,
            from_version: m.from_version,
            to_version: m.to_version,
            newest: m.newest,
            oldest: m.oldest,
        }
    }
}
