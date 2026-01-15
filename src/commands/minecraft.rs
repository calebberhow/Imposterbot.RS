use async_minecraft_ping::{ConnectionConfig, ServerError, StatusResponse};
use poise::CreateReply;
use poise::serenity_prelude::futures::{self, Stream, StreamExt};
use poise::serenity_prelude::{self as serenity};
use sea_orm::ActiveValue::Set;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use tracing::{debug, info, trace};

use crate::entities::mc_server;
use crate::infrastructure::colors;
use crate::infrastructure::ids::{id_to_string, require_guild_id};
use crate::infrastructure::util::{DebuggableReply, defer_or_broadcast};
use crate::{Context, Error};

async fn ping_mc_server(
    config: impl Into<ConnectionConfig>,
) -> Result<StatusResponse, ServerError> {
    trace!("Pinging minecraft server");
    let conn = config.into().connect().await?;
    let response = conn.status().await?;
    trace!("Minecraft server response: {:?}", response.status);
    Ok(response.status)
}

async fn mcserver_autocomplete<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    // Get guild id
    debug!(
        partial = partial,
        "mcserver_autocomplete executed with args"
    );
    let guild_id = match require_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => return futures::stream::empty().boxed(),
    };

    let result: Vec<String> = mc_server::Entity::find()
        .select_only()
        .column(mc_server::Column::Name)
        .filter(mc_server::Column::GuildId.eq(id_to_string(guild_id)))
        .filter(mc_server::Column::Name.starts_with(partial))
        .order_by_asc(mc_server::Column::Name)
        .limit(10)
        .into_tuple()
        .all(&ctx.data().db_pool)
        .await
        .unwrap_or_default();
    trace!("Produced autocomplete values: {:?}", result);
    futures::stream::iter(result).boxed()
}

/// Set of commands to check status and update registration of advertised minecraft servers.
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    track_deletion,
    guild_only,
    subcommands("status", "remove", "add", "update")
)]
pub async fn mc(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Gets the status of a minecraft server advertised on this guild.
#[poise::command(slash_command, prefix_command, track_edits, track_deletion, guild_only)]
async fn status(
    ctx: Context<'_>,
    #[description = "Server Name"]
    #[autocomplete = "mcserver_autocomplete"]
    name: String,
    #[description = "Visible to you only? (default: true)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    debug!(
        name = name,
        ephemeral = ephemeral,
        "mcstatus executed with args"
    );

    let ephemeral_resolved = ephemeral.unwrap_or(true);
    let _typing = defer_or_broadcast(ctx, ephemeral_resolved).await?;

    let optional_server_info = get_mcserver(ctx, &name).await?;
    debug!("Found server info {:?}", optional_server_info);

    if let Some(server_info) = optional_server_info {
        let mut connection = ConnectionConfig::build(&server_info.address).with_srv_lookup();
        if let Some(port) = server_info.port {
            connection = connection.with_port(port);
        }
        let status_result = ping_mc_server(connection).await;

        let mut embed = serenity::CreateEmbed::new().title(format!("{} Server Status", &name));
        if let Some(port) = server_info.port {
            embed = embed.field(
                "Address",
                format!("{}:{}", &server_info.address, port),
                false,
            );
        } else {
            embed = embed.field("Address", &server_info.address, false);
        }

        if let Some(version) = server_info.version {
            embed = embed.field("Version", version, false);
        }

        if let Some(modpack) = server_info.modpack {
            embed = embed.field("Modpack", modpack, false);
        }

        if let Some(instructions) = server_info.instructions {
            embed = embed.field("Instructions", instructions, false);
        }

        if let Some(thumbnail) = server_info.thumbnail {
            embed = embed.thumbnail(thumbnail);
        }

        if let Ok(ref status) = status_result {
            let description = if let Some(s) = server_info.custom_description {
                s
            } else {
                match status.description {
                    async_minecraft_ping::ServerDescription::Plain(ref text) => text,
                    async_minecraft_ping::ServerDescription::Object { ref text } => text,
                }
                .clone()
            };
            embed = embed
                .color(colors::green())
                .description(description)
                .field("Status", "Online", false)
                .field(
                    "Players Online",
                    format!("{}/{}", status.players.online, status.players.max),
                    false,
                );
        } else {
            if let Some(description) = server_info.custom_description {
                embed = embed.description(description);
            }

            embed = embed.color(colors::red()).field("Status", "Offline", false);
            info!("Minecraft serer '{}' is offline.", name);
        }

        let reply = CreateReply::default()
            .embed(embed)
            .ephemeral(ephemeral_resolved);
        trace!("Sending reply: {:?}", DebuggableReply::new(&reply));
        ctx.send(reply).await?;
        Ok(())
    } else {
        info!("Minecraft server '{}' not found.", name);
        return Err(format!("Minecraft server '{}' not found.", name).into());
    }
}

