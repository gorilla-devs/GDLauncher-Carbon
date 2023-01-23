use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Ord, PartialOrd, PartialEq, Eq)]
pub struct AppConfiguration<'a> {
    pub _id: i32,
    pub default_db_url: &'a str,
}
