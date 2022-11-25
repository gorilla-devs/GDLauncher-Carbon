#[napi::module_init]
fn init_core() {
    std::thread::spawn(|| {
        let runtime = tokio::runtime::Runtime::new();
        runtime
            .unwrap() /* This should never fail */
            .block_on(async {
                
            })
    });
}
