use std::{collections::HashMap, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::{project::Project, tag::Category, version::Version};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CategoriesResponse(pub Vec<Category>);

impl Deref for CategoriesResponse {
    type Target = Vec<Category>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for CategoriesResponse {
    type Item = Category;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Category> for CategoriesResponse {
    fn from_iter<I: IntoIterator<Item = Category>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        CategoriesResponse(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ProjectsResponse(pub Vec<Project>);

impl Deref for ProjectsResponse {
    type Target = Vec<Project>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for ProjectsResponse {
    type Item = Project;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Project> for ProjectsResponse {
    fn from_iter<I: IntoIterator<Item = Project>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        ProjectsResponse(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionsResponse(pub Vec<Version>);

impl Deref for VersionsResponse {
    type Target = Vec<Version>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for VersionsResponse {
    type Item = Version;
    type IntoIter = std::vec::IntoIter<Self::Item>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<Version> for VersionsResponse {
    fn from_iter<I: IntoIterator<Item = Version>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = Vec::with_capacity(size_lower);
        for i in iter {
            c.push(i);
        }
        VersionsResponse(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VersionHashesResponse(pub HashMap<String, Version>);

impl Deref for VersionHashesResponse {
    type Target = HashMap<String, Version>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for VersionHashesResponse {
    type Item = (String, Version);
    type IntoIter = std::collections::hash_map::IntoIter<String, Version>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl FromIterator<(String, Version)> for VersionHashesResponse {
    fn from_iter<I: IntoIterator<Item = (String, Version)>>(iter: I) -> Self {
        let iter = iter.into_iter();
        let (size_lower, _) = iter.size_hint();
        let mut c = HashMap::with_capacity(size_lower);
        for (hash, version) in iter {
            c.insert(hash, version);
        }
        VersionHashesResponse(c)
    }
}
