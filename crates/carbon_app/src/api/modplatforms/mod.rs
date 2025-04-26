use self::{
    curseforge::structs::CFFEFile,
    modrinth::structs::{MRFEVersion, MRFEVersionFile},
};
use crate::{
    api::{
        keys::modplatforms::{
            CURSEFORGE_GET_FILES, CURSEFORGE_GET_MOD, CURSEFORGE_GET_MOD_DESCRIPTION,
            CURSEFORGE_GET_MOD_FILE, CURSEFORGE_GET_MOD_FILE_CHANGELOG, CURSEFORGE_GET_MOD_FILES,
            CURSEFORGE_GET_MODS, CURSEFORGE_SEARCH, GET_UNIFIED_CATEGORIES, GET_UNIFIED_MODLOADERS,
            MODRINTH_GET_PROJECT, MODRINTH_GET_PROJECT_TEAM, MODRINTH_GET_PROJECT_VERSIONS,
            MODRINTH_GET_PROJECTS, MODRINTH_GET_TEAM, MODRINTH_GET_VERSION, MODRINTH_GET_VERSIONS,
            MODRINTH_SEARCH, UNIFIED_SEARCH, UNIFIED_SEARCH_PROJECT_TYPE,
        },
        modplatforms::curseforge::structs::CFFEModLoaderType,
        router::router,
    },
    managers::App,
    mirror_into,
};
use carbon_platforms::{curseforge::ClassId, modrinth::project::ProjectType};
use curseforge::structs::CFFEClassId;
use modrinth::structs::MRFEProjectType;
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;
use tracing::info;
use unified::{FEUnifiedCategories, FEUnifiedCategory, FEUnifiedModLoaders, FEUnifiedSearchType};

mod curseforge;
mod filters;
mod modrinth;
mod unified;

pub(super) fn mount() -> RouterBuilder<App> {
    router! {
        // Curseforge
        query CURSEFORGE_SEARCH[app, filters: curseforge::filters::CFFEModSearchParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.search(filters.into()).await?;

            Ok(curseforge::responses::FEModSearchResponse::from(response))
        }

        query CURSEFORGE_GET_MOD[app, mod_parameters: curseforge::filters::CFFEModParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mod(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModResponse::from(response))
        }

        query CURSEFORGE_GET_MODS[app, mod_parameters: curseforge::filters::CFFEModsParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mods(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModsResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_DESCRIPTION[app, mod_parameters: curseforge::filters::CFFEModDescriptionParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mod_description(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModDescriptionResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE[app, mod_parameters: curseforge::filters::CFFEModFileParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mod_file(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFileResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILES[app, mod_parameters: curseforge::filters::CFFEModFilesParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mod_files(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFilesResponse::from(response))
        }

        query CURSEFORGE_GET_FILES[app, mod_parameters: curseforge::filters::CFFEFilesParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_files(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEFilesResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE_CHANGELOG[app, mod_parameters: curseforge::filters::CFFEModFileChangelogParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.curseforge.get_mod_file_changelog(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFileChangelogResponse::from(response))
        }

        // Modrinth
        query MODRINTH_SEARCH[app, search_params: modrinth::filters::MRFEProjectSearchParameters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.search(search_params.into()).await?;

            Ok(modrinth::responses::MRFEProjectSearchResponse::from(response))

        }

        query GET_UNIFIED_MODLOADERS[app, _args: ()] {
            let modplatforms = app.modplatforms_manager();
            let curseforge = CFFEModLoaderType::iter().collect::<Vec<_>>();
            let modrinth = modrinth::responses::MRFELoadersResponse::from(modplatforms.modrinth.get_loaders().await?);

            Ok(FEUnifiedModLoaders {
                curseforge: curseforge.into_iter().map(Into::into).collect(),
                modrinth: modrinth.into_iter().map(|v| v.name).map(Into::into).collect(),
            })
        }

        query MODRINTH_GET_PROJECT[app, project: modrinth::filters::MRFEProjectID  ] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_project(project.into()).await?;

            Ok(modrinth::structs::MRFEProject::from(response))
        }

        query MODRINTH_GET_PROJECTS[app, projects: modrinth::filters::MRFEProjectIDs] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_projects(projects.into()).await?;

            Ok(modrinth::responses::MRFEProjectsResponse::from(response))
        }

        query MODRINTH_GET_PROJECT_VERSIONS[app, filters: modrinth::filters::MRFEProjectVersionsFilters] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_project_versions(filters.into()).await?;

            Ok(modrinth::responses::MRFEVersionsResponse::from(response))
        }

        query MODRINTH_GET_VERSION[app, version: modrinth::filters::MRFEVersionID] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_version(version.into()).await?;

            Ok(modrinth::structs::MRFEVersion::from(response))
        }

        query MODRINTH_GET_VERSIONS[app, versions: modrinth::filters::MRFEVersionIDs] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_versions(versions.into()).await?;

            Ok(modrinth::responses::MRFEVersionsResponse::from(response))
        }

        query MODRINTH_GET_PROJECT_TEAM[app, project: modrinth::filters::MRFEProjectID] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_project_team(project.into()).await?;

            Ok(modrinth::responses::MRFETeamResponse::from(response))
        }

        query MODRINTH_GET_TEAM[app, team: modrinth::filters::MRFETeamID] {
            let modplatforms = app.modplatforms_manager();
            let response = modplatforms.modrinth.get_team(team.into()).await?;

            Ok(modrinth::responses::MRFETeamResponse::from(response))
        }

        query UNIFIED_SEARCH[app, search_params: filters::FEUnifiedSearchParameters] {
            match search_params.search_api {
                Some(FESearchAPI::Curseforge) => {
                    let search_params: curseforge::filters::CFFEModSearchParameters = search_params.try_into()?;
                    let modplatforms = app.modplatforms_manager();
                    let curseforge_response = modplatforms.curseforge.search(search_params.into()).await?;
                    let fe_curseforge_response = curseforge::responses::FEModSearchResponse::from(curseforge_response);
                    Ok(unified::FEUnifiedSearchResponse::from(fe_curseforge_response))
                }
                Some(FESearchAPI::Modrinth) => {
                    let search_params: modrinth::filters::MRFEProjectSearchParameters = search_params.try_into()?;
                    let modplatforms = app.modplatforms_manager();
                    let modrinth_response = modplatforms.modrinth.search(search_params.into()).await?;
                    let fe_modrinth_response = modrinth::responses::MRFEProjectSearchResponse::from(modrinth_response);
                    Ok(unified::FEUnifiedSearchResponse::from(fe_modrinth_response))
                }
                None => {
                    // Search both platforms and merge results
                    let modplatforms = app.modplatforms_manager();

                    let cf_params: curseforge::filters::CFFEModSearchParameters = search_params.clone().try_into()?;
                    let mr_params: modrinth::filters::MRFEProjectSearchParameters = search_params.try_into()?;

                    let (cf_response, mr_response) = tokio::try_join!(
                        modplatforms.curseforge.search(cf_params.into()),
                        modplatforms.modrinth.search(mr_params.into())
                    )?;

                    let cf_results = curseforge::responses::FEModSearchResponse::from(cf_response);
                    let mr_results = modrinth::responses::MRFEProjectSearchResponse::from(mr_response);

                    let merged = unified::FEUnifiedSearchResponse::merge(cf_results.into(), mr_results.into());
                    Ok(merged)
                }
            }
        }

        query UNIFIED_SEARCH_PROJECT_TYPE[app, _args: ()] {
            Ok(FEUnifiedSearchType::iter().collect::<Vec<_>>())
        }

        query GET_UNIFIED_CATEGORIES[app, _args:()] {
            let modplatforms = app.modplatforms_manager();
            let curseforge_categories = modplatforms.curseforge.get_categories();
            let modrinth_categories = modplatforms.modrinth.get_categories();

            let (cf_categories, mr_categories) = tokio::try_join!(
                curseforge_categories,
                modrinth_categories
            )?;

            let cf_categories = cf_categories.data.into_iter().map(|category| (category.id, FEUnifiedCategory::from(category))).collect();
            let mr_categories = mr_categories.into_iter().map(|category| (category.name.clone(), FEUnifiedCategory::from(category))).collect();

            Ok(FEUnifiedCategories {
                modrinth: mr_categories,
                curseforge: cf_categories,
            })
        }
    }
}

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum FESearchAPI {
    Curseforge,
    Modrinth,
}

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
#[repr(i32)]
pub enum ModChannel {
    Alpha = 0,
    Beta,
    Stable,
}
impl Default for ModChannel {
    fn default() -> Self {
        Self::Stable
    }
}

impl TryFrom<i32> for ModChannel {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Alpha),
            1 => Ok(Self::Beta),
            2 => Ok(Self::Stable),
            _ => Err(anyhow::anyhow!(
                "Invalid mod channel id {value} not in range 0..=2"
            )),
        }
    }
}

