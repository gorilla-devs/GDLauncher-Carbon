use rspc::RouterBuilderLike;

use crate::{
    api::{
        keys::modplatforms::{
            CURSEFORGE_GET_CATEGORIES, CURSEFORGE_GET_FILES, CURSEFORGE_GET_MOD,
            CURSEFORGE_GET_MODS, CURSEFORGE_GET_MOD_DESCRIPTION, CURSEFORGE_GET_MOD_FILE,
            CURSEFORGE_GET_MOD_FILES, CURSEFORGE_GET_MOD_FILE_CHANGELOG, CURSEFORGE_SEARCH,
            MODRINTH_SEARCH,
        },
        router::router,
    },
    managers::App,
};

mod curseforge;
mod modrinth;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        // Curseforge
        query CURSEFORGE_SEARCH[app, filters: curseforge::filters::FEModSearchParameters] {
            let modplatforms = &app.modplatforms_manager;
                let response = modplatforms.curseforge.search(filters.into()).await?;

            Ok(curseforge::responses::FEModSearchResponse::from(response))
        }

        query CURSEFORGE_GET_CATEGORIES[app, args: ()] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_categories().await?;

            Ok(curseforge::responses::FECategoriesResponse::from(response))
        }

        query CURSEFORGE_GET_MOD[app, mod_parameters: curseforge::filters::FEModParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModResponse::from(response))
        }

        query CURSEFORGE_GET_MODS[app, mod_parameters: curseforge::filters::FEModsParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mods(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModsResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_DESCRIPTION[app, mod_parameters: curseforge::filters::FEModDescriptionParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_description(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModDescriptionResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE[app, mod_parameters: curseforge::filters::FEModFileParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFileResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILES[app, mod_parameters: curseforge::filters::FEModFilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_files(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFilesResponse::from(response))
        }

        query CURSEFORGE_GET_FILES[app, mod_parameters: curseforge::filters::FEFilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_files(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEFilesResponse::from(response))
        }

        query CURSEFORGE_GET_MOD_FILE_CHANGELOG[app, mod_parameters: curseforge::filters::FEModFileChangelogParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file_changelog(mod_parameters.into()).await?;

            Ok(curseforge::responses::FEModFileChangelogResponse::from(response))
        }

        // Modrinth
        query MODRINTH_SEARCH[app, search_params: modrinth::filters::FEProjectSearchParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.modrinth.search(search_params.into()).await?;

            Ok(modrinth::responses::FEProjectSearchResponse::from(response))

        }
        // query MODRINTH_GET_CATEGORIES[app ]
    }
}