/// Removes an advertised minecraft server.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
async fn remove(
    ctx: Context<'_>,
    #[autocomplete = "mcserver_autocomplete"]
    #[description = "Server Name"]
    name: String,
) -> Result<(), Error> {
    debug!(name = name, "rm_mcserver executed with args");

    let srv_match = get_mcserver(ctx, &name).await?;
    if let Some(_) = srv_match {
        return Err(format!("Server '{}' already exists.", name).into());
    }

    // Remove server from list
    let guild_id = require_guild_id(ctx)?;
    mc_server::Entity::delete_by_id((id_to_string(guild_id), name.clone()))
        .exec(&ctx.data().db_pool)
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully removed server '{}'", name))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct McServerResult {
    pub address: String,
    pub port: Option<u16>,
    pub version: Option<String>,
    pub modpack: Option<String>,
    pub custom_description: Option<String>,
    pub instructions: Option<String>,
    pub thumbnail: Option<String>,
}

async fn get_mcserver(ctx: Context<'_>, name: &String) -> Result<Option<McServerResult>, Error> {
    let guild_id = require_guild_id(ctx)?;

    let found = mc_server::Entity::find_by_id((id_to_string(guild_id), name.clone()))
        .one(&ctx.data().db_pool)
        .await?;

    match found {
        Some(value) => {
            let port = if value.port > 0 && value.port < u16::MAX as i32 {
                Some(value.port as u16)
            } else {
                None
            };
            let version = if !value.version.is_empty() {
                Some(value.version)
            } else {
                None
            };
            let modpack = if !value.modpack.is_empty() {
                Some(value.modpack)
            } else {
                None
            };
            let custom_description = if !value.custom_description.is_empty() {
                Some(value.custom_description)
            } else {
                None
            };
            let instructions = if !value.instructions.is_empty() {
                Some(value.instructions)
            } else {
                None
            };
            let thumbnail = if !value.thumbnail.is_empty() {
                Some(value.thumbnail)
            } else {
                None
            };
            Ok(Some(McServerResult {
                address: value.address,
                port: port,
                version: version,
                modpack: modpack,
                custom_description: custom_description,
                instructions: instructions,
                thumbnail: thumbnail,
            }))
        }
        _ => Ok(None),
    }
}

