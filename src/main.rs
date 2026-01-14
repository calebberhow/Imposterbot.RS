use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use env_logger::{Builder, Env};

use imposterbot::infrastructure::botdata::Data;
use imposterbot::infrastructure::environment;
use imposterbot::infrastructure::util::get_data_directory;
use migration::{Migrator, MigratorTrait};
use poise::serenity_prelude::{self as serenity, GatewayIntents, UserId};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use tracing::{debug, error, info, warn};

#[cfg(feature = "voice")]
use songbird::SerenityInit;

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
        .format_timestamp_millis()
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

fn ensure_data_dir_created() -> tokio::io::Result<()> {
    let path = get_data_directory();
    std::fs::create_dir_all(path)
}

async fn try_create_db_pool() -> Result<DatabaseConnection, imposterbot::Error> {
    let db_url = std::env::var("DATABASE_URL").expect("missing environment variable DATABASE_URL");
    let opt = ConnectOptions::new(db_url.clone());
    if opt.get_url().starts_with("sqlite:") {}
    let db = Database::connect(opt).await?;
    Ok(db)
}

async fn create_db_pool() -> DatabaseConnection {
    match try_create_db_pool().await {
        Err(e) => {
            // error!("Failed to create database pool: {:?}", e);
            panic!("Failed to create database pool: {:?}", e);
        }
        Ok(pool) => pool,
    }
}

async fn init_db(db: &DatabaseConnection) -> Result<(), imposterbot::Error> {
    Migrator::up(db, None).await?;
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
        imposterbot::commands::minecraft::mc(),
        imposterbot::commands::roll::roll(),
        imposterbot::commands::coinflip::coinflip(),
        imposterbot::commands::member_management::configure_welcome_channel(),
        imposterbot::commands::member_management::add_default_member_role(),
        imposterbot::commands::member_management::remove_default_member_role(),
        imposterbot::commands::member_management::test_member_add(),
        imposterbot::commands::member_management::test_member_remove(),
        #[cfg(feature = "voice")]
        imposterbot::commands::voice::play(),
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

enum OwnerParseError {
    MissingEnvVar,
    UserIdParseError(String),
}

fn try_get_owners_env() -> Result<Vec<UserId>, OwnerParseError> {
    let env_var = std::env::var(environment::OWNERS).map_err(|_| OwnerParseError::MissingEnvVar)?;
    env_var
        .split(',')
        .into_iter()
        .map(|value| {
            value
                .trim()
                .parse::<u64>()
                .map(|num| UserId::new(num))
                .map_err(|e| OwnerParseError::UserIdParseError(e.to_string()))
        })
        .collect()
}

fn create_discord_framework(
    pool: DatabaseConnection,
) -> poise::Framework<Data, imposterbot::Error> {
    let initialize_owners: bool;
    let owners: std::collections::HashSet<UserId>;
    match try_get_owners_env() {
        Ok(owners_vec) => {
            initialize_owners = false;
            owners = std::collections::HashSet::from_iter(owners_vec);
        }
        Err(error) => {
            match error {
                OwnerParseError::UserIdParseError(e) => {
                    warn!("Invalid UserId in {}: {}", environment::OWNERS, e);
                }
                _ => {}
            }
            initialize_owners = true;
            owners = std::collections::HashSet::new();
        }
    }
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
            initialize_owners: initialize_owners,
            owners: owners,
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
                        ctx.author().name,
                    );

                    ctx.data()
                        .invoc_time
                        .write()
                        .unwrap()
                        .insert(ctx.id(), Instant::now());
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    let invoc_time_map = ctx.data().invoc_time.read().unwrap();
                    let start_time = invoc_time_map.get(&ctx.id());
                    match start_time {
                        Some(start_time) => {
                            let duration = start_time.elapsed();
                            debug!("Command {} finished in {:?}", ctx.command().name, duration);
                        }
                        None => {
                            error!(
                                "Post-command hook called for command without a start-time set."
                            );
                        }
                    }
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
                Box::pin(imposterbot::infrastructure::event_handler::event_handler(
                    _ctx, event, _framework, _data,
                ))
            },
            ..Default::default()
        })
        .setup(|_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    db_pool: pool,
                    invoc_time: Default::default(),
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
    ensure_data_dir_created().expect("Data directory should be creatable");
    let pool = create_db_pool().await;
    init_db(&pool).await.unwrap();
    let framework = create_discord_framework(pool);

    let intents = serenity::GatewayIntents::non_privileged()
        .union(GatewayIntents::MESSAGE_CONTENT)
        .union(GatewayIntents::GUILD_MEMBERS);
    #[cfg(feature = "voice")]
    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .register_songbird()
        .type_map_insert::<imposterbot::commands::voice::HttpKey>(reqwest::Client::new())
        .await
        .unwrap();
    #[cfg(not(feature = "voice"))]
    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await
        .unwrap();
    let client_future = client.start();

    tokio::select! {
        _ = termination() => {
            info!("Bot is shutting down!");
            client.shard_manager.shutdown_all().await;
        }
        _ = client_future => {
            error!("Bot event loop closed unexpectedly. Shutting down.");
        }
    }
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
