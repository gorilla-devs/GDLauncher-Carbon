//! Models related to users
//!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/user_model)

use super::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub username: String,
    /// The user's display name
    pub name: Option<String>,
    pub id: String,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TeamMember {
    /// The ID of the member's team
    pub team_id: String,
    pub user: User,
    pub role: String,
    /// Whether the user has accepted membership of the team
    /// (requires authorisation to view)
    pub accepted: bool,
    pub ordering: Option<i64>,
}
