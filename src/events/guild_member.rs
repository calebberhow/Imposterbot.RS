/*
    Handles saying hello and goodbye when members join and leave the guild.

    Adds specified role(s) to new members.
*/

use std::collections::HashMap;

use poise::{
    CreateReply,
    serenity_prelude::{
        ChannelId, Context, CreateAttachment, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter,
        CreateMessage, GuildId, Member, Mentionable, PartialGuild, RoleId, User, futures::future,
    },
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use strfmt::strfmt;
use tracing::{Level, error, trace};

use crate::{
    Error, entities,
    infrastructure::{
        botdata::Data,
        environment::get_data_directory,
        ids::{id_from_string, id_to_string},
    },
    record_member_fields,
};

enum MemberEvent<'a> {
    Join(&'a Member),
    Leave(&'a GuildId, &'a User),
}

#[derive(Default, Clone, Debug)]
pub struct MemberNotificationMessageDetails {
    pub content: Option<String>,
    pub embed: Option<MemberNotificationEmbedDetails>,
}

#[derive(Default, Clone, Debug)]
pub struct MemberNotificationEmbedDetails {
    pub description: Option<String>,
    pub thumbnail: Option<MemberNotificationFile>,
    pub author: Option<String>,
    pub author_icon_url: Option<MemberNotificationFile>,
    pub footer: Option<String>,
    pub footer_icon_url: Option<MemberNotificationFile>,
}

#[derive(Default, Clone, Debug)]
pub struct MemberNotificationFile {
    /// True if this file is sent as an attachment, or false if it is sent as a plain url.
    pub attachment: bool,
    /// If `attachment == true`, this is a filename,
    /// otherwise, this is a web address.
    pub url: String,
}

impl MemberNotificationMessageDetails {
    /// Defines a format which may be used to template instances for actual member events.
    pub fn format(
        content: Option<String>,
        embed: bool,
        description: Option<String>,
        thumbnail: Option<MemberNotificationFile>,
        author: Option<String>,
        author_icon_url: Option<MemberNotificationFile>,
        footer: Option<String>,
        footer_icon_url: Option<MemberNotificationFile>,
    ) -> Self {
        Self {
            content: content,
            embed: if embed {
                Some(MemberNotificationEmbedDetails {
                    description: description,
                    thumbnail: thumbnail,
                    author: author,
                    author_icon_url: author_icon_url,
                    footer: footer,
                    footer_icon_url: footer_icon_url,
                })
            } else {
                None
            },
        }
    }

    /// Defines message content for an actual member event for a given format.
    pub fn for_member(
        member: &Member,
        guild: Option<PartialGuild>,
        format: MemberNotificationMessageDetails,
    ) -> Self {
        let mut fmtargs = HashMap::<String, String>::new();
        fmtargs.insert("name".into(), member.user.name.clone());
        fmtargs.insert("mention".into(), member.mention().to_string());
        if let Some(avatar) = member.avatar_url().or(member.user.avatar_url()) {
            fmtargs.insert("user_avatar".into(), avatar);
        }
        if let Some(guild) = guild {
            if let Some(member_count) = guild.approximate_member_count {
                fmtargs.insert("member_count".into(), member_count.to_string());
            }
            if let Some(presence_count) = guild.approximate_presence_count {
                fmtargs.insert("online_member_count".into(), presence_count.to_string());
            }
        }

        Self::from_fmt_args(fmtargs, format)
    }

    pub fn for_user(
        user: &User,
        guild: Option<PartialGuild>,
        format: MemberNotificationMessageDetails,
    ) -> Self {
        let mut fmtargs = HashMap::<String, String>::new();
        fmtargs.insert("name".into(), user.name.clone());
        fmtargs.insert("rules".into(), "(Not yet implemented)".into());
        if let Some(avatar) = user.avatar_url() {
            fmtargs.insert("user_avatar".into(), avatar);
        }

        if let Some(guild) = guild {
            if let Some(member_count) = guild.approximate_member_count {
                fmtargs.insert("member_count".into(), member_count.to_string());
            }
            if let Some(presence_count) = guild.approximate_presence_count {
                fmtargs.insert("online_member_count".into(), presence_count.to_string());
            }
        }

        Self::from_fmt_args(fmtargs, format)
    }

    fn from_fmt_args(
        fmtargs: HashMap<String, String>,
        format: MemberNotificationMessageDetails,
    ) -> Self {
        fn get_string(fmt: Option<String>, args: &HashMap<String, String>) -> Option<String> {
            if let Some(content_fmt) = fmt {
                strfmt(&*content_fmt, &args).ok()
            } else {
                None
            }
        }

        fn get_attachment(
            fmt: Option<MemberNotificationFile>,
            args: &HashMap<String, String>,
        ) -> Option<MemberNotificationFile> {
            if let Some(content_fmt) = fmt {
                if let Some(formatted_url) = strfmt(&*content_fmt.url, &args).ok() {
                    Some(MemberNotificationFile {
                        attachment: content_fmt.attachment,
                        url: if content_fmt.attachment {
                            content_fmt.url // attachments cannot have format args (they are uploaded files)
                        } else {
                            formatted_url
                        },
                    })
                } else {
                    if content_fmt.attachment {
                        Some(MemberNotificationFile {
                            attachment: content_fmt.attachment,
                            url: content_fmt.url,
                        })
                    } else {
                        None
                    }
                }
            } else {
                None
            }
        }

        let content = get_string(format.content, &fmtargs);
        let embed: Option<MemberNotificationEmbedDetails> = if let Some(embd_fmt) = format.embed {
            Some(MemberNotificationEmbedDetails {
                description: get_string(embd_fmt.description, &fmtargs),
                thumbnail: get_attachment(embd_fmt.thumbnail, &fmtargs),
                author: get_string(embd_fmt.author, &fmtargs),
                author_icon_url: get_attachment(embd_fmt.author_icon_url, &fmtargs),
                footer: get_string(embd_fmt.footer, &fmtargs),
                footer_icon_url: get_attachment(embd_fmt.footer_icon_url, &fmtargs),
            })
        } else {
            None
        };

        Self {
            content: content,
            embed: embed,
        }
    }

    pub async fn to_embed(
        &self,
        guild_id: &GuildId,
    ) -> Option<(CreateEmbed, Vec<CreateAttachment>)> {
        if let Some(embed_details) = &self.embed {
            let mut embed = CreateEmbed::default();
            let mut attachments: Vec<CreateAttachment> = vec![];
            if let Some(x) = &embed_details.description {
                embed = embed.description(x);
            }

            if let Some(thumbnail_file) = &embed_details.thumbnail {
                if thumbnail_file.attachment {
                    match CreateAttachment::path(
                        get_data_directory()
                            .join("user_content")
                            .join(id_to_string(guild_id.clone()))
                            .join(&thumbnail_file.url),
                    )
                    .await
                    {
                        Ok(attachment) => {
                            embed = embed
                                .thumbnail(format!("attachment://{}", attachment.filename.clone()));
                            attachments.push(attachment);
                        }
                        Err(e) => {
                            error!(
                                "Attempted to create attachment with user content that does not exist: {:?}",
                                e
                            );
                        }
                    }
                } else {
                    embed = embed.thumbnail(&thumbnail_file.url);
                }
            }

            if let Some(x) = &embed_details.author {
                let mut author = CreateEmbedAuthor::new(x);
                if let Some(icon_file) = &embed_details.author_icon_url {
                    if icon_file.attachment {
                        match CreateAttachment::path(
                            get_data_directory()
                                .join("user_content")
                                .join(id_to_string(guild_id.clone()))
                                .join(&icon_file.url),
                        )
                        .await
                        {
                            Ok(attachment) => {
                                author = author.icon_url(format!(
                                    "attachment://{}",
                                    attachment.filename.clone()
                                ));
                                attachments.push(attachment);
                            }
                            Err(e) => {
                                error!(
                                    "Attempted to create attachment with user content that does not exist: {:?}",
                                    e
                                );
                            }
                        }
                    } else {
                        author = author.icon_url(&icon_file.url);
                    }
                }

                embed = embed.author(author);
            }

            if let Some(x) = &embed_details.footer {
                let mut footer = CreateEmbedFooter::new(x);

                if let Some(icon_file) = &embed_details.footer_icon_url {
                    if icon_file.attachment {
                        match CreateAttachment::path(
                            get_data_directory()
                                .join("user_content")
                                .join(id_to_string(guild_id.clone()))
                                .join(&icon_file.url),
                        )
                        .await
                        {
                            Ok(attachment) => {
                                footer = footer.icon_url(format!(
                                    "attachment://{}",
                                    attachment.filename.clone()
                                ));
                                attachments.push(attachment);
                            }
                            Err(e) => {
                                error!(
                                    "Attempted to create attachment with user content that does not exist: {:?}",
                                    e
                                );
                            }
                        }
                    } else {
                        footer = footer.icon_url(&icon_file.url);
                    }
                }

                embed = embed.footer(footer);
            }

            Some((embed, attachments))
        } else {
            None
        }
    }

    pub async fn to_message(&self, guild_id: &GuildId) -> CreateMessage {
        let mut message = CreateMessage::default();
        if let Some(x) = &self.content {
            message = message.content(x);
        }
        let embed_opt = self.to_embed(guild_id).await;

        if let Some(embd_and_attachments) = embed_opt {
            message = message.embed(embd_and_attachments.0);
            message = message.add_files(embd_and_attachments.1);
        }

        message
    }

    pub async fn to_reply(&self, guild_id: &GuildId) -> CreateReply {
        let mut reply = CreateReply::default();
        if let Some(x) = &self.content {
            reply = reply.content(x);
        }
        let embed_opt = self.to_embed(guild_id).await;

        if let Some(embd_and_attachments) = embed_opt {
            reply = reply.embed(embd_and_attachments.0);
            for attachment in embd_and_attachments.1 {
                reply = reply.attachment(attachment);
            }
        }

        reply
    }
}

pub async fn get_member_notification_details(
    db: &DatabaseConnection,
    guild_id: &GuildId,
    join: bool,
) -> Option<MemberNotificationMessageDetails> {
    fn optional_string(string: String) -> Option<String> {
        if string.is_empty() {
            None
        } else {
            Some(string)
        }
    }

    fn optional_attachment(file: bool, url: String) -> Option<MemberNotificationFile> {
        if url.is_empty() {
            None
        } else {
            Some(MemberNotificationFile {
                attachment: file,
                url: url,
            })
        }
    }

    match entities::member_notification_message::Entity::find_by_id((id_to_string(*guild_id), join))
        .one(db)
        .await
    {
        Ok(model) => model.map(|model| {
            MemberNotificationMessageDetails::format(
                optional_string(model.content),
                !model.description.is_empty()
                    || !model.author.is_empty()
                    || !model.footer.is_empty()
                    || !model.thumbnail_url.is_empty(),
                optional_string(model.description),
                optional_attachment(model.thumbnail_is_file, model.thumbnail_url),
                optional_string(model.author),
                optional_attachment(model.author_icon_is_file, model.author_icon_url),
                optional_string(model.footer),
                optional_attachment(model.footer_icon_is_file, model.footer_icon_url),
            )
        }),
        Err(err) => {
            error!(
                "An error occurred while fetching member notification message: {}",
                err
            );
            None
        }
    }
}

async fn get_member_notification_channel(
    db: &DatabaseConnection,
    guild_id: &GuildId,
    join: bool,
) -> Option<ChannelId> {
    let query_result =
        entities::member_notification_channel::Entity::find_by_id((id_to_string(*guild_id), join))
            .one(db)
            .await;

    match query_result {
        Ok(model) => model
            .map(
                |model| match id_from_string::<ChannelId>(model.channel_id.as_str()) {
                    Ok(id) => Some(id),
                    Err(error) => {
                        error!(
                            "Error occurred while parsing member notification channel: {}. Value: {}",
                            error,
                            model.channel_id
                        );
                        None
                    }
                },
            )
            .flatten(),
        Err(error) => {
            error!(
                "Error occurred while getting member notification channel: {}",
                error
            );
            None
        }
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

async fn notify_member_event(
    ctx: &Context,
    data: &Data,
    event: MemberEvent<'_>,
) -> Result<(), Error> {
    let guild_id = match event {
        MemberEvent::Join(member) => &member.guild_id,
        MemberEvent::Leave(guild_id, _) => guild_id,
    };
    let join = match event {
        MemberEvent::Join(_) => true,
        MemberEvent::Leave(_, _) => false,
    };
    let (channel, format, guild) = future::join3(
        get_member_notification_channel(&data.db_pool, guild_id, join),
        get_member_notification_details(&data.db_pool, guild_id, join),
        guild_id.to_partial_guild_with_counts(ctx), // TODO: this request is quite large and slow. Figure out how to more quickly retrieve the guild member count.
    )
    .await;

    let channel = match channel {
        Some(x) => x,
        None => return Ok(()), // Notification channel not confiugred on this guild.
    };

    let format = match format {
        Some(x) => x,
        None => return Ok(()), // Notification message not configured on this guild.
    };

    let content = match event {
        MemberEvent::Join(member) => {
            MemberNotificationMessageDetails::for_member(member, guild.ok(), format)
        }
        MemberEvent::Leave(_, user) => {
            MemberNotificationMessageDetails::for_user(user, guild.ok(), format)
        }
    };

    trace!("Member event content: {:?}", content);
    let reply = content.to_message(&guild_id).await;
    channel.send_message(ctx, reply).await?;
    Ok(())
}

#[tracing::instrument(level = Level::DEBUG, err(level = Level::WARN), skip_all)]
async fn add_initial_member_roles(
    ctx: &Context,
    data: &Data,
    new_member: &Member,
) -> Result<(), Error> {
    match get_member_roles_on_join(&data.db_pool, &new_member.guild_id).await {
        Some(roles) => match new_member.add_roles(ctx, &roles).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        },
        None => Ok(()),
    }
}

#[tracing::instrument(level = tracing::Level::INFO, err(level = tracing::Level::WARN), skip_all, fields(user = tracing::field::Empty, guild_id = tracing::field::Empty))]
pub async fn guild_member_add(
    ctx: &Context,
    data: &Data,
    new_member: &Member,
) -> Result<(), Error> {
    record_member_fields!(new_member);
    if let Err(e) = notify_member_event(ctx, data, MemberEvent::Join(new_member)).await {
        error!("Failed to welcome new member: {}", e)
    }
    if let Err(e) = add_initial_member_roles(ctx, data, new_member).await {
        error!("Failed to add roles to new member: {}", e)
    }
    Ok(())
}

#[tracing::instrument(level = tracing::Level::INFO, err(level = tracing::Level::WARN), skip_all, fields(user = tracing::field::Empty, guild_id = tracing::field::Empty))]
pub async fn guild_member_remove(
    ctx: &Context,
    data: &Data,
    guild_id: &GuildId,
    user: &User,
) -> Result<(), Error> {
    record_member_fields!(user, guild_id);
    if let Err(e) = notify_member_event(ctx, data, MemberEvent::Leave(guild_id, user)).await {
        error!("Failed to welcome member leave: {}", e)
    }
    Ok(())
}
