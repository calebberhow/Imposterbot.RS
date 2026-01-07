use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use env_logger::{Builder, Env};
use imposterbot::infrastructure::botdata::Data;
use imposterbot::infrastructure::util::get_data_directory;
use imposterbot::{commands::minecraft::McServerList, infrastructure::environment};
use log::{error, info};
use poise::serenity_prelude::{self as serenity, GatewayIntents};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::sync::RwLock;

fn get_log_path_var() -> Option<bool> {
    match std::env::var(environment::LOG_PATH) {
        Ok(path) => match path.parse::<bool>() {
            Ok(value) => Some(value),
            Err(e) => {
                error!("Failed to parse the value: {:?}", e);
                None
            }
        },
        Err(_) => None,
    }
}

fn init_env_logger() {
    let env = Env::default()
        .filter_or(environment::LOG_LEVEL, "warn,imposterbot=info")
        .write_style_or(environment::LOG_STYLE, "always");
    Builder::from_env(env)
        .default_format()
        .format_source_path(get_log_path_var().unwrap_or(false))
        .format_timestamp_secs()
        .init();
}

fn load_env_file() -> Option<PathBuf> {
    dotenvy::dotenv().ok()
}

fn log_env_file_result(env_file: Option<PathBuf>) {
    if let Some(path) = env_file {
        info!("Loaded environment variables from {}", path.display());
    } else {
        info!("No .env file found, proceeding with system environment variables.");
    }
}

async fn try_create_db_pool() -> Result<Arc<sqlx::SqlitePool>, imposterbot::Error> {
    let data_dir = get_data_directory();
    let db_url = data_dir.join("imposterbot_data.db");
    info!(
        "Connecting to database with url: {}",
        db_url.to_str().unwrap()
    );
    let options = SqliteConnectOptions::from_str(db_url.to_str().unwrap())?.create_if_missing(true);

    // if (options.get_filename())
    // std::fs::create_dir_all(options.get_filename())?;

    let pool = SqlitePoolOptions::new()
        .connect_with(options)
        .await
        .unwrap();

    Ok(Arc::new(pool))
}

async fn create_db_pool() -> Arc<SqlitePool> {
    match try_create_db_pool().await {
        Err(e) => {
            // error!("Failed to create database pool: {:?}", e);
            panic!("Failed to create database pool: {:?}", e);
        }
        Ok(pool) => pool,
    }
}

async fn init_db(pool: Arc<SqlitePool>) -> Result<(), imposterbot::Error> {
    let mut conn = pool.acquire().await?;
    sqlx::migrate!().run(&mut *conn).await.unwrap();
    info!("Database initialized.");
    Ok(())
}

fn get_discord_token() -> String {
    let token = std::env::var(environment::DISCORD_TOKEN).expect(
        format!(
            "missing environment variable {}",
            environment::DISCORD_TOKEN
        )
        .as_str(),
    );
    info!("{} variable found.", environment::DISCORD_TOKEN);

    return token;
}

fn get_enabled_commands() -> Vec<poise::Command<Data, imposterbot::Error>> {
    let default_commands = vec![
        imposterbot::commands::builtins::help(),
        imposterbot::commands::builtins::register(),
        imposterbot::commands::minecraft::mcstatus(),
        imposterbot::commands::minecraft::rm_mcserver(),
        imposterbot::commands::minecraft::add_mcserver(),
        imposterbot::commands::minecraft::update_mcserver(),
        imposterbot::commands::roll::roll(),
        imposterbot::commands::coinflip::coinflip(),
    ];

    // Get the list of commands disabled by environment variable
    let disable_commands_env = std::env::var("COMMAND_DISABLE_LIST").unwrap_or_default();
    let disabled_commands = disable_commands_env.split(",");

    // Log the disabled commands
    let disabled_commands_info: HashSet<String> = disabled_commands
        .clone()
        .map(|s| s.to_lowercase())
        .filter(|s| {
            !s.is_empty()
                && default_commands
                    .iter()
                    .any(|cmd| cmd.name.to_lowercase() == *s)
        })
        .collect();
    if disabled_commands_info.is_empty() {
        info!("Loading default commands");
    } else {
        info!("Disabled commands: {:?}", disabled_commands_info);
    }

    // Return the enabled commands
    default_commands
        .into_iter()
        .filter(|cmd| {
            !disabled_commands
                .clone()
                .into_iter()
                .any(|disabled| cmd.name.to_uppercase() == disabled.to_uppercase())
        })
        .collect()
}

async fn get_init_mcsever_list(pool: Arc<SqlitePool>) -> McServerList {
    match McServerList::from_db(pool.clone()).await {
        Ok(list) => {
            info!(
                "Loaded {} Minecraft server configurations from database during startup.",
                list.servers.len()
            );
            return list;
        }
        Err(e) => {
            error!(
                "Failed to load Minecraft server configurations from database during startup: {:?}",
                e
            );

            return McServerList::default();
        }
    }
}

fn create_discord_framework(pool: Arc<SqlitePool>) -> poise::Framework<Data, imposterbot::Error> {
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: get_enabled_commands(),
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some("!".into()),
                mention_as_prefix: true,
                edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                    Duration::from_secs(3600),
                ))),
                ..Default::default()
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    info!(
                        "Executing Command: {:?} for {} ({})",
                        ctx.command().name,
                        ctx.author()
                            .clone()
                            .member
                            .and_then(|m| m.nick)
                            .unwrap_or(ctx.author().display_name().to_string()),
                        ctx.author().name
                    );
                })
            },
            on_error: |error| {
                Box::pin(async move {
                    if let Err(e) = poise::builtins::on_error(error).await {
                        error!("{:?}", e);
                    }
                })
            },
            event_handler: |_ctx, event, _framework, _data| {
                Box::pin(imposterbot::infrastructure::events::event_handler(
                    _ctx, event, _framework, _data,
                ))
            },
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
                let init_server_list = get_init_mcsever_list(pool.clone()).await;
                Ok(Data {
                    mcserver_list: Arc::new(RwLock::new(init_server_list)),
                    db_pool: pool,
                })
            })
        })
        .build();

    for cmd in framework.options().commands.iter() {
        info!("Loaded command: {:#?}", cmd.name);
    }

    return framework;
}

#[tokio::main]
async fn main() {
    let env_file = load_env_file();
    init_env_logger();
    info!("Starting Imposterbot...");
    log_env_file_result(env_file);
    let token = get_discord_token();

    let pool = create_db_pool().await;
    init_db(pool.clone()).await.unwrap();

    let framework = create_discord_framework(pool);

    let intents = serenity::GatewayIntents::non_privileged().union(GatewayIntents::MESSAGE_CONTENT);
    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client.unwrap().start().await.unwrap();
}
