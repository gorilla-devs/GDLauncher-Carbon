pub struct Account {
		pub username: String,
		pub uuid: String,
		pub type_: AccountType,
}

pub enum AccountType {
		/// Offline account with any username. Cannot log into servers.
		Offline,
		/// Authenticated MS account.
		Microsoft,
}

pub struct AccountWithStatus {
		pub account: Account,
		pub status: AccountStatus,
}

pub enum AccountStatus {
		/// An account that can be launched with the given access token.
		Launchable { access_token: Option<String> },
		/// An account with an expired access token that needs to be refreshed.
		Expired,
		/// An account that is currently having its access token refreshed.
		Refreshing,
}
