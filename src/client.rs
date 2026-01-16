use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context as _;
use imposterbot::infrastructure::{botdata::Data, environment, environment::env_var_with_context};
use poise::serenity_prelude::{self as serenity, GatewayIntents, UserId};
use sea_orm::DatabaseConnection;
use tracing::{debug, error, info, warn};

pub async fn create_serenity_client(db: DatabaseConnection) -> anyhow::Result<serenity::Client> {
    let token = env_var_with_context(environment::DISCORD_TOKEN)?;
    let intents = serenity::GatewayIntents::non_privileged()
        .union(GatewayIntents::MESSAGE_CONTENT)
        .union(GatewayIntents::GUILD_MEMBERS);
    let framework = create_poise_framework(db);

    let mut client_builder = serenity::ClientBuilder::new(token, intents).framework(framework);
    client_builder = configure_voice(client_builder);
    client_builder
        .await
        .context("Failed to create serenity client")
}

#[cfg(feature = "voice")]
fn configure_voice(builder: serenity::ClientBuilder) -> serenity::ClientBuilder {
    use songbird::SerenityInit;

    builder
        .register_songbird()
        .type_map_insert::<imposterbot::commands::voice::HttpKey>(reqwest::Client::new())
}

#[cfg(not(feature = "voice"))]
fn configure_voice(builder: serenity::ClientBuilder) -> serenity::ClientBuilder {
    builder
}

fn create_poise_framework(pool: DatabaseConnection) -> poise::Framework<Data, imposterbot::Error> {
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

                    if let Ok(mut invoc_time) = ctx.data().invoc_time.write() {
                        invoc_time.insert(ctx.id(), Instant::now());
                    }
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    if let Ok(invoc_time_map) = ctx.data().invoc_time.read() {
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

fn get_enabled_commands() -> Vec<poise::Command<Data, imposterbot::Error>> {
    let default_commands = vec![
        imposterbot::commands::builtins::help(),
        imposterbot::commands::builtins::register(),
        imposterbot::commands::minecraft::mc(),
        imposterbot::commands::roll::roll(),
        imposterbot::commands::coinflip::coinflip(),
        imposterbot::commands::member_management::channels::configure_welcome_channel(),
        imposterbot::commands::member_management::channels::configure_leave_channel(),
        imposterbot::commands::member_management::roles::add_default_member_role(),
        imposterbot::commands::member_management::roles::remove_default_member_role(),
        imposterbot::commands::member_management::notifications::test_member_add(),
        imposterbot::commands::member_management::notifications::test_member_remove(),
        imposterbot::commands::member_management::notifications::cfg_member_notification(),
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
