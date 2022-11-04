use std::{collections::HashSet, sync::Arc};

use anyhow::{bail, Ok, Result};
use chrono::{Duration, Local};
use jsonwebtoken::TokenData;
use lazy_static::lazy_static;
use tokio::sync::Mutex;

use self::ms_auth::{
    AzureData, AzureJwtClaims, McAuth, McProfile, MsAuth, OpenIdResponse, AZURE_DATA,
    AZ_OPENID_URL, MS_KEY,
};

pub mod ms_auth;
pub mod napi;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Account {
    pub ms_data: MsAuth,
    pub mc_data: McAuth,
    pub mc_profile: McProfile,
}

/// Basic data structure for accounts
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Accounts {
    inner: HashSet<Arc<Account>>,
    selected_account: Option<Arc<Account>>,
}

impl Accounts {
    pub async fn new() -> Result<Self> {
        let azure_data = AzureData::new().await?;
        *AZURE_DATA.lock().await = Some(azure_data);

        Ok((&*ACCOUNTS).lock().await.clone())
    }
    pub async fn add_account(&mut self, account: Account) {
        let account_ref = Arc::new(account);

        self.inner.insert(account_ref.clone());
        self.selected_account = Some(account_ref.clone());
    }

    pub async fn remove_account(&mut self, account: Arc<Account>) {
        self.inner.remove(&account);

        if self.selected_account == Some(account.clone()) {
            // Assign next account if available
            if let Some(account) = self.inner.iter().next() {
                self.selected_account = Some(account.clone());
            } else {
                self.selected_account = None;
            }
        }
    }

    /// Verifies whether an account is still valid.
    /// If it's not, it will try to refresh tokens.
    /// If that fails, it will flag the account as expired.
    pub async fn validate_account(&self, account: Arc<Account>) -> Result<()> {
        let azure_data = AZURE_DATA.lock().await;
        if let Some(mut data) = (&*azure_data).clone() {
            data.validate_token(&account.ms_data.id_token).await?;

            return Ok(());
        }
        bail!("Azure data not initialized");
    }

    pub fn get_selected_account(&self) -> Option<Arc<Account>> {
        self.selected_account.clone()
    }

    pub fn get_accounts(&self) -> &HashSet<Arc<Account>> {
        &self.inner
    }
}

lazy_static! {
    pub static ref ACCOUNTS: Arc<Mutex<Accounts>> = Arc::new(Mutex::new(Accounts {
        inner: HashSet::new(),
        selected_account: None,
    }));
}
