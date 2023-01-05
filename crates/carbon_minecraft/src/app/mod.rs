use std::collections::HashMap;
use crate::instance::Instance;

struct App{   /// this MUST be moved outside
    instances : Vec<Instance>,
    instances_by_name_index : HashMap<String, Instance>
}