use murmurhash32::murmurhash2;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::{io::Read, thread, time};

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
