/*
    Handles saying hello and goodbye when members join and leave the guild.

    Adds specified role(s) to new members.
*/

use std::sync::Arc;

use log::{error, warn};
use poise::serenity_prelude::{ChannelId, GuildId, Member, User};
use sqlx::SqlitePool;

use crate::{
    Error,
    infrastructure::util::{lossless_i64_to_u64, lossless_u64_to_i64},
};

async fn _get_welcome_channel(db_pool: Arc<SqlitePool>, guild_id: GuildId) -> Option<ChannelId> {
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

async fn say_hello(_user: &User) -> Result<(), Error> {
    Ok(())
}

async fn say_goodbye(_guild_id: &GuildId, _user: &User) -> Result<(), Error> {
    Ok(())
}

async fn add_initial_member_roles(_new_member: &Member) -> Result<(), Error> {
    Ok(())
}

pub async fn guild_member_add(new_member: &Member) -> Result<(), Error> {
    if let Err(e) = say_hello(&new_member.user).await {
        error!("Failed to welcome new member: {}", e)
    }
    if let Err(e) = add_initial_member_roles(new_member).await {
        error!("Failed to add roles to new member: {}", e)
    }
    Ok(())
}

pub async fn guild_member_remove(
    guild_id: &GuildId,
    user: &User,
    _member_data_if_available: &Option<Member>,
) -> Result<(), Error> {
    if let Err(e) = say_goodbye(guild_id, user).await {
        error!("Failed to welcome new member: {}", e)
    }
    Ok(())
}
