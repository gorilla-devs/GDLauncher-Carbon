#[cfg(feature = "production")]
#[inline(always)]
pub fn get_client() -> reqwest_middleware::ClientBuilder {
    iridium::get_client()
}

#[cfg(not(feature = "production"))]
#[inline(always)]
pub fn get_client() -> reqwest_middleware::ClientBuilder {
    let client = reqwest::Client::builder().build().unwrap();
    reqwest_middleware::ClientBuilder::new(client)
}
