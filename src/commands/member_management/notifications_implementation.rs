use std::{path::Path, pin::Pin};

use poise::{
    CreateReply,
    serenity_prelude::{self as serenity, Attachment, GuildId},
};
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{NotSet, Set, Unchanged},
    EntityTrait, IntoActiveModel,
};
use tokio::io::AsyncWriteExt;
use tracing::{Level, error, trace, warn};
use uuid::Uuid;

use crate::{
    Context, Error, entities,
    events::guild_member::MemberNotificationFile,
    infrastructure::{
        environment::get_guild_user_content_directory,
        ids::{id_to_string, require_guild_id},
    },
};

#[derive(Debug)]
pub enum NotificationType {
    Join,
    Leave,
}

#[derive(Default, Debug)]
struct NotificationManagementRequest {
    pub content: Option<String>,
    pub clear_content: Option<bool>,
    pub description: Option<String>,
    pub clear_description: Option<bool>,
    pub thumbnail_file: Option<serenity::Attachment>,
    pub thumbnail_url: Option<String>,
    pub clear_thumbnail: Option<bool>,
    pub author: Option<String>,
    pub clear_author: Option<bool>,
    pub author_icon_file: Option<serenity::Attachment>,
    pub author_icon_url: Option<String>,
    pub clear_author_icon: Option<bool>,
    pub footer: Option<String>,
    pub clear_footer: Option<bool>,
    pub footer_icon_file: Option<serenity::Attachment>,
    pub footer_icon_url: Option<String>,
    pub clear_footer_icon: Option<bool>,
}

impl NotificationManagementRequest {
    fn required_content(mut self, value: Option<String>) -> Self {
        self.clear_content = Some(value.is_none());
        self.content = value;
        self
    }

    fn required_description(mut self, value: Option<String>) -> Self {
        self.clear_description = Some(value.is_none());
        self.description = value;
        self
    }

    fn required_thumbnail(
        mut self,
        file: Option<serenity::Attachment>,
        url: Option<String>,
    ) -> Self {
        self.clear_thumbnail = Some(file.is_none() && url.is_none());
        self.thumbnail_file = file;
        self.thumbnail_url = url;
        self
    }

    fn required_author(mut self, value: Option<String>) -> Self {
        self.clear_author = Some(value.is_none());
        self.author = value;
        self
    }

    fn required_author_icon(
        mut self,
        file: Option<serenity::Attachment>,
        url: Option<String>,
    ) -> Self {
        self.clear_author_icon = Some(file.is_none() && url.is_none());
        self.author_icon_file = file;
        self.author_icon_url = url;
        self
    }

    fn required_footer(mut self, value: Option<String>) -> Self {
        self.clear_footer = Some(value.is_none());
        self.footer = value;
        self
    }

    fn required_footer_icon(
        mut self,
        file: Option<serenity::Attachment>,
        url: Option<String>,
    ) -> Self {
        self.clear_footer_icon = Some(file.is_none() && url.is_none());
        self.footer_icon_file = file;
        self.footer_icon_url = url;
        self
    }
}

/// Returns default is `clear == Some(true)`, otherwise returns Value,
fn apply_clear<T>(value: Option<T>, clear: Option<bool>) -> Option<T>
where
    T: Default,
{
    if clear == Some(true) {
        Some(Default::default())
    } else {
        value
    }
}

fn get_file_attachment(
    url: Option<String>,
    file: &Option<serenity::Attachment>,
) -> Option<MemberNotificationFile> {
    file.as_ref()
        .map(|f| MemberNotificationFile {
            attachment: true,
            url: f.filename.clone(),
        })
        .or(url.map(|u| MemberNotificationFile {
            attachment: false,
            url: u,
        }))
}

