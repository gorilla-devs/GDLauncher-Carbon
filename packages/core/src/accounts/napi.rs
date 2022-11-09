use napi::bindgen_prelude::*;
use napi_derive::napi;

use super::Accounts;

#[napi(object, js_name = "account")]
struct NAPIAccount {
    pub id: String,
    pub name: String,
}

#[napi(object, js_name = "accounts")]
struct NAPIAccounts {
    pub accounts: Vec<NAPIAccount>,
    pub selected_account: Option<NAPIAccount>,
}

impl From<Accounts> for NAPIAccounts {
    fn from(accounts: Accounts) -> Self {
        let accounts_new: Vec<NAPIAccount> = accounts
            .inner
            .into_iter()
            .map(|account| NAPIAccount {
                id: account.mc_profile.id.clone(),
                name: account.mc_profile.name.clone(),
            })
            .collect();

        // Get index of selected account
        let selected_account = accounts.selected_account.map(|account| NAPIAccount {
            id: account.mc_profile.id.clone(),
            name: account.mc_profile.name.clone(),
        });

        NAPIAccounts {
            accounts: accounts_new,
            selected_account,
        }
    }
}

#[napi]
pub async fn init_accounts() -> Result<NAPIAccounts> {
    let accounts = Accounts::new().await.unwrap();
    Ok(accounts.into())
}
