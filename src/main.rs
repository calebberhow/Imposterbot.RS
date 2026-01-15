mod client;
mod database;
mod logging;
mod shutdown;

use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logging::init_logger();
    let db = database::init_database().await?;

    let mut client = client::create_serenity_client(db).await?;
    let shard_manager = client.shard_manager.clone();
    let client_future = client.start();

    shutdown::run_until_shutdown(client_future, async move || {
        info!("Bot is shutting down!");
        shard_manager.shutdown_all().await;
        Ok(())
    })
    .await?;

    Ok(())
}
