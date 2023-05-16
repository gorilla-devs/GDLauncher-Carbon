use std::collections::HashMap;

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

pub type FEManagedJavaArchMap = HashMap<FEManagedJavaArch, Vec<FEManagedJavaVersion>>;
pub type FEManagedJavaOsMap = HashMap<FEManagedJavaOs, FEManagedJavaArchMap>;
