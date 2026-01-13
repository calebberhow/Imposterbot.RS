use async_minecraft_ping::{ConnectionConfig, ServerError, StatusResponse};
use poise::CreateReply;
use poise::serenity_prelude::futures::{self, Stream, StreamExt};
use poise::serenity_prelude::{self as serenity};
use tracing::{debug, error, info, trace};

use crate::infrastructure::colors;
use crate::infrastructure::util::{
    DebuggableReply, defer_or_broadcast, lossless_u64_to_i64, require_guild_id,
};
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

    let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
    let mut conn = match ctx.data().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to aquire db connection: {}", e);
            return futures::stream::empty().boxed();
        }
    };
    struct McServerResult {
        name: String,
    }
    let result = sqlx::query_file_as!(
        McServerResult,
        "./src/queries/mc/get_all_mcserver_names.sql",
        guild_id_i64,
        partial
    )
    .fetch_all(&mut *conn)
    .await
    .unwrap_or_default();

    futures::stream::iter(result)
        .map(|info| info.name.to_string())
        .inspect(|name| trace!("Produced autocomplete value: {}", name))
        .boxed()
}

#[poise::command(slash_command, prefix_command, track_edits, track_deletion, guild_only)]
pub async fn mcstatus(
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

#[poise::command(
    slash_command,
    prefix_command,
    rename = "rm-mcserver",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn rm_mcserver(
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
    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let guild_id = require_guild_id(ctx)?;
    let i64_guild_id = lossless_u64_to_i64(guild_id.get());
    let query_result =
        sqlx::query_file!("./src/queries/mc/delete_mcserver.sql", name, i64_guild_id)
            .execute(&mut *conn)
            .await?;
    drop(conn);

    if query_result.rows_affected() == 0 {
        return Err(format!("Server '{}' not found in database. In-memory server list refreshed as it was desynchronized from database.", name).into());
    }

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
    let i64_guild_id = lossless_u64_to_i64(guild_id.get());
    let mut conn = ctx.data().db_pool.acquire().await?;
    struct McServerQueryResult {
        address: String,
        port: i64,
        version: String,
        modpack: String,
        custom_description: String,
        instructions: String,
        thumbnail: String,
    }
    sqlx::query_file_as!(
        McServerQueryResult,
        "./src/queries/mc/get_mcserver.sql",
        i64_guild_id,
        name
    )
    .fetch_optional(&mut *conn)
    .await
    .map_err(|e| e.into())
    .map(|result| match result {
        Some(value) => {
            let port = if value.port > 0 && value.port < u16::MAX as i64 {
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
            Some(McServerResult {
                address: value.address,
                port: port,
                version: version,
                modpack: modpack,
                custom_description: custom_description,
                instructions: instructions,
                thumbnail: thumbnail,
            })
        }
        _ => None,
    })
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "add-mcserver",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn add_mcserver(
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
        "add_mcserver executed with args"
    );

    let srv_match = get_mcserver(ctx, &name).await?;
    if let Some(_) = srv_match {
        return Err(format!("Server '{}' already exists.", name).into());
    }

    // Add server to database
    let guild_id = require_guild_id(ctx)?;
    let i64_guild_id = lossless_u64_to_i64(guild_id.get());
    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let port_or_zero = port.unwrap_or(0);
    let version_or_empty = version.unwrap_or("".into());
    let modpack_or_empty = modpack.unwrap_or("".into());
    let custom_description_or_empty = custom_description.unwrap_or("".into());
    let thumbnail_or_empty = thumbnail.unwrap_or("".into());
    let instructions_or_empty = instructions.unwrap_or("".into());
    sqlx::query_file!(
        "./src/queries/mc/insert_mcserver.sql",
        i64_guild_id,
        name,
        address,
        port_or_zero,
        version_or_empty,
        modpack_or_empty,
        thumbnail_or_empty,
        custom_description_or_empty,
        instructions_or_empty
    )
    .execute(&mut *conn)
    .await?;

    drop(conn);

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully added server '{}'", name))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "update-mcserver",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn update_mcserver(
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

    if address.is_none()
        && port.is_none()
        && version.is_none()
        && modpack.is_none()
        && custom_description.is_none()
        && instructions.is_none()
    {
        return Err("At least one parameter must be updated.".into());
    }

    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let guild_id = require_guild_id(ctx)?;
    let gid_i64 = lossless_u64_to_i64(guild_id.get());
    sqlx::query_file!(
        "./src/queries/mc/update_mcserver.sql",
        address,
        port_value,
        version,
        modpack,
        custom_description,
        instructions,
        thumbnail,
        gid_i64,
        name,
    )
    .execute(&mut *conn)
    .await?;
    drop(conn);

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully updated server '{}'", name))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
