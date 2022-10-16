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

#[napi]
fn auth() {
    use {accounts::auth::*, reqwest::blocking::Client};

    let client = Client::new();
    let device_code =
     DeviceCode::new("221e73fa-365e-4263-9e06-7a0a1f277960"/* You would ideally replace this with your own CID which you can get from creating an azure application*/, None, &client).unwrap();

    if let Some(inner) = &device_code.inner {
        println!("{:?}", inner);
    }

    let auth = device_code.authenticate(&client).unwrap();
    println!("{:?}", auth);
}
