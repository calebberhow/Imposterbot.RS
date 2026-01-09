use log::{trace, warn};
use poise::{
    CreateReply,
    serenity_prelude::{
        GuildChannel, RoleId,
        futures::{self, Stream, StreamExt},
    },
};

use crate::{
    Context, Error,
    events::guild_member::{guild_member_add, guild_member_remove},
    infrastructure::util::{lossless_i64_to_u64, lossless_u64_to_i64, require_guild_id},
};

async fn default_role_autocomplete<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    // Get guild id
    trace!(partial=partial; "default_role_autocomplete executed with args");
    let guild_id = match require_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => return futures::stream::empty().boxed(),
    };

    let conn = match ctx.data().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return futures::stream::empty().boxed();
        }
    };

    let guild_id_value = lossless_u64_to_i64(guild_id.get());
    async_stream::stream! {
        let mut conn = conn;

        struct RoleObj {
            role_id: i64,
        }

        let mut rows = sqlx::query_file_as!(
            RoleObj,
            "./src/queries/get_member_roles_on_join.sql",
            guild_id_value
        )
        .fetch(&mut *conn);

        while let Some(row) = rows.next().await {
            let role_obj = match row {
                Ok(r) => r,
                Err(_) => continue,
            };

            let role_id = RoleId::new(lossless_i64_to_u64(role_obj.role_id));

            let role = match guild_id.role(&ctx, role_id).await {
                Ok(r) => r,
                Err(_) => continue,
            };

            if role.name.to_lowercase().starts_with(&partial.to_lowercase()) {
                yield role.name;
            }
        }
    }
    .boxed()
}

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn configure_welcome_channel(
    ctx: Context<'_>,
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    trace!("configured welcome channel: {:?}", channel);
    let guild_id = require_guild_id(ctx)?;

    let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
    let mut conn = match ctx.data().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return Err(e.into());
        }
    };
    if let Some(channel) = channel {
        let channel_id_i64 = lossless_u64_to_i64(channel.id.get());
        sqlx::query_file!(
            "./src/queries/add_or_update_welcome_channel.sql",
            guild_id_i64,
            channel_id_i64
        )
        .execute(&mut *conn)
        .await?;
        ctx.send(
            CreateReply::default()
                .content("Successfully set welcome channel")
                .ephemeral(true),
        )
        .await?;
    } else {
        sqlx::query_file!("./src/queries/delete_welcome_channel.sql", guild_id_i64)
            .execute(&mut *conn)
            .await?;

        ctx.send(
            CreateReply::default()
                .content("Successfully removed welcome channel")
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn add_default_member_role(ctx: Context<'_>, role: RoleId) -> Result<(), Error> {
    trace!("adding default member role: {:?}", role);
    let guild_id = require_guild_id(ctx)?;
    let mut conn = match ctx.data().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return Err(e.into());
        }
    };

    let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
    let channel_id_i64 = lossless_u64_to_i64(role.get());
    let query_result = sqlx::query_file!(
        "./src/queries/add_member_role_on_join.sql",
        guild_id_i64,
        channel_id_i64
    )
    .execute(&mut *conn)
    .await?;

    if query_result.rows_affected() != 1 {
        warn!(
            "Unexpected query result while adding default member role: {:?}",
            query_result
        );
        return Err("Failed to add default member role".into());
    }
    ctx.send(
        CreateReply::default()
            .content("Successfully added default role")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn remove_default_member_role(
    ctx: Context<'_>,
    #[autocomplete = "default_role_autocomplete"] role: String,
) -> Result<(), Error> {
    trace!("deleting default member role: {:?}", role);
    let guild_id = require_guild_id(ctx)?;
    let mut conn = match ctx.data().db_pool.acquire().await {
        Ok(c) => c,
        Err(e) => {
            warn!("DB connection failed: {}", e);
            return Err(e.into());
        }
    };

    let role_id = match guild_id.roles(ctx).await {
        Ok(roles) => roles
            .iter()
            .find(|r: &(&RoleId, &poise::serenity_prelude::Role)| r.1.name == role)
            .map(|x| x.0.clone()),
        Err(e) => return Err(e.into()),
    };

    match role_id {
        Some(role_id) => {
            let guild_id_i64 = lossless_u64_to_i64(guild_id.get());
            let channel_id_i64 = lossless_u64_to_i64(role_id.get());
            let query_result = sqlx::query_file!(
                "./src/queries/remove_member_role_on_join.sql",
                guild_id_i64,
                channel_id_i64
            )
            .execute(&mut *conn)
            .await?;

            if query_result.rows_affected() != 1 {
                warn!(
                    "Unexpected query result while removing default member role: {:?}",
                    query_result
                );
                return Err("Failed to remove default member role".into());
            }
            ctx.send(
                CreateReply::default()
                    .content("Successfully removed default role")
                    .ephemeral(true),
            )
            .await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .content("Role not found")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, owners_only, guild_only)]
pub async fn test_member_add(ctx: Context<'_>) -> Result<(), Error> {
    let member = match ctx.author_member().await {
        Some(member) => member,
        None => return Err("Must be in guild".into()),
    };
    guild_member_add(ctx.serenity_context(), ctx.data(), &member).await?;
    ctx.send(
        CreateReply::default()
            .content("Acknowledged!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, owners_only, guild_only)]
pub async fn test_member_remove(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = require_guild_id(ctx)?;
    guild_member_remove(ctx.serenity_context(), ctx.data(), &guild_id, ctx.author()).await?;
    ctx.send(
        CreateReply::default()
            .content("Acknowledged!")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}
