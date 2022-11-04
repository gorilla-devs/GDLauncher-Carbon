use accounts::{ms_auth::DeviceCode, ACCOUNTS};
use murmurhash32::murmurhash2;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::{io::Read, thread, time};

mod accounts;
mod component;
mod instance;
mod minecraft;

#[napi]
pub async fn fibonacci(num: i32, num1: u32) -> u32 {
    let ten_millis = time::Duration::from_secs(3);
    thread::sleep(ten_millis);
    (num as u32) + num1
}

#[allow(dead_code)]
#[napi]
async fn compute_path_murmur(path: String) -> Result<u32> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    buffer.retain(|&x| (x != 9 && x != 10 && x != 13 && x != 32));
    let hash = murmurhash2(&buffer);
    Ok(hash)
}

#[napi(ts_return_type = "Promise<string>")]
pub fn auth(
    env: Env,
    #[napi(ts_arg_type = "(token: string) => void")] reporter: JsFunction,
) -> napi::Result<napi::JsObject> {
    // creating a promise which we can later resolve from another thread
    let (deferred, promise) = env.create_deferred()?;

    // wrapping the reporter callback with a threadsafe function
    let tsfn: napi::threadsafe_function::ThreadsafeFunction<
        String, // changethis value to whatever you need, it is the parameter type of the function
        napi::threadsafe_function::ErrorStrategy::Fatal,
    > = reporter.create_threadsafe_function(
        0,
        |ctx: napi::threadsafe_function::ThreadSafeCallContext<String>| {
            // this callback transforms the input we get in the tsfn.call() into napi values
            ctx.env.create_string(ctx.value.as_str()).map(|v| vec![v])
        },
    )?;

    env.execute_tokio_future(
        async move {
            let client = reqwest::Client::new();
            let device_code = DeviceCode::new(&client).await.unwrap();
            let user_code = device_code.clone().inner.unwrap().user_code.clone();

            // this is how you can call the threadsafe function form another thread
            tsfn.call(
                user_code,
                napi::threadsafe_function::ThreadsafeFunctionCallMode::Blocking,
            );

            println!("device_code: {:?}", device_code);
            let auth = device_code.poll_device_code_auth(&client).await.unwrap();

            let mc_auth = auth.finalize_auth(&client).await.unwrap();
            let mc_profile = mc_auth.get_mc_profile(&client).await.unwrap();

            let account = accounts::Account {
                ms_data: auth,
                mc_data: mc_auth,
                mc_profile,
            };

            println!("{:?}", account.ms_data.get_id_token_claims().await.unwrap());
            let accounts = &*ACCOUNTS.lock().await;

            accounts.clone().add_account(account).await;

            // here the promise which we returned gets resolved with a computed value
            deferred.resolve(|_| Ok("Some stuff"));

            Ok(())
        },
        // this resolver converts the output of our async block to a js value which gets passed to the .then() callback is JS
        |&mut env, _| env.get_undefined(),
    )
    .unwrap();

    // here we instantly return a promise object which later can be resolved
    Ok(promise)
}
