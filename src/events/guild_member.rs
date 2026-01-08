/*
    Handles saying hello and goodbye when members join and leave the guild.

    Adds specified role(s) to new members.
*/

use std::sync::Arc;

use log::{error, trace, warn};
use poise::{
    CreateReply,
    serenity_prelude::{
        ChannelId, Context, CreateAttachment, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
        GuildId, Member, RoleId, User, futures::future,
    },
};
use sqlx::SqlitePool;

use crate::{
    Error,
    infrastructure::{
        botdata::Data,
        colors,
        util::{
            get_media_directory, lossless_i64_to_u64, lossless_u64_to_i64, send_message_from_reply,
        },
    },
};

async fn get_welcome_channel(db_pool: Arc<SqlitePool>, guild_id: &GuildId) -> Option<ChannelId> {
    let mut conn = match db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return None;
        }
    };

    struct WelcomeChannelResult {
        channel_id: i64,
    }

    let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
    let query_result = sqlx::query_file_as!(
        WelcomeChannelResult,
        "./src/queries/get_welcome_channel.sql",
        guild_id_i64
    )
    .fetch_one(&mut *conn)
    .await;

    match query_result {
        Ok(result) => Some(ChannelId::new(lossless_i64_to_u64(result.channel_id))),
        Err(_) => None,
    }
}

async fn get_member_roles_on_join(
    db_pool: Arc<SqlitePool>,
    guild_id: &GuildId,
) -> Option<Vec<RoleId>> {
    let mut conn = match db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return None;
        }
    };

    struct RoleResult {
        role_id: i64,
    }

    let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
    let query_result = sqlx::query_file_as!(
        RoleResult,
        "./src/queries/get_member_roles_on_join.sql",
        guild_id_i64
    )
    .fetch_all(&mut *conn)
    .await;

    match query_result {
        Ok(result) => Some(
            result
                .iter()
                .map(|role| RoleId::new(lossless_i64_to_u64(role.role_id)))
                .collect(),
        ),
        Err(e) => {
            error!("Failed to get member roles on join: {}", e);
            None
        }
    }
}

async fn say_hello(ctx: &Context, data: &Data, member: &Member) -> Result<(), Error> {
    let (welcome_channel, guild) = future::join(
        get_welcome_channel(data.db_pool.clone(), &member.guild_id),
        member.guild_id.to_partial_guild_with_counts(ctx), // TODO: this request is quite large and slow. Figure out how to more quickly retrieve the guild member count.
    )
    .await;
    let channel = match welcome_channel {
        Some(x) => x,
        None => return Ok(()), // Welcome channel not confiugred on this guild
    };

    let mut author = CreateEmbedAuthor::new("𝚆𝚎𝚕𝚌𝚘𝚖𝚎 𝚝𝚘 𝙲𝚘𝚣𝚢 𝙲𝚘𝚜𝚖𝚘𝚜!");
    if let Some(x) = member.user.avatar_url() {
        author = author.icon_url(x);
    }

    let attachment = CreateAttachment::path(get_media_directory().join("cozyanim.gif")).await?;
    let mut embed = CreateEmbed::new()
        .description(format!("{} has joined!", member.user.name))
        .author(author)
        .color(colors::royal_blue())
        .thumbnail(format!("attachment://{}", attachment.filename));

    // Only add member count footer if member count is available from http request
    if let Ok(g) = &guild
        && let Some(count) = g.approximate_member_count
    {
        embed = embed.footer(CreateEmbedFooter::new(format!("Member Count: {}", count)));
    }

    let reply = CreateReply::default()
        .embed(embed)
        .content(format!("Welcome {}!", member.display_name()))
        .attachment(attachment);
    send_message_from_reply(&channel, ctx, reply).await?;
    Ok(())
}

async fn say_goodbye(
    ctx: &Context,
    guild_id: &GuildId,
    data: &Data,
    user: &User,
) -> Result<(), Error> {
    let channel = match get_welcome_channel(data.db_pool.clone(), guild_id).await {
        Some(x) => x,
        None => return Ok(()),
    };

    let reply = CreateReply::default().content(format!("Goodbye, {}", user.name));

    send_message_from_reply(&channel, ctx, reply).await?;
    Ok(())
}

async fn add_initial_member_roles(
    ctx: &Context,
    data: &Data,
    new_member: &Member,
) -> Result<(), Error> {
    trace!("Added initial roles to {}", new_member.user.name);
    match get_member_roles_on_join(data.db_pool.clone(), &new_member.guild_id).await {
        Some(roles) => match new_member.add_roles(ctx, &roles).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        },
        None => Ok(()),
    }
}

pub async fn guild_member_add(
    ctx: &Context,
    data: &Data,
    new_member: &Member,
) -> Result<(), Error> {
    trace!("Guild member added {}", new_member.user.name);
    if let Err(e) = say_hello(ctx, data, new_member).await {
        error!("Failed to welcome new member: {}", e)
    }
    if let Err(e) = add_initial_member_roles(ctx, data, new_member).await {
        error!("Failed to add roles to new member: {}", e)
    }
    Ok(())
}

pub async fn guild_member_remove(
    ctx: &Context,
    data: &Data,
    guild_id: &GuildId,
    user: &User,
) -> Result<(), Error> {
    if let Err(e) = say_goodbye(ctx, guild_id, data, user).await {
        error!("Failed to welcome new member: {}", e)
    }
    Ok(())
}
