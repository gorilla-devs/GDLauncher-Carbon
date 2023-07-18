use std::{
    collections::HashMap,
    convert::{Into, TryInto},
    ops::Deref,
};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{
    responses::{
        CategoriesResponse, ProjectsResponse, TeamResponse, VersionHashesResponse, VersionsResponse, LoadersResponse,
    },
    search::ProjectSearchResponse,
};

use crate::api::modplatforms::modrinth::structs::{
    FEModrinthCategory, FEModrinthProject, FEModrinthProjectSearchResult, FEModrinthVersion,
};

use super::structs::{FEModrinthTeamMember, FEModrinthLoader};

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectSearchResponse {
    /// The List of Results
    pub hits: Vec<FEModrinthProjectSearchResult>,
    /// The number of results that were skipped by the query
    pub offset: u32,
    /// the number of results that were returned by the query
    pub limit: u32,
    /// the total number of results that match the query
    pub total_hits: u32,
}

impl From<ProjectSearchResponse> for FEModrinthProjectSearchResponse {
    fn from(results: ProjectSearchResponse) -> Self {
        FEModrinthProjectSearchResponse {
            hits: results.hits.into_iter().map(Into::into).collect(),
            offset: results.offset,
            limit: results.limit,
            total_hits: results.total_hits,
        }
    }
}

impl TryFrom<FEModrinthProjectSearchResponse> for ProjectSearchResponse {
    type Error = anyhow::Error;

    fn try_from(results: FEModrinthProjectSearchResponse) -> Result<Self, Self::Error> {
        Ok(ProjectSearchResponse {
            hits: results
                .hits
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>, _>>()?,
            offset: results.offset,
            limit: results.limit,
            total_hits: results.total_hits,
        })
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthCategoriesResponse(Vec<FEModrinthCategory>);

impl Deref for FEModrinthCategoriesResponse {
    type Target = Vec<FEModrinthCategory>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthCategoriesResponse {
    type Item = FEModrinthCategory;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthCategory> for FEModrinthCategoriesResponse {
    fn from_iter<I: IntoIterator<Item = FEModrinthCategory>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthCategoriesResponse(c)
    }
}

impl From<CategoriesResponse> for FEModrinthCategoriesResponse {
    fn from(value: CategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl From<FEModrinthCategoriesResponse> for CategoriesResponse {
    fn from(value: FEModrinthCategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthLoadersResponse(pub Vec<FEModrinthLoader>);

impl Deref for FEModrinthLoadersResponse {
    type Target = Vec<FEModrinthLoader>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthLoadersResponse {
    type Item = FEModrinthLoader;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthLoader> for FEModrinthLoadersResponse {
    fn from_iter<I: IntoIterator<Item = FEModrinthLoader>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthLoadersResponse(c)
    }
}

impl From<LoadersResponse> for FEModrinthLoadersResponse {
    fn from(value: LoadersResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl From<FEModrinthLoadersResponse> for LoadersResponse {
    fn from(value: FEModrinthLoadersResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthProjectsResponse(pub Vec<FEModrinthProject>);

impl Deref for FEModrinthProjectsResponse {
    type Target = Vec<FEModrinthProject>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthProjectsResponse {
    type Item = FEModrinthProject;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthProject> for FEModrinthProjectsResponse {
    fn from_iter<I: IntoIterator<Item = FEModrinthProject>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthProjectsResponse(c)
    }
}

impl From<ProjectsResponse> for FEModrinthProjectsResponse {
    fn from(value: ProjectsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<FEModrinthProjectsResponse> for ProjectsResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthProjectsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthVersionsResponse(pub Vec<FEModrinthVersion>);

impl Deref for FEModrinthVersionsResponse {
    type Target = Vec<FEModrinthVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthVersionsResponse {
    type Item = FEModrinthVersion;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthVersion> for FEModrinthVersionsResponse {
    fn from_iter<I: IntoIterator<Item = FEModrinthVersion>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthVersionsResponse(c)
    }
}

impl From<VersionsResponse> for FEModrinthVersionsResponse {
    fn from(value: VersionsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<FEModrinthVersionsResponse> for VersionsResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthVersionsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthVersionHashesResponse(pub HashMap<String, FEModrinthVersion>);

impl Deref for FEModrinthVersionHashesResponse {
    type Target = HashMap<String, FEModrinthVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthVersionHashesResponse {
    type Item = (String, FEModrinthVersion);
    type IntoIter = std::collections::hash_map::IntoIter<String, FEModrinthVersion>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, FEModrinthVersion)> for FEModrinthVersionHashesResponse {
    fn from_iter<I: IntoIterator<Item = (String, FEModrinthVersion)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = HashMap::with_capacity(size_lower);
        for (hash, version) in iter {
            c.insert(hash, version);
        }
        FEModrinthVersionHashesResponse(c)
    }
}

impl From<VersionHashesResponse> for FEModrinthVersionHashesResponse {
    fn from(value: VersionHashesResponse) -> Self {
        value
            .into_iter()
            .map(|(key, version)| (key, version.into()))
            .collect()
    }
}

impl TryFrom<FEModrinthVersionHashesResponse> for VersionHashesResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEModrinthVersionHashesResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(|(key, version)| match version.try_into() {
                Ok(version) => Ok((key, version)),
                Err(err) => Err(err),
            })
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEModrinthTeamResponse(pub Vec<FEModrinthTeamMember>);
impl Deref for FEModrinthTeamResponse {
    type Target = Vec<FEModrinthTeamMember>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEModrinthTeamResponse {
    type Item = FEModrinthTeamMember;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEModrinthTeamMember> for FEModrinthTeamResponse {
    fn from_iter<I: IntoIterator<Item = FEModrinthTeamMember>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEModrinthTeamResponse(c)
    }
}

impl From<TeamResponse> for FEModrinthTeamResponse {
    fn from(value: TeamResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}