mirror_into!(
    ModChannel,
    carbon_platforms::ModChannel,
    |value| match value {
        Other::Alpha => Self::Alpha,
        Other::Beta => Self::Beta,
        Other::Stable => Self::Stable,
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
pub enum ModPlatform {
    Curseforge,
    Modrinth,
}

mirror_into!(
    ModPlatform,
    carbon_platforms::ModPlatform,
    |value| match value {
        Other::Curseforge => Self::Curseforge,
        Other::Modrinth => Self::Modrinth,
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone, Copy)]
pub struct ModChannelWithUsage {
    pub channel: ModChannel,
    pub allow_updates: bool,
}

mirror_into!(
    ModChannelWithUsage,
    carbon_platforms::ModChannelWithUsage,
    |value| {
        Self {
            channel: value.channel.into(),
            allow_updates: value.allow_updates,
        }
    }
);

#[derive(Type, Debug, Deserialize, Serialize, Clone)]
pub struct ModSources {
    pub channels: Vec<ModChannelWithUsage>,
    pub platform_blacklist: Vec<ModPlatform>,
}

mirror_into!(ModSources, carbon_platforms::ModSources, |value| Self {
    channels: value.channels.into_iter().map(Into::into).collect(),
    platform_blacklist: value
        .platform_blacklist
        .into_iter()
        .map(Into::into)
        .collect(),
});

#[derive(Type, Debug, Serialize)]
#[serde(tag = "platform")]
pub enum RemoteVersion {
    Curseforge(CFFEFile),
    Modrinth(MRFEVersion),
}

impl From<carbon_platforms::RemoteVersion> for RemoteVersion {
    fn from(value: carbon_platforms::RemoteVersion) -> Self {
        use carbon_platforms::RemoteVersion as Other;

        match value {
            Other::Curseforge(cf) => Self::Curseforge(cf.into()),
            Other::Modrinth(mr) => Self::Modrinth(mr.into()),
        }
    }
}
