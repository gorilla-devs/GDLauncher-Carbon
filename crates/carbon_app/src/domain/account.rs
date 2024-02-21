use chrono::{DateTime, Utc};

#[derive(Debug)]
pub struct Account {
    pub username: String,
    pub uuid: String,
    pub last_used: DateTime<Utc>,
    pub type_: AccountType,
    pub skin_id: Option<String>,
}

#[derive(Debug)]
pub enum AccountType {
    /// Offline account with any username. Cannot log into servers.
    Offline,
    /// Authenticated MS account.
    Microsoft,
}

#[derive(Debug)]
pub struct AccountWithStatus {
    pub account: Account,
    pub status: AccountStatus,
}

#[derive(Debug)]
pub enum AccountStatus {
    /// An account that can be launched with the given access token.
    Ok { access_token: Option<String> },
    /// An account with an expired access token that needs to be refreshed.
    Expired,
    /// An account that is currently having its access token refreshed.
    Refreshing,
    /// An account that is unable to be refreshed and needs re-login.
    Invalid,
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum StatusFlags {
    BannedFromMultiplayer,
    XboxMultiplayerDisabled,
}
