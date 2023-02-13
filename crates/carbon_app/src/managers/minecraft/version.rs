use std::sync::Arc;

use carbon_domain::minecraft::version::Libraries;

use crate::db::PrismaClient;

pub async fn get(db: Arc<PrismaClient>, version: String) {
    todo!()
}

pub async fn get_allowed_libraries() -> Libraries {
    todo!()
}
