use std::convert::TryInto;

use rspc::Type;
use serde::{Deserialize, Serialize};

use crate::domain::modplatforms::modrinth::search::{ProjectSearchResponse, ProjectSearchResult};

use super::structs::FEProjectSearchResult;

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
            hits: results
                .hits
                .into_iter()
                .map(|result| result.into())
                .collect(),
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
                .map(|result| result.try_into())
                .collect::<Result<Vec<_>, _>>()?,
            offset: results.offset,
            limit: results.limit,
            total_hits: results.total_hits,
        })
    }
}
