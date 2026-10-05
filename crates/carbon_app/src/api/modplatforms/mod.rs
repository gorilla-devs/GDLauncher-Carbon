use crate::{
    api::{keys::modplatforms::*, router::router},
    managers::App,
};
use carbon_platforms::{
    curseforge::{
        ClassId,
        filters::{ModDescriptionParameters, ModParameters},
    },
    modrinth::project::ProjectType,
};
use curseforge::structs::{CFFEFile, CFFEModLoaderType};
use modrinth::structs::MRFEVersion;
use responses::{
    FEUnifiedCategories, FEUnifiedCategory, FEUnifiedModLoaderType, FEUnifiedModLoaders,
    FEUnifiedPlatform, FEUnifiedSearchType,
};
use rspc::RouterBuilder;
use serde::{Deserialize, Serialize};
use specta::Type;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

mod common;
mod curseforge;
mod filters;
mod modrinth;
mod responses;

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
                Some(FEUnifiedPlatform::Curseforge) => {
                    let modplatforms = app.modplatforms_manager();
                    let curseforge_response = modplatforms.curseforge.search(search_params.try_into()?).await?;
                    Ok(responses::FEUnifiedSearchResponse::from(curseforge_response))
                }
                Some(FEUnifiedPlatform::Modrinth) => {
                    let modplatforms = app.modplatforms_manager();
                    let modrinth_response = modplatforms.modrinth.search(search_params.into()).await?;
                    Ok(responses::FEUnifiedSearchResponse::from(modrinth_response))
                }
                None => {
                    // Search both platforms and merge results
                    let modplatforms = app.modplatforms_manager();

                    let cf_params = search_params.clone().try_into()?;
                    let (cf_response, mr_response) = tokio::try_join!(
                        modplatforms.curseforge.search(cf_params),
                        modplatforms.modrinth.search(search_params.into())
                    )?;

                    let merged = responses::FEUnifiedSearchResponse::merge(cf_response.into(), mr_response.into());
                    Ok(merged)
                }
            }
        }

        query UNIFIED_GET_PROJECT[app, project: filters::FEUnifiedProjectID] {
            let modplatforms = app.modplatforms_manager();
            let response: responses::FEUnifiedSearchResultWithDescription = match project {
                filters::FEUnifiedProjectID::Curseforge(id) => {
                    let response = modplatforms.curseforge.get_mod(ModParameters {
                        mod_id: id
                    });

                    let description = modplatforms.curseforge.get_mod_description(ModDescriptionParameters {
                        mod_id: id
                    });

                    let (response, description) = tokio::try_join!(response, description)?;

                    responses::FEUnifiedSearchResultWithDescription {
                        result: responses::FEUnifiedSearchResult::from(response),
                        full_description_body: description.data,
                    }
                }
                filters::FEUnifiedProjectID::Modrinth(id) => {
                    let project_response = modplatforms.modrinth.get_project(id.into());
                    let project = project_response.await?;

                    let team_response = modplatforms.modrinth.get_team(modrinth::filters::MRFETeamID(project.team.clone()).into()).await.ok();

                    let body = markdown::to_html_with_options(&project.body, &markdown::Options {
                        compile: markdown::CompileOptions {
                          allow_dangerous_html: true,
                          allow_dangerous_protocol: true,
                          ..markdown::CompileOptions::default()
                        },
                        ..markdown::Options::default()
                    }).map_err(|e| anyhow::anyhow!("Failed to convert markdown to html: {}", e))?;

                    responses::FEUnifiedSearchResultWithDescription {
                        result: responses::FEUnifiedSearchResult::from_project_with_team(
                            project,
                            team_response.map(|tr| tr.into_iter().map(|member| member.into()).collect())
                        ),
                        full_description_body: body,
                    }
                }
            };

            Ok(response)
        }

        query UNIFIED_GET_PROJECTS_BY_IDS[app, request: filters::FEUnifiedBatchRequest] {
            use carbon_platforms::curseforge::filters::{ModSearchParameters, ModSearchParametersQuery, ModsParameters, ModsParametersBody};
            use carbon_platforms::modrinth::search::ProjectIDs;
            use std::collections::HashSet;

            let modplatforms = app.modplatforms_manager();
            let mut results: Vec<responses::FEUnifiedSearchResult> = Vec::new();
            let mut errors: Vec<String> = Vec::new();
            let mut seen_cf_ids: HashSet<i32> = HashSet::new();
            let mut seen_mr_ids: HashSet<String> = HashSet::new();

            // 1. Fetch CurseForge by numeric IDs
            if !request.curseforge_ids.is_empty() {
                match modplatforms.curseforge.get_mods(ModsParameters {
                    body: ModsParametersBody { mod_ids: request.curseforge_ids.clone() }
                }).await {
                    Ok(response) => {
                        let returned: HashSet<i32> = response.data.iter().map(|m|m.id).collect();
                        for id in &request.curseforge_ids { if !returned.contains(id) { errors.push(format!("CurseForge ID {id} not found")); } }
                        for mod_data in response.data {
                            seen_cf_ids.insert(mod_data.id);
                            results.push(responses::FEUnifiedSearchResult::from(mod_data));
                        }
                    }
                    Err(e) => {
                        for id in &request.curseforge_ids {
                            errors.push(format!("CurseForge ID {} not found: {}", id, e));
                        }
                    }
                }
            }

            // 2. Fetch CurseForge-only slugs (from CF URLs)
            for slug in &request.curseforge_only_slugs {
                match modplatforms.curseforge.search(ModSearchParameters {
                    query: ModSearchParametersQuery {
                        game_id: 432,
                        slug: Some(slug.clone()),
                        page_size: Some(1),
                        search_filter: None,
                        game_version: None,
                        category_ids: None,
                        sort_order: None,
                        sort_field: None,
                        class_id: None,
                        mod_loader_types: None,
                        game_version_type_id: None,
                        author_id: None,
                        index: None,
                    }
                }).await {
                    Ok(response) => {
                        if let Some(found) = response.data.into_iter().find(|m| m.slug.eq_ignore_ascii_case(slug)) {
                            if !seen_cf_ids.contains(&found.id) {
                                seen_cf_ids.insert(found.id);
                                results.push(responses::FEUnifiedSearchResult::from(found));
                            }
                        } else {
                            errors.push(format!("CurseForge slug '{}' not found", slug));
                        }
                    }
                    Err(e) => errors.push(format!("CurseForge slug '{}' search error: {}", slug, e))
                }
            }

            // Resolve projects individually so slug aliases and omitted items
            // have an explicit result; versions use their distinct endpoint.
            let mr = resolve_modrinth_batch(request.modrinth_only_ids, request.modrinth_version_ids,
                |id| modplatforms.modrinth.get_project(id.into()),
                |id| modplatforms.modrinth.get_version(carbon_platforms::modrinth::search::VersionID(id))).await;
            for item in mr.results {
                seen_mr_ids.insert(item.id.clone());
                results.push(item);
            }
            errors.extend(mr.errors);

            // 4. Fetch slugs from BOTH platforms
            for slug in &request.slugs {
                let mut found_on_any = false;

                // Try CurseForge slug search
                if let Ok(response) = modplatforms.curseforge.search(ModSearchParameters {
                    query: ModSearchParametersQuery {
                        game_id: 432,
                        slug: Some(slug.clone()),
                        page_size: Some(1),
                        search_filter: None,
                        game_version: None,
                        category_ids: None,
                        sort_order: None,
                        sort_field: None,
                        class_id: None,
                        mod_loader_types: None,
                        game_version_type_id: None,
                        author_id: None,
                        index: None,
                    }
                }).await {
                    if let Some(found) = response.data.into_iter().find(|m| m.slug.eq_ignore_ascii_case(slug)) {
                        found_on_any = true;
                        if !seen_cf_ids.contains(&found.id) {
                            seen_cf_ids.insert(found.id);
                            results.push(responses::FEUnifiedSearchResult::from(found));
                        }
                    }
                }

                // A returned alias is found even when its canonical row already exists.
                if let Ok(project) = modplatforms.modrinth.get_project(slug.clone().into()).await {
                    found_on_any = true;
                    if seen_mr_ids.insert(project.id.clone()) {
                        results.push(responses::FEUnifiedSearchResult::from(project));
                    }
                }

                if !found_on_any {
                    errors.push(format!("Slug '{}' not found on either platform", slug));
                }
            }

            Ok(filters::FEUnifiedBatchResponse { results, errors })
        }

        query GET_UNIFIED_MODLOADERS[app, _args: ()] {
            Ok(FEUnifiedModLoaders(FEUnifiedModLoaderType::iter().collect::<Vec<_>>()))
        }

        query UNIFIED_SEARCH_PROJECT_TYPE[app, _args: ()] {
            Ok(FEUnifiedSearchType::iter().collect::<Vec<_>>())
        }

        query GET_UNIFIED_CATEGORIES[app, _args:()] {
            let modplatforms = app.modplatforms_manager();
            let curseforge_categories = modplatforms.curseforge.get_categories();
            let modrinth_categories = modplatforms.modrinth.get_categories();

            resolve_categories(curseforge_categories, modrinth_categories).await
        }
    }
}
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

