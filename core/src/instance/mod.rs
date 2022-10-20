use std::{
    ops::Deref,
    sync::{Arc, MutexGuard},
};

use crate::component::{
    components::modloaders::{forge::ForgeModLoader, vanilla::VanillaModLoader},
    Component,
};

struct JavaMemoryOverride {
    min_mem_alloc: u16,
    max_mem_alloc: u16,
    perm_gen: u16,
}

pub struct Instance {
    components: Vec<Component>,
    override_java_path: Option<String>,
    override_java_args: Option<Vec<String>>,
    override_java_memory: Option<JavaMemoryOverride>,
}

impl Instance {
    fn add_component(&mut self, c: Component) {
        self.components.push(c);
    }
    fn add_components(&mut self, c: Vec<Component>) {
        self.components.extend(c);
    }
}
