use std::sync::Arc;

use async_minecraft_ping::{ConnectionConfig, ServerError, StatusResponse};
use log::{info, trace, warn};
use poise::CreateReply;
use poise::serenity_prelude::futures::{self, Stream, StreamExt};
use poise::serenity_prelude::{self as serenity, GuildId};
use sqlx::SqlitePool;

use crate::infrastructure::util::{
    DebuggableReply, lossless_i64_to_u64, lossless_u64_to_i64, require_guild_id,
};
use crate::{Context, Error};

#[derive(Debug)]
pub struct McServerList {
    pub servers: Vec<McServerInfo>,
}

impl Default for McServerList {
    fn default() -> Self {
        McServerList { servers: vec![] }
    }
}

#[derive(Debug, Clone)]
pub struct McServerInfo {
    pub guild: GuildId,
    pub address: String,
    pub port: Option<u16>,
    pub name: String,
}

impl McServerInfo {
    pub fn new(
        guild: GuildId,
        name: impl Into<String>,
        address: impl Into<String>,
        port: Option<u16>,
    ) -> Self {
        McServerInfo {
            guild: guild,
            address: address.into(),
            port,
            name: name.into(),
        }
    }
}

impl Into<ConnectionConfig> for McServerInfo {
    fn into(self) -> ConnectionConfig {
        let mut config = ConnectionConfig::build(self.address).with_srv_lookup();
        if let Some(port) = self.port {
            config = config.with_port(port);
        }
        config
    }
}

impl McServerList {
    pub fn find(&self, guild_id: GuildId, name: impl Into<String>) -> Option<&McServerInfo> {
        let name = name.into();

        for server_info in &self.servers {
            if guild_id == server_info.guild && server_info.name.eq_ignore_ascii_case(&name) {
                return Some(server_info);
            }
        }
        None
    }

    pub async fn refresh(&mut self, pool: Arc<SqlitePool>) -> Result<(), Error> {
        let mut conn = pool.acquire().await?;

        // Exact match of database schema
        struct McServerInfoDto {
            guild_id: i64,
            address: String,
            port: i64,
            name: String,
        }
        let sql_result =
            sqlx::query_file_as!(McServerInfoDto, "./src/queries/get_all_mcserver.sql")
                .fetch_all(&mut *conn)
                .await?;

        self.servers.clear();
        for server in sql_result {
            // Converts exact match of database schema into idiomatic version
            self.servers.push(McServerInfo {
                guild: GuildId::new(lossless_i64_to_u64(server.guild_id)),
                address: server.address,
                port: if server.port > 0 && server.port < u16::MAX as i64 {
                    Some(server.port as u16)
                } else {
                    None
                },
                name: server.name,
            });
        }

        Ok(())
    }

    pub async fn from_db(pool: Arc<SqlitePool>) -> Result<Self, Error> {
        let mut server_list = McServerList::default();
        server_list.refresh(pool).await?;
        Ok(server_list)
    }
}

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
    trace!(partial=partial; "mcserver_autocomplete executed with args");
    let guild_id = match require_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => return futures::stream::empty().boxed(),
    };

    let list = ctx.data().mcserver_list.read().await;
    let clone = list.servers.clone();
    drop(list); // Release the read lock early
    let stream = futures::stream::iter(clone)
        .filter(move |info| {
            futures::future::ready(
                info.guild.get() == guild_id.get()
                    && info
                        .name
                        .to_lowercase()
                        .starts_with(&partial.to_lowercase()),
            )
        })
        .map(|info| info.name.to_string())
        .inspect(|name| trace!("Produced autocomplete value: {}", name))
        .boxed();
    stream
}

