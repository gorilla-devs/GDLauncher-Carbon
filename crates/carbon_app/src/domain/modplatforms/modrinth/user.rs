//! Models related to users
//!
//! [documentation](https://docs.modrinth.com/api-spec/#tag/user_model)

use super::*;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct User {
    pub username: ArcStr,
    /// The user's display name
    pub name: Option<ArcStr>,
    /// The user's email, only visible to the user itself when authenticated
    pub email: Option<ArcStr>,
    /// A description of the user
    pub bio: Option<ArcStr>,
    /// Various data relating to the user's payouts status,
    /// only visible to the user itself when authenticated
    pub payout_data: Option<PayoutData>,
    pub id: ArcStr,
    /// The user's GitHub ID
    pub github_id: Option<Number>,
    pub avatar_url: Url,
    pub created: UtcDateTime,
    pub role: UserRole,
    /// Any badges applicable to this user.
    /// These are currently unused and not displayed, and as such are subject to change.
    ///
    /// [documentation](https://docs.modrinth.com/api-spec/#tag/user_model)
    pub badges: Number,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PayoutData {
    balance: ArcStr,
    payout_wallet: Option<PayoutWallet>,
    payout_wallet_type: Option<PayoutWalletType>,
    payout_address: Option<ArcStr>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TeamMember {
    /// The ID of the member's team
    pub team_id: ArcStr,
    pub user: User,
    pub role: ArcStr,
    /// The user's permissions in bitflag format
    /// (requires authorisation to view)
    ///
    /// In order from first to tenth bit, they indicate the ability to:
    /// - UPLOAD_VERSION
    /// - DELETE_VERSION
    /// - EDIT_DETAILS
    /// - EDIT_BODY
    /// - MANAGE_INVITES
    /// - REMOVE_MEMBER
    /// - EDIT_MEMBER
    /// - DELETE_PROJECT
    /// - VIEW_ANALYTICS
    /// - VIEW_PAYOUTS
    pub permissions: Option<Number>,
    /// Whether the user has accepted membership of the team
    /// (requires authorisation to view)
    pub accepted: bool,
    /// The split of payouts going to this user.
    /// The proportion of payouts they get is their split divided by the sum of the splits of all members.
    pub payouts_split: Option<Number>,
    pub ordering: Option<Number>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Notification {
    pub id: ArcStr,
    /// The ID of the user who received the notification
    pub user_id: ArcStr,
    #[serde(rename = "type")]
    pub notification_type: Option<NotificationType>,
    pub title: ArcStr,
    pub text: String,
    /// A _relative_ link to the related project/version
    pub link: ArcStr,
    pub read: bool,
    pub created: UtcDateTime,
    /// A list of actions that can be performed
    pub actions: Vec<NotificationAction>,
}

// Undocumented struct pulled from the labrinth source code
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NotificationAction {
    pub title: ArcStr,
    /// The route to call when this notification action is called.
    /// Contains the HTTP method and route respectively.
    pub action_route: (ArcStr, ArcStr),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PayoutHistory {
    pub all_time: ArcStr,
    /// The amount made by the user in the previous 30 days
    pub last_month: ArcStr,
    pub payouts: Vec<Payout>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Payout {
    pub created: UtcDateTime,
    pub amount: Number,
    pub status: ArcStr,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PayoutWallet {
    PayPal,
    Venmo,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayoutWalletType {
    Email,
    Phone,
    UserHandle,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    ProjectUpdate,
    TeamInvite,
    StatusUpdate,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum UserRole {
    Developer,
    Moderator,
    Admin,
}
