/*
    Handles saying hello and goodbye when members join and leave the guild.

    Adds specified role(s) to new members.
*/

use poise::{
    CreateReply,
    serenity_prelude::{
        ChannelId, Context, CreateAttachment, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
        GuildId, Member, RoleId, User, futures::future,
    },
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use tracing::{error, trace};

use crate::{
    Error, entities,
    infrastructure::{
        botdata::Data,
        colors,
        util::{get_media_directory, id_from_string, id_to_string, send_message_from_reply},
    },
};

async fn get_welcome_channel(db: &DatabaseConnection, guild_id: &GuildId) -> Option<ChannelId> {
    let query_result = entities::welcome_channel::Entity::find_by_id(id_to_string(*guild_id))
        .one(db)
        .await;

    match query_result.ok().flatten() {
        Some(model) => id_from_string::<ChannelId>(model.channel_id.as_str()).ok(),
        _ => None,
    }
}

pub async fn get_member_roles_on_join(
    db: &DatabaseConnection,
    guild_id: &GuildId,
) -> Option<Vec<RoleId>> {
    let query_result = entities::welcome_roles::Entity::find()
        .filter(entities::welcome_roles::Column::GuildId.eq(id_to_string(*guild_id)))
        .one(db)
        .await;

    match query_result {
        Ok(result) => Some(
            result
                .iter()
                .map(|role| id_from_string::<RoleId>(role.role_id.as_str()))
                .filter(|result| result.is_ok())
                .map(|result| result.expect("Failed results should have been filtered out"))
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
        get_welcome_channel(&data.db_pool, &member.guild_id),
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
    let channel = match get_welcome_channel(&data.db_pool, guild_id).await {
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
    match get_member_roles_on_join(&data.db_pool, &new_member.guild_id).await {
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