// Shared production orchestration boundary; injected futures avoid any provider IO in tests.
async fn resolve_categories(
    curseforge_categories: impl std::future::Future<
        Output = anyhow::Result<
            carbon_platforms::curseforge::CurseForgeResponse<
                Vec<carbon_platforms::curseforge::Category>,
            >,
        >,
    >,
    modrinth_categories: impl std::future::Future<
        Output = anyhow::Result<carbon_platforms::modrinth::responses::CategoriesResponse>,
    >,
) -> anyhow::Result<FEUnifiedCategories> {
    let (cf, mr) = tokio::join!(curseforge_categories, modrinth_categories);
    let (curseforge, curseforge_error) = match cf {
        Ok(response) => (
            response
                .data
                .into_iter()
                .map(|c| (c.id, FEUnifiedCategory::from(c)))
                .collect(),
            None,
        ),
        Err(e) => (Default::default(), Some(category_error(e))),
    };
    let (modrinth, modrinth_error) = match mr {
        Ok(response) => (
            response
                .into_iter()
                .map(|category| {
                    let fe_type: FEUnifiedSearchType = category.project_type.clone().into();
                    let key = format!(
                        "{}:{}",
                        serde_plain::to_string(&fe_type).unwrap_or_default(),
                        category.name
                    );
                    (key, FEUnifiedCategory::from(category))
                })
                .collect(),
            None,
        ),
        Err(e) => (Default::default(), Some(category_error(e))),
    };
    Ok(FEUnifiedCategories {
        curseforge,
        modrinth,
        curseforge_error,
        modrinth_error,
    })
}
fn category_error(error: anyhow::Error) -> String {
    error.to_string().chars().take(400).collect()
}

