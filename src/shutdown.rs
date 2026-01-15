use anyhow::Context;
use poise::serenity_prelude as serenity;

pub async fn run_until_shutdown<T, F, Fut>(
    client_future: T,
    cleanup: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    T: Future<Output = Result<(), serenity::Error>>,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<(), Box<dyn std::error::Error>>>,
{
    tokio::select! {
        term_result = termination() => {
            cleanup().await?;
            term_result.context("Recieved unexpected error from termination signal.")?;
        }
        client_result = client_future => {
            cleanup().await?;
            client_result.context("Bot event loop closed unexpectedly.")?;
        }
    }
    Ok(())
}

#[cfg(windows)]
async fn termination() -> tokio::io::Result<()> {
    tokio::signal::ctrl_c().await
}

#[cfg(unix)]
async fn termination() -> tokio::io::Result<()> {
    let sigint = tokio::signal::ctrl_c();
    let sigterm = sigterm();
    tokio::select! {
        res = sigint => res,
        res = sigterm => res
    }
}

#[cfg(unix)]
async fn sigterm() -> tokio::io::Result<()> {
    tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?
        .recv()
        .await;
    Ok(())
}
