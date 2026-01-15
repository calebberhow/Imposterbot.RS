use migration::OnConflict;
use poise::{
    CreateReply,
    serenity_prelude::{
        GuildChannel, RoleId,
        futures::{self, Stream, StreamExt},
    },
};
use sea_orm::{ActiveValue::Set, EntityTrait};
use tracing::{debug, trace};

use crate::{
    Context, Error,
    entities::{welcome_channel, welcome_roles},
    events::guild_member::{get_member_roles_on_join, guild_member_add, guild_member_remove},
    infrastructure::ids::{id_to_string, require_guild_id},
};

async fn default_role_autocomplete<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    // Get guild id
    debug!(
        partial = partial,
        "default_role_autocomplete executed with args"
    );
    let guild_id = match require_guild_id(ctx) {
        Ok(id) => id,
        Err(_) => return futures::stream::empty().boxed(),
    };

    let roles = get_member_roles_on_join(&ctx.data().db_pool, &guild_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|r| id_to_string(r.clone()));

    futures::stream::iter(roles).boxed()
}

/// Configures a channel for the bot to send welcome and goodbye messages to.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only,
    category = "Management"
)]
pub async fn configure_welcome_channel(
    ctx: Context<'_>,
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    trace!("configured welcome channel: {:?}", channel);
    let guild_id = require_guild_id(ctx)?;

    if let Some(channel) = channel {
        welcome_channel::Entity::insert(welcome_channel::ActiveModel {
            guild_id: Set(id_to_string(guild_id.clone())),
            channel_id: Set(id_to_string(channel.id.clone())),
        })
        .on_conflict(
            OnConflict::column(welcome_channel::Column::GuildId)
                .update_columns([welcome_channel::Column::ChannelId])
                .to_owned(),
        )
        .exec(&ctx.data().db_pool)
        .await?;
        ctx.send(
            CreateReply::default()
                .content("Successfully set welcome channel")
                .ephemeral(true),
        )
        .await?;
    } else {
        welcome_channel::Entity::delete_by_id(id_to_string(guild_id))
            .exec(&ctx.data().db_pool)
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

/// Adds a role that will be applied to all new members when they join.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only,
    category = "Management"
)]
pub async fn add_default_member_role(ctx: Context<'_>, role: RoleId) -> Result<(), Error> {
    trace!("adding default member role: {:?}", role);
    let guild_id = require_guild_id(ctx)?;

    welcome_roles::Entity::insert(welcome_roles::ActiveModel {
        guild_id: Set(id_to_string(guild_id.clone())),
        role_id: Set(id_to_string(role.clone())),
    })
    .exec(&ctx.data().db_pool)
    .await?;

    ctx.send(
        CreateReply::default()
            .content("Successfully added default role")
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Removes a role that is applied to all new members when they join.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only,
    category = "Management"
)]
pub async fn remove_default_member_role(
    ctx: Context<'_>,
    #[autocomplete = "default_role_autocomplete"] role: String,
) -> Result<(), Error> {
    trace!("deleting default member role: {:?}", role);
    let guild_id = require_guild_id(ctx)?;

    let role_id = match guild_id.roles(ctx).await {
        Ok(roles) => roles
            .iter()
            .find(|r: &(&RoleId, &poise::serenity_prelude::Role)| r.1.name == role)
            .map(|x| x.0.clone()),
        Err(e) => return Err(e.into()),
    };

    match role_id {
        Some(role_id) => {
            welcome_roles::Entity::delete_by_id((id_to_string(guild_id), id_to_string(role_id)))
                .exec(&ctx.data().db_pool)
                .await?;

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

/// Tests the welcome functions by simulating a member joining the guild.
#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    guild_only,
    hide_in_help,
    category = "Management"
)]
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

/// Tests the welcome functions by simulating a member leaving the guild.
#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    guild_only,
    hide_in_help,
    category = "Management"
)]
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
