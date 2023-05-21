use std::{collections::HashMap, ops::Deref};

use rspc::Type;
use serde::{Deserialize, Serialize};

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum FEVendor {
    Azul,
}

impl From<crate::domain::java::Vendor> for FEVendor {
    fn from(v: crate::domain::java::Vendor) -> Self {
        use crate::domain::java::Vendor;
        match v {
            Vendor::Azul => Self::Azul,
        }
    }
}

impl From<FEVendor> for crate::domain::java::Vendor {
    fn from(v: FEVendor) -> Self {
        match v {
            FEVendor::Azul => Self::Azul,
        }
    }
}

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
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
pub enum FEManagedJavaArch {
    X64,
    X86,
    Aarch64,
}

impl From<crate::domain::java::JavaArch> for FEManagedJavaArch {
    fn from(v: crate::domain::java::JavaArch) -> Self {
        use crate::domain::java::JavaArch;
        match v {
            JavaArch::X64 => Self::X64,
            JavaArch::X86 => Self::X64,
            JavaArch::Aarch64 => Self::Aarch64,
        }
    }
}

impl From<FEManagedJavaArch> for crate::domain::java::JavaArch {
    fn from(v: FEManagedJavaArch) -> Self {
        match v {
            FEManagedJavaArch::X64 => Self::X64,
            FEManagedJavaArch::X86 => Self::X64,
            FEManagedJavaArch::Aarch64 => Self::Aarch64,
        }
    }
}

#[derive(Type, Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct FEManagedJavaVersion {
    id: String,
    name: String,
    download_url: String,
}

impl From<crate::managers::java::managed::ManagedJavaVersion> for FEManagedJavaVersion {
    fn from(v: crate::managers::java::managed::ManagedJavaVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            download_url: v.download_url,
        }
    }
}

impl From<FEManagedJavaVersion> for crate::managers::java::managed::ManagedJavaVersion {
    fn from(v: FEManagedJavaVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            download_url: v.download_url,
        }
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct FEManagedJavaArchMap(pub HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>);

impl Deref for FEManagedJavaArchMap {
    type Target = HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaArchMap> for FEManagedJavaArchMap {
    fn from(v: crate::managers::java::managed::ManagedJavaArchMap) -> Self {
        Self(
            v.0.into_iter()
                .map(|(k, v)| (k.into(), v.into_iter().map(|v| v.into()).collect()))
                .collect(),
        )
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct FEManagedJavaOsMap(pub HashMap<FEManagedJavaOs, FEManagedJavaArchMap>);

impl Deref for FEManagedJavaOsMap {
    type Target = HashMap<FEManagedJavaOs, FEManagedJavaArchMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaOsMap> for FEManagedJavaOsMap {
    fn from(v: crate::managers::java::managed::ManagedJavaOsMap) -> Self {
        Self(v.0.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}

#[derive(Type, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FEManagedJavaSetupArgs {
    os: FEManagedJavaOs,
    arch: FEManagedJavaArch,
    vendor: FEVendor,
    id: String,
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FEManagedJavaSetupProgress {
    Idle,
    Downloading(String, String),
    Extracting(String, String),
    Done,
}
