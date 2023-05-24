use crate::api::java::managed::{FEManagedJavaSetupArgs, FEManagedJavaSetupProgress};
use crate::api::managers::App;
use crate::api::router::router;
use crate::domain::java::{JavaComponentType, JavaVendor};
use crate::{api::keys::java::*, domain::java::Java};
use rspc::{RouterBuilderLike, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use self::managed::{FEManagedJavaArch, FEManagedJavaOs, FEManagedJavaOsMap, FEVendor};

mod managed;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query GET_AVAILABLE_JAVAS[app, _args: ()] {
            get_all_available_javas(app, _args).await
        }

        query GET_MANAGED_OS[app, _args: ()] {
            get_managed_os(app, _args)
        }

        query GET_MANAGED_ARCH[app, _args: ()] {
            get_managed_arch(app, _args)
        }

        query GET_MANAGED_VENDORS[app, _args: ()] {
            get_managed_vendors(app, _args)
        }

        query GET_MANAGED_VERSIONS_BY_VENDOR[app, args: FEVendor] {
            get_managed_versions_by_vendor(app, args).await
        }

        mutation SETUP_MANAGED_JAVA[app, args: FEManagedJavaSetupArgs] {
            setup_managed_java(app, args).await
        }

        query GET_SETUP_MANAGED_JAVA_PROGRESS[app, args: ()] {
            get_setup_managed_java_progress(app, args).await
        }

        query GET_SYSTEM_JAVA_PROFILES[app, args: ()] {
            get_system_java_profiles(app, args).await
        }

        mutation UPDATE_SYSTEM_JAVA_PROFILE_PATH[app, args: FEUpdateSystemJavaProfileArgs] {
            update_system_java_profile_path(app, args).await
        }
    }
}

async fn get_all_available_javas(
    app: App,
    _args: (),
) -> anyhow::Result<HashMap<u8, Vec<FEJavaComponent>>> {
    let all_javas = app.java_manager().get_available_javas().await?;

    let mut result = HashMap::new();
    for (major, javas) in all_javas {
        result.insert(
            major,
            javas.into_iter().map(FEJavaComponent::from).collect(),
        );
    }

    Ok(result)
}

fn get_managed_os(app: App, _args: ()) -> anyhow::Result<Vec<FEManagedJavaOs>> {
    let all_os = app.java_manager().managed_service.get_all_os();

    Ok(all_os.into_iter().map(FEManagedJavaOs::from).collect())
}

fn get_managed_arch(app: App, _args: ()) -> anyhow::Result<Vec<FEManagedJavaArch>> {
    let all_arch = app.java_manager().managed_service.get_all_archs();

    Ok(all_arch.into_iter().map(FEManagedJavaArch::from).collect())
}

fn get_managed_vendors(app: App, _args: ()) -> anyhow::Result<Vec<FEVendor>> {
    let all_vendors = app.java_manager().managed_service.get_all_vendors();

    Ok(all_vendors.into_iter().map(FEVendor::from).collect())
}

async fn get_managed_versions_by_vendor(
    app: App,
    args: FEVendor,
) -> anyhow::Result<FEManagedJavaOsMap> {
    let managed_java_map_os = app
        .java_manager()
        .managed_service
        .get_versions_for_vendor(JavaVendor::from(args))
        .await?;

    Ok(managed_java_map_os.into())
}

async fn setup_managed_java(app: App, args: FEManagedJavaSetupArgs) -> anyhow::Result<()> {
    app.java_manager()
        .managed_service
        .setup_managed(
            args.os.into(),
            args.arch.into(),
            args.vendor.into(),
            args.id,
            app.clone(),
        )
        .await
}

async fn get_setup_managed_java_progress(
    app: App,
    _args: (),
) -> anyhow::Result<FEManagedJavaSetupProgress> {
    let res = app
        .java_manager()
        .managed_service
        .setup_progress
        .lock()
        .await
        .clone();

    Ok(res.into())
}

async fn get_system_java_profiles(app: App, _args: ()) -> anyhow::Result<Vec<FESystemJavaProfile>> {
    let profiles = app.java_manager().get_system_java_profiles().await?;

    Ok(profiles
        .into_iter()
        .map(FESystemJavaProfile::from)
        .collect())
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FEUpdateSystemJavaProfileArgs {
    pub profile_name: FESystemJavaProfileName,
    pub java_id: String,
}

async fn update_system_java_profile_path(
    app: App,
    args: FEUpdateSystemJavaProfileArgs,
) -> anyhow::Result<()> {
    app.java_manager()
        .update_system_java_profile_path(args.profile_name.into(), args.java_id)
        .await
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct Javas(HashMap<u8, Vec<FEJavaComponent>>);

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct FEJavaComponent {
    id: String,
    path: String,
    version: String,
    #[serde(rename = "type")]
    _type: FEJavaComponentType,
    is_valid: bool,
}

impl From<Java> for FEJavaComponent {
    fn from(java: Java) -> Self {
        Self {
            id: java.id,
            path: java.component.path,
            version: String::from(java.component.version),
            _type: FEJavaComponentType::from(java.component._type),
            is_valid: java.is_valid,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
enum FEJavaComponentType {
    Local,
    Managed,
    Custom,
}

impl From<JavaComponentType> for FEJavaComponentType {
    fn from(t: JavaComponentType) -> Self {
        match t {
            JavaComponentType::Local => Self::Local,
            JavaComponentType::Managed => Self::Managed,
            JavaComponentType::Custom => Self::Custom,
        }
    }
}

#[derive(Type, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetDefaultArgs {
    major_version: u8,
    id: String,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FESystemJavaProfileName {
    Legacy,
    Alpha,
    Beta,
    Gamma,
    MinecraftJavaExe,
}

impl From<crate::domain::java::SystemJavaProfileName> for FESystemJavaProfileName {
    fn from(name: crate::domain::java::SystemJavaProfileName) -> Self {
        use crate::domain::java::SystemJavaProfileName;
        match name {
            SystemJavaProfileName::Legacy => Self::Legacy,
            SystemJavaProfileName::Alpha => Self::Alpha,
            SystemJavaProfileName::Beta => Self::Beta,
            SystemJavaProfileName::Gamma => Self::Gamma,
            SystemJavaProfileName::MinecraftJavaExe => Self::MinecraftJavaExe,
        }
    }
}

impl From<FESystemJavaProfileName> for crate::domain::java::SystemJavaProfileName {
    fn from(name: FESystemJavaProfileName) -> Self {
        match name {
            FESystemJavaProfileName::Legacy => Self::Legacy,
            FESystemJavaProfileName::Alpha => Self::Alpha,
            FESystemJavaProfileName::Beta => Self::Beta,
            FESystemJavaProfileName::Gamma => Self::Gamma,
            FESystemJavaProfileName::MinecraftJavaExe => Self::MinecraftJavaExe,
        }
    }
}

#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
struct FESystemJavaProfile {
    name: FESystemJavaProfileName,
    java_id: Option<String>,
}

impl From<crate::domain::java::SystemJavaProfile> for FESystemJavaProfile {
    fn from(profile: crate::domain::java::SystemJavaProfile) -> Self {
        Self {
            name: profile.name.into(),
            java_id: profile.java.map(|j| j.id),
        }
    }
}
