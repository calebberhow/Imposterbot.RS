use anyhow::{Context, Result};
use imposterbot::infrastructure::environment::{self, env_var_with_context, get_data_directory};
use migration::{Migrator, MigratorTrait};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::info;

pub async fn init_database() -> Result<DatabaseConnection> {
    ensure_data_dir_created()?;
    let db = create_db_pool().await?;
    init_db(&db).await?;

    Ok(db)
}

fn ensure_data_dir_created() -> Result<()> {
    let path = get_data_directory();
    std::fs::create_dir_all(&path).context(format!("Failed to create data directory {:?}", path))
}

async fn create_db_pool() -> Result<DatabaseConnection> {
    let db_url = env_var_with_context(environment::DATABASE_URL)?;
    let opt = ConnectOptions::new(db_url.clone());
    let db = Database::connect(opt).await?;
    Ok(db)
}

async fn init_db(db: &DatabaseConnection) -> Result<()> {
    let res = Migrator::up(db, None)
        .await
        .context("Failed to migrate database to latest");
    if res.is_ok() {
        info!("Database initialized.");
    }
    res
}
