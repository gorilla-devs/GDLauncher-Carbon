use std::{collections::HashMap, ops::Deref};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::java::JavaVersion;

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
#[serde(rename_all = "camelCase")]
pub enum FEVendor {
    Azul,
}

impl From<crate::domain::java::JavaVendor> for FEVendor {
    fn from(v: crate::domain::java::JavaVendor) -> Self {
        use crate::domain::java::JavaVendor;
        match v {
            JavaVendor::Azul => Self::Azul,
        }
    }
}

impl From<FEVendor> for crate::domain::java::JavaVendor {
    fn from(v: FEVendor) -> Self {
        match v {
            FEVendor::Azul => Self::Azul,
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
            JavaArch::X86_32 => Self::X64,
            JavaArch::Arm32 => Self::Arm32,
            JavaArch::Arm64 => Self::Arm64,
        }
    }
}

impl From<FEManagedJavaArch> for crate::domain::java::JavaArch {
    fn from(v: FEManagedJavaArch) -> Self {
        match v {
            FEManagedJavaArch::X64 => Self::X86_64,
            FEManagedJavaArch::X86 => Self::X86_64,
            FEManagedJavaArch::Arm32 => Self::Arm32,
            FEManagedJavaArch::Arm64 => Self::Arm64,
        }
    }
}

#[derive(Type, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaVersion {
    id: String,
    name: String,
    download_url: String,
    java_version: String,
}

impl From<crate::managers::java::managed::ManagedJavaVersion>
    for FEManagedJavaVersion
{
    fn from(v: crate::managers::java::managed::ManagedJavaVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            download_url: v.download_url,
            java_version: v.java_version.to_string(),
        }
    }
}

impl TryFrom<FEManagedJavaVersion>
    for crate::managers::java::managed::ManagedJavaVersion
{
    type Error = anyhow::Error;

    fn try_from(v: FEManagedJavaVersion) -> Result<Self, Self::Error> {
        Ok(Self {
            id: v.id,
            name: v.name,
            download_url: v.download_url,
            java_version: JavaVersion::try_from(&*v.java_version)?,
        })
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaArchMap(
    pub HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>,
);

impl Deref for FEManagedJavaArchMap {
    type Target = HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaArchMap>
    for FEManagedJavaArchMap
{
    fn from(v: crate::managers::java::managed::ManagedJavaArchMap) -> Self {
        Self(
            v.0.into_iter()
                .map(|(k, v)| {
                    (k.into(), v.into_iter().map(|v| v.into()).collect())
                })
                .collect(),
        )
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaOsMap(
    pub HashMap<FEManagedJavaOs, FEManagedJavaArchMap>,
);

impl Deref for FEManagedJavaOsMap {
    type Target = HashMap<FEManagedJavaOs, FEManagedJavaArchMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaOsMap>
    for FEManagedJavaOsMap
{
    fn from(v: crate::managers::java::managed::ManagedJavaOsMap) -> Self {
        Self(v.0.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

#[derive(Type, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaSetupArgs {
    pub os: FEManagedJavaOs,
    pub arch: FEManagedJavaArch,
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
            Step::Downloading(a, b) => {
                Self::Downloading(a.to_string(), b.to_string())
            }
            Step::Extracting(a, b) => {
                Self::Extracting(a.to_string(), b.to_string())
            }
            Step::Done => Self::Done,
        }
    }
}
