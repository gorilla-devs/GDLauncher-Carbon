#[napi::module_init]
fn init_core() {
    carbon_app::init();
}
