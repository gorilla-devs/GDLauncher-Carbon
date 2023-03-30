pub fn new() -> reqwest::Client {
    // TODO! Add a cache
    reqwest::Client::builder().build().unwrap()
}
