mod configuration;

use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::app::configuration::AppConfiguration;
use crate::instance::Instance;

struct App {
    instances: Vec<RwLock<Instance>>,
    instances_by_name_index: HashMap<String, Instance>,
    app_configuration: AppConfiguration,
}


impl App {

    fn start_instance_by_name(instance_name: String) {

    }

}