async fn create_file_from_attachment_safe(
    guild_id: &GuildId,
    attachment: Attachment,
    files_added: &mut Vec<String>,
) -> Result<String, crate::Error> {
    #[derive(Debug)]
    enum CreateAttachmentFileError {
        DiscordApiError,
        FlushError(String, crate::Error),
        WriteError(String, crate::Error),
        CreateFileError(crate::Error),
    }
    async fn try_create_file(
        guild_id: &GuildId,
        attachment: Attachment,
    ) -> Result<String, CreateAttachmentFileError> {
        trace!("Creating file for attachment: {:?}", &attachment);
        let path = get_guild_user_content_directory(*guild_id);
        trace!("Ensuring user directory exists: {}", &path.display());
        tokio::fs::create_dir_all(&path)
            .await
            .map_err(|x| CreateAttachmentFileError::CreateFileError(x.into()))?;
        let guid = Uuid::new_v4();
        let ext = Path::new(&attachment.filename).extension();
        let random_filename = if let Some(x) = ext {
            format!("{}.{}", guid, x.display())
        } else {
            guid.to_string()
        };
        trace!("Downloading file attachment");
        let mut response = reqwest::get(attachment.url)
            .await
            .map_err(|_| CreateAttachmentFileError::DiscordApiError)?;
        if !response.status().is_success() {
            warn!("Discord returned non-success api response");
            return Err(CreateAttachmentFileError::DiscordApiError);
        }
        trace!("Response: {:?}", response);
        trace!(
            "Creating file: {} at path {}",
            &path.display(),
            &random_filename
        );
        let mut file = tokio::fs::File::create_new(&path.join(&random_filename))
            .await
            .map_err(|x| CreateAttachmentFileError::CreateFileError(x.into()))?;
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|x| CreateAttachmentFileError::WriteError(random_filename.clone(), x.into()))?
        {
            file.write_all(&chunk).await.map_err(|x| {
                CreateAttachmentFileError::WriteError(random_filename.clone(), x.into())
            })?;
        }
        file.flush().await.map_err(|x| {
            CreateAttachmentFileError::FlushError(random_filename.clone(), x.into())
        })?;
        Ok(random_filename)
    }

    match try_create_file(guild_id, attachment).await {
        Ok(file_name) => {
            files_added.push(file_name.clone());
            Ok(file_name)
        }
        Err(error) => {
            warn!("Failed to create file: {:?}", error);
            let remove_file = match &error {
                CreateAttachmentFileError::DiscordApiError => None,
                CreateAttachmentFileError::FlushError(f, _) => Some(f.clone()),
                CreateAttachmentFileError::WriteError(f, _) => Some(f.clone()),
                CreateAttachmentFileError::CreateFileError(_) => None,
            };

            if let Some(f) = remove_file {
                files_added.push(f);
            }
            for file in files_added {
                match tokio::fs::remove_file(file).await {
                    Err(e) => {
                        error!("Newly created file cannot be removed: {}", e);
                    }
                    _ => {}
                }
            }

            Err(match error {
                CreateAttachmentFileError::DiscordApiError => None,
                CreateAttachmentFileError::FlushError(_, e) => Some(e),
                CreateAttachmentFileError::WriteError(_, e) => Some(e),
                CreateAttachmentFileError::CreateFileError(e) => Some(e),
            }
            .unwrap_or(format!("Failed to save attachment.").into()))
        }
    }
}

