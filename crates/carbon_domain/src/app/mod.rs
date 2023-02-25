pub mod configuration;

use crate::app::configuration::ConfigurationManager;
use crate::instance::Instance;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::RwLock;

type ManagerContainer<M> = RwLock<Arc<Weak<M>>>;

struct App {
    instances: Vec<RwLock<Instance>>,
    instances_by_name_index: HashMap<String, Instance>,
    configuration_manager: ManagerContainer<ConfigurationManager>,
}

impl App {
    fn start_instance_by_name(_instance_name: String) {}
}
