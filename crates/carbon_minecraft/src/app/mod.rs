mod app_configuration;

use std::collections::HashMap;
use crate::app::app_configuration::AppConfiguration;
use crate::instance::Instance;

struct App {
    /// this MUST be moved outside
    instances: Vec<Instance>,
    instances_by_name_index: HashMap<String, Instance>,
    app_configuration: AppConfiguration,
}


impl App {

    fn start_instance_by_name(instance_name: String) {

    }

}