/// Fully implements a notification management request, including database access, http requests for new attachments, and deletion of old attachments.
#[tracing::instrument(level = Level::TRACE, skip(ctx))]
async fn configure_member_notifications_impl(
    ctx: Context<'_>,
    r#type: NotificationType,
    request: NotificationManagementRequest,
) -> Result<(), Error> {
    fn active_model_file_attachment(
        is_file: sea_orm::ActiveValue<bool>,
        file_url: sea_orm::ActiveValue<String>,
    ) -> Option<String> {
        if !match is_file {
            Set(value) => value,
            Unchanged(value) => value,
            NotSet => false,
        } {
            return None;
        }

        match file_url {
            Set(value) => Some(value),
            Unchanged(value) => Some(value),
            NotSet => None,
        }
    }

    ctx.defer_ephemeral().await?;

    let guild_id = require_guild_id(ctx)?;
    let is_join = match r#type {
        NotificationType::Join => true,
        NotificationType::Leave => false,
    };
    let existing = entities::member_notification_message::Entity::find_by_id((
        id_to_string(guild_id),
        is_join,
    ))
    .one(&ctx.data().db_pool)
    .await?;

    let content = apply_clear(request.content, request.clear_content);
    let description = apply_clear(request.description, request.clear_description);
    let thumbnail = apply_clear(
        get_file_attachment(request.thumbnail_url, &request.thumbnail_file),
        request.clear_thumbnail,
    );
    let author = apply_clear(request.author, request.clear_author);
    let author_icon = apply_clear(
        get_file_attachment(request.author_icon_url, &request.author_icon_file),
        request.clear_author_icon,
    );
    let footer = apply_clear(request.footer, request.clear_footer);
    let footer_icon = apply_clear(
        get_file_attachment(request.footer_icon_url, &request.footer_icon_file),
        request.clear_footer_icon,
    );

    let mut files_to_delete: Vec<String> = vec![];
    let mut files_added: Vec<String> = vec![];
    let mut model = match existing {
        Some(row) => row.into_active_model(),
        None => entities::member_notification_message::ActiveModel {
            guild_id: Set(id_to_string(guild_id.clone())),
            join: Set(is_join),
            ..Default::default()
        },
    };

    if let Some(x) = content {
        model.content = Set(x);
    }

    if let Some(x) = description {
        model.description = Set(x);
    }

    if let Some(x) = thumbnail {
        if let Some(old_file) =
            active_model_file_attachment(model.thumbnail_is_file, model.thumbnail_url)
        {
            files_to_delete.push(old_file.clone());
        }

        model.thumbnail_is_file = Set(x.attachment);
        model.thumbnail_url = Set(x.url.clone());

        if x.attachment
            && !x.url.is_empty()
            && let Some(attachment) = request.thumbnail_file
        {
            match create_file_from_attachment_safe(&guild_id, attachment, &mut files_added).await {
                Ok(filename) => {
                    model.thumbnail_url = Set(filename);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    if let Some(x) = author {
        model.author = Set(x);
    }

    if let Some(x) = author_icon {
        if let Some(old_file) =
            active_model_file_attachment(model.author_icon_is_file, model.author_icon_url)
        {
            files_to_delete.push(old_file.clone());
        }

        model.author_icon_is_file = Set(x.attachment);
        model.author_icon_url = Set(x.url.clone());

        if x.attachment
            && !x.url.is_empty()
            && let Some(attachment) = request.author_icon_file
        {
            match create_file_from_attachment_safe(&guild_id, attachment, &mut files_added).await {
                Ok(filename) => {
                    model.author_icon_url = Set(filename);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    if let Some(x) = footer {
        model.footer = Set(x);
    }

    if let Some(x) = footer_icon {
        if let Some(old_file) =
            active_model_file_attachment(model.footer_icon_is_file, model.footer_icon_url)
        {
            files_to_delete.push(old_file.clone());
        }

        model.footer_icon_is_file = Set(x.attachment);
        model.footer_icon_url = Set(x.url.clone());

        if x.attachment
            && !x.url.is_empty()
            && let Some(attachment) = request.footer_icon_file
        {
            match create_file_from_attachment_safe(&guild_id, attachment, &mut files_added).await {
                Ok(filename) => {
                    model.footer_icon_url = Set(filename);
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    model.update(&ctx.data().db_pool).await?;

    // Delete old files from disk
    if !files_to_delete.is_empty() {
        let path = get_guild_user_content_directory(guild_id);
        let mut errors: Vec<std::io::Error> = vec![];
        for file in files_to_delete {
            match tokio::fs::remove_file(path.join(file)).await {
                Ok(_) => {}
                Err(e) => {
                    errors.push(e);
                }
            };
        }

        if !errors.is_empty() {
            let err_str = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<String>>()
                .join(", ");
            warn!(
                "Encountered errors attempting to remove user content files: {}",
                err_str
            );
        }
    }

    let notification_details = crate::events::guild_member::get_member_notification_details(
        &ctx.data().db_pool,
        &guild_id,
        is_join,
    )
    .await;

    match notification_details {
        Some(format) => {
            ctx.send(
                CreateReply::default()
                    .content("Successfully configured member notification message. Below is a sample of the new format:")
                    .ephemeral(true),
            )
            .await?;

            let guild = guild_id.to_partial_guild_with_counts(ctx).await; // TODO: this request is quite large and slow. Figure out how to more quickly retrieve the guild member count.
            let notification_details = if !is_join {
                crate::events::guild_member::MemberNotificationMessageDetails::for_user(
                    ctx.author(),
                    guild.ok(),
                    format,
                )
            } else {
                match ctx.author_member().await {
                    Some(member) => {
                        crate::events::guild_member::MemberNotificationMessageDetails::for_member(
                            &member,
                            guild.ok(),
                            format,
                        )
                    }
                    None => {
                        crate::events::guild_member::MemberNotificationMessageDetails::for_user(
                            ctx.author(),
                            guild.ok(),
                            format,
                        )
                    }
                }
            };

            let reply = notification_details
                .to_reply(&guild_id)
                .await
                .ephemeral(true);
            ctx.send(reply).await?;
        }
        None => {
            ctx.send(
                CreateReply::default()
                    .content("Successfully configured member notification message")
                    .ephemeral(true),
            )
            .await?;
        }
    }

    Ok(())
}

macro_rules! member_cmd_impl {
    ($fn_name:ident, $param_name:ident, $builder_method:ident) => {
        fn $fn_name<'a>(
            ctx: Context<'a>,
            $param_name: Option<String>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
            Box::pin(async move {
                configure_member_notifications_impl(
                    ctx,
                    Self::NOTIFICATION_TYPE,
                    NotificationManagementRequest::default().$builder_method($param_name),
                )
                .await
            })
        }
    };

    ($fn_name:ident, $param_name_1:ident, $param_name_2:ident, $builder_method:ident) => {
        fn $fn_name<'a>(
            ctx: Context<'a>,
            $param_name_1: Option<serenity::Attachment>,
            $param_name_2: Option<String>,
        ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
            Box::pin(async move {
                configure_member_notifications_impl(
                    ctx,
                    Self::NOTIFICATION_TYPE,
                    NotificationManagementRequest::default()
                        .$builder_method($param_name_1, $param_name_2),
                )
                .await
            })
        }
    };
}

/// Contains implementations for various notification modifier commands with variable notification type.
pub trait MemberEventConfigurer {
    const NOTIFICATION_TYPE: NotificationType;

    fn full_impl<'a>(
        ctx: Context<'a>,
        content: Option<String>,
        description: Option<String>,
        thumbnail_file: Option<serenity::Attachment>,
        thumbnail_url: Option<String>,
        author: Option<String>,
        author_icon_file: Option<serenity::Attachment>,
        author_icon_url: Option<String>,
        footer: Option<String>,
        footer_icon_file: Option<serenity::Attachment>,
        footer_icon_url: Option<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move {
            configure_member_notifications_impl(
                ctx,
                Self::NOTIFICATION_TYPE,
                NotificationManagementRequest::default()
                    .required_content(content)
                    .required_description(description)
                    .required_thumbnail(thumbnail_file, thumbnail_url)
                    .required_author(author)
                    .required_author_icon(author_icon_file, author_icon_url)
                    .required_footer(footer)
                    .required_footer_icon(footer_icon_file, footer_icon_url),
            )
            .await
        })
    }

    member_cmd_impl!(content_impl, content, required_content);
    member_cmd_impl!(description_impl, description, required_description);
    member_cmd_impl!(
        thumbnail_impl,
        thumbnail_file,
        thumbnail_url,
        required_thumbnail
    );
    member_cmd_impl!(author_impl, author, required_author);
    member_cmd_impl!(
        author_icon_impl,
        author_icon_file,
        author_icon_url,
        required_author_icon
    );
    member_cmd_impl!(footer_impl, footer, required_footer);
    member_cmd_impl!(
        footer_icon_impl,
        footer_icon_file,
        footer_icon_url,
        required_footer_icon
    );
}