/// Adds an advertised minecraft server.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
async fn add(
    ctx: Context<'_>,
    name: String,
    address: String,
    port: Option<u16>,
    version: Option<String>,
    modpack: Option<String>,
    custom_description: Option<String>,
    instructions: Option<String>,
    thumbnail: Option<String>,
) -> Result<(), Error> {
    debug!(
        name = name,
        address = address,
        port = port,
        version = version,
        modpack = modpack,
        custom_description = custom_description,
        instructions = instructions,
        thumbnail = thumbnail,
        "add_mcserver executed with args"
    );

    let srv_match = get_mcserver(ctx, &name).await?;
    if let Some(_) = srv_match {
        return Err(format!("Server '{}' already exists.", name).into());
    }

    // Add server to database
    let guild_id = require_guild_id(ctx)?;
    let port_or_zero = port.unwrap_or(0);
    let version_or_empty = version.unwrap_or("".into());
    let modpack_or_empty = modpack.unwrap_or("".into());
    let custom_description_or_empty = custom_description.unwrap_or("".into());
    let instructions_or_empty = instructions.unwrap_or("".into());
    let thumbnail_or_empty = thumbnail.unwrap_or("".into());

    mc_server::Entity::insert(mc_server::ActiveModel {
        guild_id: Set(id_to_string(guild_id)),
        name: Set(name.clone()),
        address: Set(address),
        port: Set(port_or_zero as i32),
        version: Set(version_or_empty),
        modpack: Set(modpack_or_empty),
        custom_description: Set(custom_description_or_empty),
        instructions: Set(instructions_or_empty),
        thumbnail: Set(thumbnail_or_empty),
    })
    .exec(&ctx.data().db_pool)
    .await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully added server '{}'", name))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Updates an advertised minecraft server.
#[poise::command(
    slash_command,
    //prefix_command, // bug in proc-macro causes prefix commands with many Option<T> parameters to have exponential compilation times
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
async fn update(
    ctx: Context<'_>,
    #[autocomplete = "mcserver_autocomplete"] name: String,
    address: Option<String>,
    port: Option<u16>,
    version: Option<String>,
    clear_version: Option<bool>,
    modpack: Option<String>,
    clear_modpack: Option<bool>,
    custom_description: Option<String>,
    clear_custom_description: Option<bool>,
    instructions: Option<String>,
    clear_instructions: Option<bool>,
    thumbnail: Option<String>,
    clear_thumbnail: Option<bool>,
) -> Result<(), Error> {
    fn apply_clear<T>(value: Option<T>, clear: Option<bool>) -> Option<T>
    where
        T: Default,
    {
        if clear.unwrap_or(false) {
            Some(Default::default())
        } else {
            value
        }
    }

    debug!(
        name = name,
        address = address,
        port = port,
        version = version,
        clear_version = clear_version,
        modpack = modpack,
        clear_modpack = clear_modpack,
        custom_description = custom_description,
        clear_custom_description = clear_custom_description,
        instructions = instructions,
        clear_instructions = clear_instructions,
        thumbnail = thumbnail,
        clear_thumbnail = clear_thumbnail,
        "update_mcserver executed with args"
    );

    let srv_match = get_mcserver(ctx, &name).await?;

    // Return early if server does not exist
    if let None = srv_match {
        return Err(format!("Server '{}' does not exist.", name).into());
    }

    if address.is_none()
        && port.is_none()
        && version.is_none()
        && clear_version.is_none()
        && modpack.is_none()
        && clear_modpack.is_none()
        && custom_description.is_none()
        && clear_custom_description.is_none()
        && instructions.is_none()
        && clear_instructions.is_none()
        && thumbnail.is_none()
        && clear_thumbnail.is_none()
    {
        return Err("At least one parameter must be updated.".into());
    }

    let port_value = match port {
        Some(x) => {
            if x > 0 {
                Some(x)
            } else {
                None
            }
        }
        _ => None,
    };

    let version = apply_clear(version, clear_version);
    let modpack = apply_clear(modpack, clear_modpack);
    let custom_description = apply_clear(custom_description, clear_custom_description);
    let instructions = apply_clear(instructions, clear_instructions);
    let thumbnail = apply_clear(thumbnail, clear_thumbnail);

    let guild_id = require_guild_id(ctx)?;
    let mut model = mc_server::ActiveModel {
        guild_id: Set(id_to_string(guild_id)),
        name: Set(name.clone()),
        ..Default::default()
    };

    if let Some(x) = address {
        model.address = Set(x);
    }

    if let Some(x) = port_value {
        model.port = Set(x.into());
    }

    if let Some(x) = version {
        model.version = Set(x);
    }

    if let Some(x) = modpack {
        model.modpack = Set(x);
    }

    if let Some(x) = custom_description {
        model.custom_description = Set(x);
    }

    if let Some(x) = instructions {
        model.instructions = Set(x);
    }

    if let Some(x) = thumbnail {
        model.thumbnail = Set(x);
    }

    mc_server::Entity::update(model)
        .exec(&ctx.data().db_pool)
        .await?;

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully updated server '{}'", name))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
