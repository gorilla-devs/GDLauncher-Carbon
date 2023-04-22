use std::path::PathBuf;

use regex::Regex;
use thiserror::Error;

pub struct MavenCoordinates {
    group_id: String,
    artifact_id: String,
    version: String,
    identifier: Option<String>,
    additional: Option<String>,
    extension: String,
}

impl MavenCoordinates {
    /// Needs to be in the format of `group:artifact:version`
    pub fn try_from(coordinates: String, additional: Option<String>) -> Result<Self, MavenError> {
        let coordinates = coordinates.trim();
        if coordinates.is_empty() || !is_maven_coordinates(coordinates) {
            return Err(MavenError::InvalidCoordinates);
        }

        parse_maven_coordinates(coordinates, additional)
    }

    pub fn into_path(self) -> PathBuf {
        let mut path = PathBuf::new();

        for part in self.group_id.split('.') {
            path = path.join(part);
        }

        path = path.join(&self.artifact_id);
        path = path.join(&self.version);

        let version = format!("-{}", self.version);

        let identifier = self.identifier.map(|a| format!("-{a}")).unwrap_or_default();

        let additional = self.additional.map(|a| format!("-{a}")).unwrap_or_default();

        path = path.join(format!(
            "{}{}{}{}.{}",
            self.artifact_id, version, identifier, additional, self.extension
        ));

        path
    }
}

#[derive(Error, Debug)]
pub enum MavenError {
    #[error("invalid maven coordinates")]
    InvalidCoordinates,
}

