use migration::OnConflict;
use poise::{CreateReply, serenity_prelude::GuildChannel};
use sea_orm::{ActiveValue::Set, EntityTrait};
use tracing::trace;

use crate::{
    Context, Error,
    entities::member_notification_channel,
    infrastructure::ids::{id_to_string, require_guild_id},
};

/// Configures a channel for the bot to send welcome messages to.
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
    #[description = "Channel to send member joined notifications. If not provided, the bot will not send notifications."]
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    trace!("configured welcome channel: {:?}", channel);
    let guild_id = require_guild_id(ctx)?;

    if let Some(channel) = channel {
        member_notification_channel::Entity::insert(member_notification_channel::ActiveModel {
            guild_id: Set(id_to_string(guild_id.clone())),
            join: Set(true),
            channel_id: Set(id_to_string(channel.id.clone())),
        })
        .on_conflict(
            OnConflict::columns([
                member_notification_channel::Column::GuildId,
                member_notification_channel::Column::Join,
            ])
            .update_columns([member_notification_channel::Column::ChannelId])
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
        member_notification_channel::Entity::delete_by_id((id_to_string(guild_id), true))
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

/// Configures a channel for the bot to send goodbye messages to.
#[poise::command(
    slash_command,
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    default_member_permissions = "ADMINISTRATOR",
    guild_only,
    category = "Management"
)]
pub async fn configure_leave_channel(
    ctx: Context<'_>,
    #[description = "Channel to send member left notifications. If not provided, the bot will not send notifications."]
    channel: Option<GuildChannel>,
) -> Result<(), Error> {
    trace!("configured leave channel: {:?}", channel);
    let guild_id = require_guild_id(ctx)?;

    if let Some(channel) = channel {
        member_notification_channel::Entity::insert(member_notification_channel::ActiveModel {
            guild_id: Set(id_to_string(guild_id.clone())),
            join: Set(false),
            channel_id: Set(id_to_string(channel.id.clone())),
        })
        .on_conflict(
            OnConflict::columns([
                member_notification_channel::Column::GuildId,
                member_notification_channel::Column::Join,
            ])
            .update_columns([member_notification_channel::Column::ChannelId])
            .to_owned(),
        )
        .exec(&ctx.data().db_pool)
        .await?;
        ctx.send(
            CreateReply::default()
                .content("Successfully set leave channel")
                .ephemeral(true),
        )
        .await?;
    } else {
        member_notification_channel::Entity::delete_by_id((id_to_string(guild_id), false))
            .exec(&ctx.data().db_pool)
            .await?;

        ctx.send(
            CreateReply::default()
                .content("Successfully removed leave channel")
                .ephemeral(true),
        )
        .await?;
    }

    Ok(())
}
