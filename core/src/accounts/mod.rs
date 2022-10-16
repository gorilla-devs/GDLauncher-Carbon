use std::collections::HashMap;

use lazy_static::lazy_static;

pub mod auth;

pub type AccountId = String;

pub struct Account {
    pub username: String,
    pub account_id: AccountId,
}

pub struct Accounts {
    pub inner: HashMap<AccountId, Account>,
}

impl Accounts {
    pub fn new() -> Self {
        Accounts {
            inner: HashMap::new(),
        }
    }
    pub fn add_account(&mut self, account: Account) {
        self.inner.insert(account.account_id.clone(), account);
    }
}

lazy_static! {
    pub static ref ACCOUNTS: Accounts = Accounts::new();
}