/// Needs to be in the format of `group:artifact:version@extension`
/// This is not the full maven specification but should be enough for our use case
fn is_maven_coordinates(maven_coordinates: &str) -> bool {
    Regex::new(r#"^[a-zA-Z0-9._-]+:[a-zA-Z0-9._-]+:[0-9]+\.[0-9]+(\.[0-9]+)?(-[a-zA-Z0-9._-]+)*(\.[a-zA-Z0-9._-]+)*(:[a-zA-Z0-9._-]+)?(@[a-zA-Z0-9._-]+)?$"#)
        .expect("failed to compile maven coordinates regex!!!")
        .is_match(maven_coordinates)
}

fn parse_maven_coordinates(
    maven_coordinates: &str,
    additional: Option<String>,
) -> Result<MavenCoordinates, MavenError> {
    let mut split = maven_coordinates.split('@');
    let maven_coordinates = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let extension = split.next();

    let mut split = maven_coordinates.split(':');
    let group_id = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let artifact_id = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let version = split.next().ok_or(MavenError::InvalidCoordinates)?;
    let identifier = split.next().map(|a| a.to_string());

    Ok(MavenCoordinates {
        group_id: group_id.to_string(),
        artifact_id: artifact_id.to_string(),
        version: version.to_string(),
        identifier,
        additional,
        extension: extension.unwrap_or("jar").to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_coordinates() {
        assert!(is_maven_coordinates("com.example:example:1.0.0"));
        assert!(is_maven_coordinates("com.example:example:1.0.0:identifier"));
        assert!(is_maven_coordinates("com.example:example:1.0"));
        assert!(is_maven_coordinates(
            "com.example:example:1.0:identifier@zip"
        ));
        assert!(is_maven_coordinates(
            "com.example:example-something:1.0.final"
        ));
        assert!(is_maven_coordinates(
            "com.example:example-something:1.0.0.Final-beta.1"
        ));
        assert!(is_maven_coordinates(
            "com.example.example:example-example:1.0.0"
        ));
        assert!(is_maven_coordinates(
            "com.example.example:example-example:1.0.0.0"
        ));
        assert!(is_maven_coordinates(
            "com.example.example:example-example:1.0.0.0.0.0.0" // Do we want this?
        ));
        assert!(is_maven_coordinates(
            "com.example.example:example-example:1.0.0-SNAPSHOT"
        ));
        assert!(is_maven_coordinates(
            "com.example.example:example-example:1.0.0-beta.1"
        ));
    }

    #[test]
    fn test_invalid_coordinates() {
        assert!(!is_maven_coordinates(""));
        assert!(!is_maven_coordinates("com.example:example:1"));
        assert!(!is_maven_coordinates("com.example:example"));
        assert!(!is_maven_coordinates(
            "com.example:example:not_a_version:extra"
        ));
        assert!(!is_maven_coordinates("@com.example:example:1.0.0"));
        assert!(!is_maven_coordinates("com.example:example:1.0.0:@"));
        assert!(!is_maven_coordinates("com.example@:example:1.0.0"));
        assert!(!is_maven_coordinates("com.example:example:1.0.0@"));
        assert!(!is_maven_coordinates("justsometext"));
    }

    #[test]
    fn test_parse_coordinates() {
        let coordinates = "com.example:example:1.0.0".to_string();
        let parsed_coordinates = parse_maven_coordinates(&coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example");
        assert_eq!(parsed_coordinates.artifact_id, "example");
        assert_eq!(parsed_coordinates.version, "1.0.0");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(parsed_coordinates.extension, "jar");

        let coordinates = "com.example.example:example-example:1.0.0-SNAPSHOT".to_string();
        let parsed_coordinates = parse_maven_coordinates(&coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example.example");
        assert_eq!(parsed_coordinates.artifact_id, "example-example");
        assert_eq!(parsed_coordinates.version, "1.0.0-SNAPSHOT");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(parsed_coordinates.extension, "jar");

        let coordinates = "com.example.example:example-example:1.0.0-SNAPSHOT@zip".to_string();
        let parsed_coordinates = parse_maven_coordinates(&coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example.example");
        assert_eq!(parsed_coordinates.artifact_id, "example-example");
        assert_eq!(parsed_coordinates.version, "1.0.0-SNAPSHOT");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(parsed_coordinates.extension, "zip");

        let coordinates =
            "com.example.example:example-example:1.0.0-SNAPSHOT:identifier".to_string();
        let parsed_coordinates = parse_maven_coordinates(&coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example.example");
        assert_eq!(parsed_coordinates.artifact_id, "example-example");
        assert_eq!(parsed_coordinates.version, "1.0.0-SNAPSHOT");
        assert_eq!(
            parsed_coordinates.identifier,
            Some("identifier".to_string())
        );
        assert_eq!(parsed_coordinates.extension, "jar");

        let coordinates =
            "com.example.example:example-example:1.0.0-SNAPSHOT:identifier@zip".to_string();
        let parsed_coordinates = parse_maven_coordinates(&coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example.example");
        assert_eq!(parsed_coordinates.artifact_id, "example-example");
        assert_eq!(parsed_coordinates.version, "1.0.0-SNAPSHOT");
        assert_eq!(
            parsed_coordinates.identifier,
            Some("identifier".to_string())
        );
        assert_eq!(parsed_coordinates.extension, "zip");
    }

    #[test]
    fn test_try_from() {
        let coordinates = "com.example:example:1.0.0".to_string();
        let parsed_coordinates = MavenCoordinates::try_from(coordinates, None).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example");
        assert_eq!(parsed_coordinates.artifact_id, "example");
        assert_eq!(parsed_coordinates.version, "1.0.0");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(parsed_coordinates.additional, None);

        let coordinates = "com.example:example:1.0.0".to_string();
        let parsed_coordinates =
            MavenCoordinates::try_from(coordinates, Some("natives-something".to_string())).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example");
        assert_eq!(parsed_coordinates.artifact_id, "example");
        assert_eq!(parsed_coordinates.version, "1.0.0");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(
            parsed_coordinates.additional,
            Some("natives-something".to_string())
        );

        let coordinates = "com.example:example:1.0.0@zip".to_string();
        let parsed_coordinates =
            MavenCoordinates::try_from(coordinates, Some("natives-something".to_string())).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example");
        assert_eq!(parsed_coordinates.artifact_id, "example");
        assert_eq!(parsed_coordinates.version, "1.0.0");
        assert_eq!(parsed_coordinates.identifier, None);
        assert_eq!(
            parsed_coordinates.additional,
            Some("natives-something".to_string())
        );
        assert_eq!(parsed_coordinates.extension, "zip");

        let coordinates = "com.example:example:1.0.0:identifier@zip".to_string();
        let parsed_coordinates =
            MavenCoordinates::try_from(coordinates, Some("natives-something".to_string())).unwrap();
        assert_eq!(parsed_coordinates.group_id, "com.example");
        assert_eq!(parsed_coordinates.artifact_id, "example");
        assert_eq!(parsed_coordinates.version, "1.0.0");
        assert_eq!(parsed_coordinates.identifier, Some("identifier".to_owned()));
        assert_eq!(
            parsed_coordinates.additional,
            Some("natives-something".to_string())
        );
        assert_eq!(parsed_coordinates.extension, "zip");

        let coordinates = "".to_string();
        assert!(MavenCoordinates::try_from(coordinates, None).is_err());

        let coordinates = "justsometext".to_string();
        assert!(MavenCoordinates::try_from(coordinates, None).is_err());
    }

    #[test]
    fn test_into_path() {
        let coordinates = "com.example:example:1.0.0".to_string();
        let parsed_coordinates = MavenCoordinates::try_from(coordinates, None).unwrap();
        let path = parsed_coordinates.into_path();
        assert_eq!(
            path,
            PathBuf::from("com")
                .join("example")
                .join("example")
                .join("1.0.0")
                .join("example-1.0.0.jar")
        );

        let coordinates = "com.example:example-mc:1.0.0".to_string();
        let parsed_coordinates =
            MavenCoordinates::try_from(coordinates, Some("natives-example".to_string())).unwrap();
        let path = parsed_coordinates.into_path();
        assert_eq!(
            path,
            PathBuf::from("com")
                .join("example")
                .join("example-mc")
                .join("1.0.0")
                .join("example-mc-1.0.0-natives-example.jar")
        );

        let coordinates = "com.example:example:1.0.0@zip".to_string();
        let parsed_coordinates = MavenCoordinates::try_from(coordinates, None).unwrap();
        let path = parsed_coordinates.into_path();
        assert_eq!(
            path,
            PathBuf::from("com")
                .join("example")
                .join("example")
                .join("1.0.0")
                .join("example-1.0.0.zip")
        );

        let coordinates = "com.example:example:1.0.0:identifier@zip".to_string();
        let parsed_coordinates =
            MavenCoordinates::try_from(coordinates, Some("additional".to_string())).unwrap();
        let path = parsed_coordinates.into_path();
        assert_eq!(
            path,
            PathBuf::from("com")
                .join("example")
                .join("example")
                .join("1.0.0")
                .join("example-1.0.0-identifier-additional.zip")
        );
    }
}
