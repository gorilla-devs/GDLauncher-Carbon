use crate::{
    api::{
        keys::modplatforms::{
            CURSEFORGE_GET_CATEGORIES, CURSEFORGE_GET_FILES, CURSEFORGE_GET_MOD,
            CURSEFORGE_GET_MODS, CURSEFORGE_GET_MOD_DESCRIPTION, CURSEFORGE_GET_MOD_FILE,
            CURSEFORGE_GET_MOD_FILE_CHANGELOG, CURSEFORGE_SEARCH,
        },
        router::router,
    },
    domain::modplatforms::curseforge::filters::{
        FilesParameters, ModDescriptionParameters, ModFileChangelogParameters, ModFileParameters,
        ModFilesParameters, ModParameters, ModSearchParameters, ModsParameters,
    },
    managers::App,
};
use rspc::RouterBuilderLike;

pub(super) fn mount() -> impl RouterBuilderLike<App> {
    router! {
        query CURSEFORGE_SEARCH[app, filters: ModSearchParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.search(filters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_CATEGORIES[app, _: ()] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_categories().await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MOD[app, mod_parameters: ModParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MODS[app, mod_parameters: ModsParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mods(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MOD_DESCRIPTION[app, mod_parameters: ModDescriptionParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_description(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MOD_FILE[app, mod_parameters: ModFileParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MOD_FILE[app, mod_parameters: ModFilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_files(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_FILES[app, mod_parameters: FilesParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_files(mod_parameters).await?;

            Ok(response)
        }

        query CURSEFORGE_GET_MOD_FILE_CHANGELOG[app, mod_parameters: ModFileChangelogParameters] {
            let modplatforms = &app.modplatforms_manager;
            let response = modplatforms.curseforge.get_mod_file_changelog(mod_parameters).await?;

            Ok(response)
        }
    }
}
