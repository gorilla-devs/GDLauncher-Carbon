#[cfg(test)]
mod generate_rspc_ts_bindings;

#[napi::module_init]
fn init_core() {
    carbon_app::init();
}