async fn resolve_modrinth_batch<P, PF, V, VF>(
    projects: Vec<String>,
    versions: Vec<String>,
    mut get_project: P,
    mut get_version: V,
) -> filters::FEUnifiedBatchResponse
where
    P: FnMut(String) -> PF,
    PF: std::future::Future<Output = anyhow::Result<carbon_platforms::modrinth::project::Project>>,
    V: FnMut(String) -> VF,
    VF: std::future::Future<Output = anyhow::Result<carbon_platforms::modrinth::version::Version>>,
{
    let mut response = filters::FEUnifiedBatchResponse {
        results: Vec::new(),
        errors: Vec::new(),
    };
    let mut seen = std::collections::HashSet::new();
    for id in projects {
        match get_project(id.clone()).await {
            Ok(project) => {
                if seen.insert((project.id.clone(), None)) {
                    response.results.push(project.into());
                }
            }
            Err(e) => response
                .errors
                .push(format!("Modrinth project '{id}' not found: {e}")),
        }
    }
    for id in versions {
        let result = async {
            let version = get_version(id.clone()).await?;
            anyhow::ensure!(
                version.id == id,
                "version response does not match requested ID"
            );
            let project = get_project(version.project_id.clone()).await?;
            anyhow::ensure!(
                project.id == version.project_id,
                "version owner does not match returned project"
            );
            Ok::<_, anyhow::Error>(project)
        }
        .await;
        match result {
            Ok(project) => {
                if seen.insert((project.id.clone(), Some(id.clone()))) {
                    let mut row = responses::FEUnifiedSearchResult::from(project);
                    row.requested_version_id = Some(id);
                    response.results.push(row);
                }
            }
            Err(e) => response
                .errors
                .push(format!("Modrinth version '{id}' not found: {e}")),
        }
    }
    response
}

#[cfg(test)]
mod task21_category_tests {
    use super::*;
    #[tokio::test]
    async fn task21_categories_healthy_provider_survives_other_failure() {
        let cf = carbon_platforms::curseforge::CurseForgeResponse {
            data: Vec::new(),
            pagination: None,
        };
        let response = resolve_categories(
            std::future::ready(Ok(cf)),
            std::future::ready(Err(anyhow::anyhow!("Modrinth offline"))),
        )
        .await;
        assert!(
            response.is_ok(),
            "healthy empty dictionary must survive unrelated outage"
        );
        let mr = carbon_platforms::modrinth::responses::CategoriesResponse(Vec::new());
        assert!(
            resolve_categories(
                std::future::ready(Err(anyhow::anyhow!("CurseForge offline"))),
                std::future::ready(Ok(mr))
            )
            .await
            .is_ok()
        );
    }
}

