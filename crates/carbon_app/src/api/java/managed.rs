use std::{collections::HashMap, ops::Deref};

use rspc::Type;
use serde::{Deserialize, Serialize};

#[derive(Type, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum FEVendor {
    Azul,
}

impl From<crate::managers::java::managed::Vendor> for FEVendor {
    fn from(v: crate::managers::java::managed::Vendor) -> Self {
        use crate::managers::java::managed::Vendor;
        match v {
            Vendor::Azul => Self::Azul,
        }
    }
}

impl From<FEVendor> for crate::managers::java::managed::Vendor {
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

impl From<crate::managers::java::managed::ManagedJavaOs> for FEManagedJavaOs {
    fn from(v: crate::managers::java::managed::ManagedJavaOs) -> Self {
        use crate::managers::java::managed::ManagedJavaOs;
        match v {
            ManagedJavaOs::Windows => Self::Windows,
            ManagedJavaOs::Linux => Self::Linux,
            ManagedJavaOs::MacOs => Self::MacOs,
        }
    }
}

impl From<FEManagedJavaOs> for crate::managers::java::managed::ManagedJavaOs {
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
    Aarch64,
}

impl From<crate::managers::java::managed::ManagedJavaArch> for FEManagedJavaArch {
    fn from(v: crate::managers::java::managed::ManagedJavaArch) -> Self {
        use crate::managers::java::managed::ManagedJavaArch;
        match v {
            ManagedJavaArch::X64 => Self::X64,
            ManagedJavaArch::Aarch64 => Self::Aarch64,
        }
    }
}

impl From<FEManagedJavaArch> for crate::managers::java::managed::ManagedJavaArch {
    fn from(v: FEManagedJavaArch) -> Self {
        match v {
            FEManagedJavaArch::X64 => Self::X64,
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
pub struct FEManagedJavaArchMap(HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>);

impl Deref for FEManagedJavaArchMap {
    type Target = HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaArchMap> for FEManagedJavaArchMap {
    fn from(v: crate::managers::java::managed::ManagedJavaArchMap) -> Self {
        Self(
            v.into_iter()
                .map(|(k, v)| (k.into(), v.into_iter().map(|v| v.into()).collect()))
                .collect(),
        )
    }
}

#[derive(Type, Debug, Clone, Serialize, Deserialize)]
pub struct FEManagedJavaOsMap(HashMap<FEManagedJavaOs, FEManagedJavaArchMap>);

impl Deref for FEManagedJavaOsMap {
    type Target = HashMap<FEManagedJavaOs, FEManagedJavaArchMap>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<crate::managers::java::managed::ManagedJavaOsMap> for FEManagedJavaOsMap {
    fn from(v: crate::managers::java::managed::ManagedJavaOsMap) -> Self {
        Self(v.into_iter().map(|(k, v)| (k.into(), v.into())).collect())
    }
}
