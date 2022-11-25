use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::global_store::GLOBAL_STORE;

use super::{
    ms_auth::{AzureData, DeviceCode, AZURE_DATA},
    Account, Accounts,
};

#[napi(object, js_name = "Account")]
struct NAPIAccount {
    pub id: String,
    pub name: String,
    pub access_token: String,
}

impl From<Account> for NAPIAccount {
    fn from(account: Account) -> Self {
        Self {
            id: account.mc_profile.id,
            name: account.mc_profile.name,
            access_token: account.mc_data.access_token,
        }
    }
}

#[napi(object, js_name = "Accounts")]
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
                access_token: account.mc_data.access_token.clone(),
            })
            .collect();

        // Get index of selected account
        let selected_account = accounts.selected_account.map(|account| NAPIAccount {
            id: account.mc_profile.id.clone(),
            name: account.mc_profile.name.clone(),
            access_token: account.mc_data.access_token.clone(),
        });

        NAPIAccounts {
            accounts: accounts_new,
            selected_account,
        }
    }
}

#[napi(object)]
struct DeviceCodeObject {
    pub user_code: String,
    pub link: String,
    pub expires_at: i64,
}


#[napi(ts_return_type = "Promise<Account>")]
pub fn auth(
    env: Env,
    #[napi(ts_arg_type = "(deviceData: DeviceCodeObject) => void")] reporter: JsFunction,
) -> napi::Result<napi::JsObject> {
    // creating a promise which we can later resolve from another thread
    let (deferred, promise) = env.create_deferred()?;

    // wrapping the reporter callback with a threadsafe function
    let tsfn: napi::threadsafe_function::ThreadsafeFunction<
        DeviceCodeObject, // changethis value to whatever you need, it is the parameter type of the function
        napi::threadsafe_function::ErrorStrategy::Fatal,
    > = reporter.create_threadsafe_function(
        0,
        |ctx: napi::threadsafe_function::ThreadSafeCallContext<DeviceCodeObject>| {
            // this callback transforms the input we get in the tsfn.call() into napi values
            Ok(vec![ctx.value])
        },
    )?;

    env.execute_tokio_future(
        async move {
            let client = reqwest::Client::new();
            let device_code = DeviceCode::new(&client).await.unwrap();

            // this is how you can call the threadsafe function form another thread
            tsfn.call(
                DeviceCodeObject {
                    user_code: device_code.clone().inner.unwrap().user_code.clone().clone(),
                    link: device_code.clone().inner.unwrap().verification_uri.clone(),
                    expires_at: device_code
                        .clone()
                        .expires_at
                        .clone()
                        .unwrap()
                        .timestamp_millis(),
                },
                napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
            );

            println!("device_code: {:?}", device_code);
            let auth = await device_code.poll_device_code_auth(&client);

            let mc_auth = auth.finalize_auth(&client).await.unwrap();
            let mc_profile = mc_auth.get_mc_profile(&client).await.unwrap();

            let account = Account {
                ms_data: auth,
                mc_data: mc_auth,
                mc_profile,
            };
            let napi_account: NAPIAccount = account.clone().into();

            let mut store = GLOBAL_STORE.lock().await;

            let store = store
                .as_mut()
                .ok_or("Empty store")
                .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;

            store.accounts.add_account(account).await;
            // here the promise which we returned gets resolved with a computed value
            deferred.resolve(|_| Ok(napi_account));

            Ok(())
        },
        // this resolver converts the output of our async block to a js value which gets passed to the .then() callback is JS
        |&mut env, _| env.get_undefined(),
    )
    .map_err(|err| napi::Error::new(napi::Status::GenericFailure, err.to_string()))?;

    // here we instantly return a promise object which later can be resolved
    Ok(promise)
}

#[napi]
pub async fn init_azure_data() -> Result<()> {
    let azure_data = AzureData::new()
        .await
        .map_err(|e| napi::Error::new(napi::Status::GenericFailure, e.to_string()))?;
    *AZURE_DATA.lock().await = Some(azure_data);

    Ok(())
}