#[cfg(test)]
mod task21_batch_tests {
    use super::*;
    fn project() -> carbon_platforms::modrinth::project::Project {
        serde_json::from_value(serde_json::json!({"slug":"alias","title":"fixture","description":"fixture","categories":[],"client_side":"required","server_side":"optional","body":"fixture","additional_categories":[],"donation_urls":[],"project_type":"modpack","downloads":1,"id":"owner","team":"team","published":"2026-01-01T00:00:00Z","updated":"2026-01-02T00:00:00Z","followers":1,"status":"approved","license":{"id":"MIT","name":"MIT"},"versions":["historical","chosenA","chosenB"],"game_versions":[],"loaders":[]})).unwrap()
    }
    fn version(id: String) -> carbon_platforms::modrinth::version::Version {
        serde_json::from_value(serde_json::json!({"name":"fixture","version_number":"1","dependencies":[],"game_versions":[],"version_type":"release","loaders":[],"featured":true,"id":id,"project_id":"owner","author_id":"author","date_published":"2026-01-01T00:00:00Z","downloads":1,"files":[]})).unwrap()
    }
    #[tokio::test]
    async fn task21_typed_versions_aliases_missing_items_and_separate_selections() {
        let calls = std::sync::Mutex::new(Vec::new());
        let response = resolve_modrinth_batch(
            vec!["alias".into(), "owner".into(), "missing".into()],
            vec!["chosenA".into(), "chosenB".into(), "bad-version".into()],
            |id| {
                calls.lock().unwrap().push(format!("project:{id}"));
                std::future::ready(if id == "missing" {
                    Err(anyhow::anyhow!("missing project"))
                } else {
                    Ok(project())
                })
            },
            |id| {
                calls.lock().unwrap().push(format!("version:{id}"));
                std::future::ready(if id == "bad-version" {
                    Err(anyhow::anyhow!("missing version"))
                } else {
                    Ok(version(id))
                })
            },
        )
        .await;
        assert_eq!(response.results.len(), 3);
        assert_eq!(
            response
                .results
                .iter()
                .map(|r| r.requested_version_id.as_deref())
                .collect::<Vec<_>>(),
            vec![None, Some("chosenA"), Some("chosenB")]
        );
        assert!(response.results.iter().all(|r| r.id == "owner"));
        assert_eq!(response.errors.len(), 2);
        assert!(response.errors.iter().any(|e| e.contains("missing")));
        assert!(response.errors.iter().any(|e| e.contains("bad-version")));
        assert!(
            !calls
                .lock()
                .unwrap()
                .iter()
                .any(|c| c == "project:chosenA" || c == "project:chosenB")
        );
    }
    #[tokio::test]
    async fn task21_version_owner_mismatch_or_missing_owner_is_explicit() {
        let result = resolve_modrinth_batch(
            Vec::new(),
            vec!["chosen".into()],
            |_| std::future::ready(Err(anyhow::anyhow!("missing owner"))),
            |id| std::future::ready(Ok(version(id))),
        )
        .await;
        assert!(result.results.is_empty());
        assert!(result.errors[0].contains("chosen"));
        let result = resolve_modrinth_batch(
            Vec::new(),
            vec!["chosen".into()],
            |_| {
                let mut p = project();
                p.id = "foreign".into();
                std::future::ready(Ok(p))
            },
            |id| std::future::ready(Ok(version(id))),
        )
        .await;
        assert!(result.results.is_empty());
        assert!(result.errors[0].contains("owner"));
    }
    #[tokio::test]
    async fn task21_category_maps_and_independent_errors_cover_all_provider_outcomes() {
        for cf_ok in [false, true] {
            for mr_ok in [false, true] {
                let cf: carbon_platforms::curseforge::Category=serde_json::from_value(serde_json::json!({"id":1,"name":"healthy CF","slug":"healthy","url":"synthetic","dateModified":"fixture","classId":4471})).unwrap();
                let mr = carbon_platforms::modrinth::tag::Category {
                    icon: None,
                    name: "healthy MR".into(),
                    project_type: ProjectType::Modpack,
                    header: "fixture".into(),
                };
                let response = resolve_categories(
                    std::future::ready(if cf_ok {
                        Ok(carbon_platforms::curseforge::CurseForgeResponse {
                            data: vec![cf],
                            pagination: None,
                        })
                    } else {
                        Err(anyhow::anyhow!("CF offline"))
                    }),
                    std::future::ready(if mr_ok {
                        Ok(carbon_platforms::modrinth::responses::CategoriesResponse(
                            vec![mr],
                        ))
                    } else {
                        Err(anyhow::anyhow!("MR offline"))
                    }),
                )
                .await
                .unwrap();
                assert_eq!(response.curseforge.len(), usize::from(cf_ok));
                assert_eq!(response.modrinth.len(), usize::from(mr_ok));
                assert_eq!(response.curseforge_error.is_some(), !cf_ok);
                assert_eq!(response.modrinth_error.is_some(), !mr_ok);
                if mr_ok {
                    assert!(response.modrinth.contains_key("modpack:healthy MR"));
                }
            }
        }
    }
}
