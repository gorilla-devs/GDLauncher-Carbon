use std::path::PathBuf;

use regex::Regex;
use thiserror::Error;

pub struct MavenCoordinates {
    group_id: String,
    artifact_id: String,
    version: String,
}

#[derive(Error, Debug)]
pub enum MavenError {
    #[error("invalid maven coordinates")]
    InvalidCoordinates,
}

/// Needs to be in the format of `group:artifact:version`
/// This is not the full maven specification but should be enough for our use case
fn is_maven_coordinates(maven_coordinates: &str) -> bool {
    Regex::new(r#"^[a-zA-Z0-9._-]+:[a-zA-Z0-9._-]+:[0-9]+\.[0-9]+\.[0-9]+$"#)
        .expect("failed to compile maven coordinates regex!!!")
        .is_match(maven_coordinates)
}

fn parse_maven_coordinates(maven_coordinates: &str) -> Result<MavenCoordinates, MavenError> {
    let mut split = maven_coordinates.split(':');
    let group_id = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let artifact_id = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let version = split.next().ok_or(MavenError::InvalidCoordinates)?;

    Ok(MavenCoordinates {
        group_id: group_id.to_string(),
        artifact_id: artifact_id.to_string(),
        version: version.to_string(),
    })
}

impl MavenCoordinates {
    /// Needs to be in the format of `group:artifact:version`
    pub fn try_from(coordinates: String) -> Result<Self, MavenError> {
        let coordinates = coordinates.trim();
        if coordinates.is_empty() || !is_maven_coordinates(coordinates) {
            return Err(MavenError::InvalidCoordinates);
        }

        parse_maven_coordinates(coordinates)
    }

    pub fn into_pathbuf(self) -> PathBuf {
        PathBuf::new()
            .join(self.group_id)
            .join(self.artifact_id)
            .join(self.version)
    }
}
