use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sea_orm::DatabaseConnection;

#[derive(Debug)]
pub struct Data {
    pub db_pool: DatabaseConnection,
    pub invoc_time: Arc<RwLock<HashMap<u64, std::time::Instant>>>,
}
