use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use sqlx::SqlitePool;

pub struct Data {
    pub db_pool: Arc<SqlitePool>,
    pub invoc_time: Arc<RwLock<HashMap<u64, std::time::Instant>>>,
}
