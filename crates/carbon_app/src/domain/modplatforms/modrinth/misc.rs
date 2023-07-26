//! Models related to miscellaneous API calls

use super::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Report {
    pub report_type: String,
    /// The ID of the item being reported
    pub item_id: String,
    /// The type of item being reported
    pub item_type: ReportItemType,
    /// An extended explanation of the report
    pub body: String,
    /// The ID of the user who submitted the report
    pub reporter: String,
    pub created: UtcDateTime,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ReportSubmission {
    pub report_type: String,
    /// The ID of the item being reported
    pub item_id: String,
    /// The type of item being reported
    pub item_type: ReportItemType,
    /// An extended explanation of the report
    pub body: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub enum ReportItemType {
    Project,
    User,
    Version,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Statistics {
    /// The number of project on Modrinth
    pub projects: u32,
    /// The number of versions on Modrinth
    pub versions: u32,
    /// The number of version files on Modrinth
    pub files: u32,
    /// The number of authors (users with projects) on Modrinth
    pub authors: u32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Welcome {
    pub about: String,
    pub documentation: Url,
    pub name: String,
    pub version: String,
}
