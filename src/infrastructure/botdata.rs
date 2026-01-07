use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::RwLock;

use crate::commands::minecraft::McServerList;

pub struct Data {
    pub mcserver_list: Arc<RwLock<McServerList>>,
    pub db_pool: Arc<SqlitePool>,
}
