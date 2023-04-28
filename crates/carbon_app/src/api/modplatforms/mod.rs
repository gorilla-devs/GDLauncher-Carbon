use rspc::RouterBuilderLike;

use crate::{
    api::{
        keys::modplatforms::{
            CURSEFORGE_GET_CATEGORIES, CURSEFORGE_GET_FILES, CURSEFORGE_GET_MOD,
            CURSEFORGE_GET_MOD_DESCRIPTION, CURSEFORGE_GET_MOD_FILE, CURSEFORGE_GET_MOD_FILE_CHANGELOG,
            CURSEFORGE_GET_MOD_FILES, CURSEFORGE_GET_MODS, CURSEFORGE_SEARCH,
        },
        modplatforms::{
            curseforge::filters::{
                FEFilesParameters, FEModDescriptionParameters, FEModFileChangelogParameters,
                FEModFileParameters, FEModFilesParameters, FEModParameters, FEModSearchParameters,
                FEModsParameters,
            },
            curseforge::responses::{
                FECategoriesResponse, FEFilesResponse, FEModDescriptionResponse,
                FEModFileChangelogResponse, FEModFileResponse, FEModFilesResponse, FEModResponse,
                FEModSearchResponse, FEModsResponse,
            },
        },
        router::router,
    },
    managers::App,
};

mod curseforge;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query CURSEFORGE_SEARCH[app, filters: FEModSearchParameters] {
            let modplatforms = &app.modplatforms_manager;
                let response = modplatforms.curseforge.search(filters.into()).await?;

            Ok(FEModSearchResponse::from(response))
        }

        query CURSEFORGE_GET_CATEGORIES[app, _: ()] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_categories().await?;

            Ok(FECategoriesResponse::from(response))
        }

        query CURSEFORGE_GET_MOD[app, mod_parameters: FEModParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod(mod_parameters.into()).await?;

            Ok(FEModResponse::from(response))
        }

        query CURSEFORGE_GET_MODS[app, mod_parameters: FEModsParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mods(mod_parameters.into()).await?;

            Ok(FEModsResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_DESCRIPTION[app, mod_parameters: FEModDescriptionParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_description(mod_parameters.into()).await?;

            Ok(FEModDescriptionResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE[app, mod_parameters: FEModFileParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file(mod_parameters.into()).await?;

            Ok(FEModFileResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILES[app, mod_parameters: FEModFilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_files(mod_parameters.into()).await?;

            Ok(FEModFilesResponse::from(response))
        }

        query CURSEFORGE_GET_FILES[app, mod_parameters: FEFilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_files(mod_parameters.into()).await?;

            Ok(FEFilesResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE_CHANGELOG[app, mod_parameters: FEModFileChangelogParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file_changelog(mod_parameters.into()).await?;

            Ok(FEModFileChangelogResponse::from(response))
        }
    }
}
