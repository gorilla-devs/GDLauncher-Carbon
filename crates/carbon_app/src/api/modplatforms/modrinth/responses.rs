use std::{
    collections::HashMap,
    convert::{Into, TryInto},
    ops::Deref,
};

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{
    responses::{CategoriesResponse, ProjectsResponse, VersionHashesResponse, VersionsResponse},
    search::ProjectSearchResponse,
};

use crate::api::modplatforms::modrinth::structs::{
    FECategory, FEProject, FEProjectSearchResult, FEVersion,
};

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEProjectSearchResponse {
    /// The List of Results
    pub hits: Vec<FEProjectSearchResult>,
    /// The number of results that were skipped by the query
    pub offset: u32,
    /// the number of results that were returned by the query
    pub limit: u32,
    /// the total number of results that match the query
    pub total_hits: u32,
}

impl From<ProjectSearchResponse> for FEProjectSearchResponse {
    fn from(results: ProjectSearchResponse) -> Self {
        FEProjectSearchResponse {
            hits: results.hits.into_iter().map(Into::into).collect(),
            offset: results.offset,
            limit: results.limit,
            total_hits: results.total_hits,
        }
    }
}

impl TryFrom<FEProjectSearchResponse> for ProjectSearchResponse {
    type Error = anyhow::Error;

    fn try_from(results: FEProjectSearchResponse) -> Result<Self, Self::Error> {
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
pub struct FECategoriesResponse(Vec<FECategory>);

impl Deref for FECategoriesResponse {
    type Target = Vec<FECategory>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FECategoriesResponse {
    type Item = FECategory;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FECategory> for FECategoriesResponse {
    fn from_iter<I: IntoIterator<Item = FECategory>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FECategoriesResponse(c)
    }
}

impl From<CategoriesResponse> for FECategoriesResponse {
    fn from(value: CategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl From<FECategoriesResponse> for CategoriesResponse {
    fn from(value: FECategoriesResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEProjectsResponse(pub Vec<FEProject>);

impl Deref for FEProjectsResponse {
    type Target = Vec<FEProject>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEProjectsResponse {
    type Item = FEProject;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEProject> for FEProjectsResponse {
    fn from_iter<I: IntoIterator<Item = FEProject>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEProjectsResponse(c)
    }
}

impl From<ProjectsResponse> for FEProjectsResponse {
    fn from(value: ProjectsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<FEProjectsResponse> for ProjectsResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEProjectsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEVersionsResponse(pub Vec<FEVersion>);

impl Deref for FEVersionsResponse {
    type Target = Vec<FEVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEVersionsResponse {
    type Item = FEVersion;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<FEVersion> for FEVersionsResponse {
    fn from_iter<I: IntoIterator<Item = FEVersion>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        FEVersionsResponse(c)
    }
}

impl From<VersionsResponse> for FEVersionsResponse {
    fn from(value: VersionsResponse) -> Self {
        value.into_iter().map(Into::into).collect()
    }
}

impl TryFrom<FEVersionsResponse> for VersionsResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEVersionsResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<_, _>>()
    }
}

#[derive(Type, Deserialize, Serialize, Debug, Clone)]
pub struct FEVersionHashesResponse(pub HashMap<String, FEVersion>);

impl Deref for FEVersionHashesResponse {
    type Target = HashMap<String, FEVersion>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for FEVersionHashesResponse {
    type Item = (String, FEVersion);
    type IntoIter = std::collections::hash_map::IntoIter<String, FEVersion>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, FEVersion)> for FEVersionHashesResponse {
    fn from_iter<I: IntoIterator<Item = (String, FEVersion)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = HashMap::with_capacity(size_lower);
        for (hash, version) in iter {
            c.insert(hash, version);
        }
        FEVersionHashesResponse(c)
    }
}

impl From<VersionHashesResponse> for FEVersionHashesResponse {
    fn from(value: VersionHashesResponse) -> Self {
        value
            .into_iter()
            .map(|(key, version)| (key, version.into()))
            .collect()
    }
}

impl TryFrom<FEVersionHashesResponse> for VersionHashesResponse {
    type Error = anyhow::Error;
    fn try_from(value: FEVersionHashesResponse) -> Result<Self, Self::Error> {
        value
            .into_iter()
            .map(|(key, version)| match version.try_into() {
                Ok(version) => Ok((key, version)),
                Err(err) => Err(err),
            })
            .collect::<Result<_, _>>()
    }
}
