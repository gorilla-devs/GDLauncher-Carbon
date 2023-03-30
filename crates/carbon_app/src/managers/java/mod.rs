use serde::{Deserialize, Serialize};

use self::parser::{parse_java_version, JavaArch, JavaVersion};

use super::ManagerRef;

use std::{
    collections::{HashMap, HashSet},
    fmt::format,
    path::PathBuf,
};

mod auto_setup;
mod constants;
mod discovery;
mod parser;
mod utils;

pub(crate) struct JavaManager {}

impl JavaManager {
    pub fn new() -> Self {
        Self {}
    }
}

impl ManagerRef<'_, JavaManager> {
    pub async fn get_available_javas(self) -> anyhow::Result<HashMap<u8, JavaComponent>> {
        let db = &self.app.prisma_client;
        let all_javas = db
            .java()
            .find_many(vec![])
            .exec()
            .await?
            .into_iter()
            .map(JavaComponent::from)
            .map(|java| (java.version.major, java))
            .collect();

        Ok(all_javas)
    }

    pub async fn get_default_javas(self) -> anyhow::Result<HashMap<u8, String>> {
        let db = &self.app.prisma_client;
        let all_javas = db
            .default_java()
            .find_many(vec![])
            .exec()
            .await?
            .into_iter()
            .map(|j| (j.major as u8, j.path))
            .collect();

        Ok(all_javas)
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct JavaComponent {
    pub path: String,
    pub arch: JavaArch,
    /// Indicates whether the component has manually been added by the user
    #[serde(rename = "type")]
    pub _type: JavaComponentType,
    pub version: JavaVersion,
}

impl From<crate::db::java::Data> for JavaComponent {
    fn from(java: crate::db::java::Data) -> Self {
        Self {
            path: java.path,
            arch: JavaArch::from(&*java.arch),
            _type: JavaComponentType::from(&*java.r#type),
            version: JavaVersion::from(&*java.full_version),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum JavaComponentType {
    Local,
    Managed,
}

impl From<&str> for JavaComponentType {
    fn from(s: &str) -> Self {
        match s {
            "Local" => Self::Local,
            "Managed" => Self::Managed,
            _ => unreachable!("Uh oh, this shouldn't happen"),
        }
    }
}

impl From<JavaComponentType> for &str {
    fn from(t: JavaComponentType) -> Self {
        match t {
            JavaComponentType::Local => "Local",
            JavaComponentType::Managed => "Managed",
        }
    }
}

pub async fn detect_available_javas() -> anyhow::Result<Vec<JavaComponent>> {
    let mut all_javas = discovery::find_java_paths().await;
    all_javas.push(PathBuf::from("java"));
    let mut available_javas = vec![];

    for java in all_javas {
        match discovery::gather_java_bin_info(&java).await {
            Ok(java_bin_info) => available_javas.push(java_bin_info),
            Err(e) => {
                eprintln!("Failed to gather Java info for {}: {}", java.display(), e);
            }
        };
    }

    Ok(available_javas)
}