#[poise::command(slash_command, prefix_command, track_edits, track_deletion)]
pub async fn mcstatus(
    ctx: Context<'_>,
    #[description = "Server Name"]
    #[autocomplete = "mcserver_autocomplete"]
    server: String,
    #[description = "Visible to you only? (default: true)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    trace!(server=server.as_str(), ephemeral=ephemeral; "mcstatus executed with args");

    let guild_id = require_guild_id(ctx)?;

    let ephemeral_resolved = ephemeral.unwrap_or(true);
    let servers = ctx.data().mcserver_list.read().await;

    let optional_server_info = servers.find(guild_id, &server);
    trace!("Found server info {:?}", optional_server_info);

    if let Some(server_info) = optional_server_info {
        if ephemeral_resolved {
            ctx.defer_ephemeral().await?;
        } else {
            ctx.defer_or_broadcast().await?;
        }
        let status_result = ping_mc_server(server_info.clone()).await;

        let mut reply = CreateReply::default();
        if let Ok(ref status) = status_result {
            let description = match status.description {
                async_minecraft_ping::ServerDescription::Plain(ref text) => text,
                async_minecraft_ping::ServerDescription::Object { ref text } => text,
            };

            let embed = serenity::CreateEmbed::new()
                .title("Minecraft Server Status")
                .description(description)
                .field("Server", &server, false)
                .field("Address", &server_info.address, false)
                .field("Status", "Online", true)
                .field(
                    "Players Online",
                    format!("{}/{}", status.players.online, status.players.max),
                    false,
                );

            reply = reply.embed(embed).ephemeral(ephemeral_resolved);
        } else {
            warn!("Minecraft serer '{}' is offline.", server);
            let embed = serenity::CreateEmbed::new()
                .title("Minecraft Server Status")
                .field("Server", &server, false)
                .field("Address", &server_info.address, false)
                .field("Status", "Offline", true);

            reply = reply.embed(embed).ephemeral(ephemeral_resolved);
        }

        trace!("Sending reply: {:?}", DebuggableReply::new(&reply));
        ctx.send(reply).await?;
        Ok(())
    } else {
        info!("Minecraft server '{}' not found.", server);
        return Err(format!("Minecraft server '{}' not found.", server).into());
    }
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "rm-mcserver",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn rm_mcserver(
    ctx: Context<'_>,
    #[autocomplete = "mcserver_autocomplete"]
    #[description = "Server Name"]
    server: String,
) -> Result<(), Error> {
    trace!(server=server.as_str(); "rm_mcserver executed with args");
    let mut servers = ctx.data().mcserver_list.write().await;
    let srv_match = servers
        .servers
        .iter()
        .position(|s| s.name.to_lowercase() == server.to_lowercase());

    let guild_id = require_guild_id(ctx)?;

    // Return early if server not found
    if let None = srv_match {
        return Err(format!("Server '{}' not found.", server).into());
    }

    // Remove server from list
    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let i64_guild_id = lossless_u64_to_i64(guild_id.get());
    let query_result = sqlx::query_file!("./src/queries/delete_mcserver.sql", server, i64_guild_id)
        .execute(&mut *conn)
        .await?;
    drop(conn);

    if query_result.rows_affected() == 0 {
        servers.refresh(ctx.data().db_pool.clone()).await?;
        return Err(format!("Server '{}' not found in database. In-memory server list refreshed as it was desynchronized from database.", server).into());
    }

    servers.servers.swap_remove(srv_match.unwrap());
    drop(servers); // Release the write lock early

    ctx.send(
        CreateReply::default()
            .content(format!("Successfully removed server '{}'", server))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    rename = "add-mcserver",
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn add_mcserver(
    ctx: Context<'_>,
    name: String,
    address: String,
    port: Option<u16>,
) -> Result<(), Error> {
    trace!(name=name.as_str(), address=address.as_str(), port=port; "rm_mcserver executed with args");
    let mut servers = ctx.data().mcserver_list.write().await;
    let srv_match = servers
        .servers
        .iter()
        .position(|s| s.name.to_lowercase() == name.to_lowercase());

    // Return early if server already exists
    if let Some(_) = srv_match {
        return Err(format!("Server '{}' already exists.", name).into());
    }

    let guild_id = require_guild_id(ctx)?;

    // Add server to database
    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let i64_guild_id = lossless_u64_to_i64(guild_id.get());
    let query_result = sqlx::query_file!(
        "./src/queries/insert_mcserver.sql",
        i64_guild_id,
        name,
        address,
        port
    )
    .execute(&mut *conn)
    .await?;

    drop(conn);

    if query_result.rows_affected() == 0 {
        servers.refresh(ctx.data().db_pool.clone()).await?;
        return Err(format!("Server '{}' already in database. In-memory server list refreshed as it was desynchronized from database.", name).into());
    }

    // Add server to list
    servers
        .servers
        .push(McServerInfo::new(guild_id, &name, address, port));
    drop(servers); // Release the write lock early

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
    default_member_permissions = "ADMINISTRATOR"
)]
pub async fn update_mcserver(
    ctx: Context<'_>,
    #[autocomplete = "mcserver_autocomplete"] name: String,
    address: Option<String>,
    port: Option<u16>,
) -> Result<(), Error> {
    trace!(name=name.as_str(), address=address.as_ref().map(|f| f.as_str()), port=port; "update_mcserver executed with args");
    let mut servers = ctx.data().mcserver_list.write().await;
    let srv_match = servers
        .servers
        .iter()
        .position(|s| s.name.to_lowercase() == name.to_lowercase());

    let guild_id = require_guild_id(ctx)?;

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

    if address.is_none() && port.is_none() {
        return Err(
            "At least one of 'address' or 'port' must be provided to update the server.".into(),
        );
    }

    let mut conn = ctx.data().db_pool.clone().acquire().await?;
    let gid_i64 = lossless_u64_to_i64(guild_id.get());
    let query_result = sqlx::query_file!(
        "./src/queries/update_mcserver.sql",
        address,
        port_value,
        name,
        gid_i64
    )
    .execute(&mut *conn)
    .await?;
    drop(conn);

    if query_result.rows_affected() == 0 {
        servers.refresh(ctx.data().db_pool.clone()).await?;
        return Err(format!("Server '{}' not found in database. In-memory server list refreshed as it was desynchronized from database.", name).into());
    }

    // Update server in list
    if address.is_some() {
        servers.servers[srv_match.unwrap()].address = address.unwrap();
    }

    if port.is_some() {
        servers.servers[srv_match.unwrap()].port = port;
    }

    drop(servers); // Release the write lock early
    ctx.send(
        CreateReply::default()
            .content(format!("Successfully updated server '{}'", name))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
