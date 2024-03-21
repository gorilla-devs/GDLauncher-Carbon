use std::{
    collections::HashMap,
    convert::{Into, TryInto},
    ops::Deref,
};

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::domain::modplatforms::modrinth::{
    responses::{
        CategoriesResponse, LoadersResponse, ProjectsResponse, TeamResponse, VersionHashesResponse,
        VersionsResponse,
    },
    search::ProjectSearchResponse,
};

use crate::api::modplatforms::modrinth::structs::{
    MRFECategory, MRFEProject, MRFEProjectSearchResult, MRFEVersion,
};

use super::structs::{MRFELoader, MRFETeamMember};

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectSearchResponse {
    /// The List of Results
    pub hits: Vec<MRFEProjectSearchResult>,
    /// The number of results that were skipped by the query
    pub offset: u32,
    /// the number of results that were returned by the query
    pub limit: u32,
    /// the total number of results that match the query
    pub total_hits: u32,
}

impl From<ProjectSearchResponse> for MRFEProjectSearchResponse {
    fn from(results: ProjectSearchResponse) -> Self {
        MRFEProjectSearchResponse {
            hits: results.hits.into_iter().map(Into::into).collect(),
            offset: results.offset,
            limit: results.limit,
            total_hits: results.total_hits,
        }
    }
}

impl TryFrom<MRFEProjectSearchResponse> for ProjectSearchResponse {
    type Error = anyhow::Error;

    fn try_from(results: MRFEProjectSearchResponse) -> Result<Self, Self::Error> {
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
pub struct MRFECategoriesResponse(Vec<MRFECategory>);

impl Deref for MRFECategoriesResponse {
    type Target = Vec<MRFECategory>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFECategoriesResponse {
    type Item = MRFECategory;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFECategory> for MRFECategoriesResponse {
    fn from_iter<I: IntoIterator<Item = MRFECategory>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFECategoriesResponse(c)
    }
}

impl From<CategoriesResponse> for MRFECategoriesResponse {
    fn from(value: CategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl From<MRFECategoriesResponse> for CategoriesResponse {
    fn from(value: MRFECategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFELoadersResponse(pub Vec<MRFELoader>);

impl Deref for MRFELoadersResponse {
    type Target = Vec<MRFELoader>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFELoadersResponse {
    type Item = MRFELoader;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFELoader> for MRFELoadersResponse {
    fn from_iter<I: IntoIterator<Item = MRFELoader>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFELoadersResponse(c)
    }
}

impl From<LoadersResponse> for MRFELoadersResponse {
    fn from(value: LoadersResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl From<MRFELoadersResponse> for LoadersResponse {
    fn from(value: MRFELoadersResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEProjectsResponse(pub Vec<MRFEProject>);

impl Deref for MRFEProjectsResponse {
    type Target = Vec<MRFEProject>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFEProjectsResponse {
    type Item = MRFEProject;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFEProject> for MRFEProjectsResponse {
    fn from_iter<I: IntoIterator<Item = MRFEProject>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFEProjectsResponse(c)
    }
}

impl From<ProjectsResponse> for MRFEProjectsResponse {
    fn from(value: ProjectsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<MRFEProjectsResponse> for ProjectsResponse {
    type Error = anyhow::Error;
    fn try_from(value: MRFEProjectsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersionsResponse(pub Vec<MRFEVersion>);

impl Deref for MRFEVersionsResponse {
    type Target = Vec<MRFEVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFEVersionsResponse {
    type Item = MRFEVersion;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFEVersion> for MRFEVersionsResponse {
    fn from_iter<I: IntoIterator<Item = MRFEVersion>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFEVersionsResponse(c)
    }
}

impl From<VersionsResponse> for MRFEVersionsResponse {
    fn from(value: VersionsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<MRFEVersionsResponse> for VersionsResponse {
    type Error = anyhow::Error;
    fn try_from(value: MRFEVersionsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct MRFEVersionHashesResponse(pub HashMap<String, MRFEVersion>);

impl Deref for MRFEVersionHashesResponse {
    type Target = HashMap<String, MRFEVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFEVersionHashesResponse {
    type Item = (String, MRFEVersion);
    type IntoIter = std::collections::hash_map::IntoIter<String, MRFEVersion>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, MRFEVersion)> for MRFEVersionHashesResponse {
    fn from_iter<I: IntoIterator<Item = (String, MRFEVersion)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = HashMap::with_capacity(size_lower);
        for (hash, version) in iter {
            c.insert(hash, version);
        }
        MRFEVersionHashesResponse(c)
    }
}

impl From<VersionHashesResponse> for MRFEVersionHashesResponse {
    fn from(value: VersionHashesResponse) -> Self {
        value
            .into_iter()
            .map(|(key, version)| (key, version.into()))
            .collect()
    }
}

impl TryFrom<MRFEVersionHashesResponse> for VersionHashesResponse {
    type Error = anyhow::Error;
    fn try_from(value: MRFEVersionHashesResponse) -> Result<Self, Self::Error> {
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
pub struct MRFETeamResponse(pub Vec<MRFETeamMember>);
impl Deref for MRFETeamResponse {
    type Target = Vec<MRFETeamMember>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for MRFETeamResponse {
    type Item = MRFETeamMember;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<MRFETeamMember> for MRFETeamResponse {
    fn from_iter<I: IntoIterator<Item = MRFETeamMember>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        MRFETeamResponse(c)
    }
}

impl From<TeamResponse> for MRFETeamResponse {
    fn from(value: TeamResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}
