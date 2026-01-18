/*!

Implements the member join / member leave notifications feature, abstracted away from the poise declarations, allowing for code de-duplication.

Without this layer of abstraction, every single function was duplicated twice (once for join and once for leave), with few options to reduce complexity.

*/

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
    Context, Error,
    entities::{self, member_notification_message},
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

#[derive(Default, Debug, Clone)]
enum OptionalClearable<T> {
    /// Ignored
    #[default]
    None,
    /// Represents a default value of T
    Clear,
    /// Some value of T
    Some(T),
}

impl<T> Into<Option<T>> for OptionalClearable<T>
where
    T: Default,
{
    fn into(self) -> Option<T> {
        match self {
            Self::Some(value) => Some(value),
            Self::Clear => Some(T::default()),
            Self::None => None,
        }
    }
}

impl<T> From<Option<T>> for OptionalClearable<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Some(value),
            None => Self::Clear,
        }
    }
}

#[derive(Debug, Clone)]
enum EmbedAttachment {
    URL(String),
    File(serenity::Attachment),
}

impl EmbedAttachment {
    fn is_file(&self) -> bool {
        match self {
            Self::File(_) => true,
            _ => false,
        }
    }

    async fn get_url_and_create_attachment(
        self,
        guild_id: &GuildId,
        files_added: &mut Vec<String>,
    ) -> Result<String, crate::Error> {
        match self {
            EmbedAttachment::URL(u) => Ok(u),
            EmbedAttachment::File(f) => {
                match create_file_from_attachment_safe(&guild_id, f, files_added).await {
                    Ok(filename) => Ok(filename),
                    Err(e) => {
                        return Err(e);
                    }
                }
            }
        }
    }
}

impl Default for EmbedAttachment {
    fn default() -> Self {
        Self::URL(String::default())
    }
}

/// Contains all of the data required to add or modify a member join or member leave notification.
/// Every field contians
#[derive(Default, Debug, Clone)]
struct NotificationManagementRequest {
    pub content: OptionalClearable<String>,
    pub title: OptionalClearable<String>,
    pub description: OptionalClearable<String>,
    pub thumbnail: OptionalClearable<EmbedAttachment>,
    pub image: OptionalClearable<EmbedAttachment>,
    pub author: OptionalClearable<String>,
    pub author_icon: OptionalClearable<EmbedAttachment>,
    pub footer: OptionalClearable<String>,
    pub footer_icon: OptionalClearable<EmbedAttachment>,
}

impl NotificationManagementRequest {
    fn content(mut self, value: Option<String>) -> Self {
        self.content = value.into();
        self
    }

    fn title(mut self, value: Option<String>) -> Self {
        self.title = value.into();
        self
    }

    fn description(mut self, value: Option<String>) -> Self {
        self.description = value.into();
        self
    }

    fn thumbnail(mut self, file: Option<serenity::Attachment>, url: Option<String>) -> Self {
        self.thumbnail = file
            .map(|f| EmbedAttachment::File(f))
            .or(url.map(|u| EmbedAttachment::URL(u)))
            .into();
        self
    }

    fn image(mut self, file: Option<serenity::Attachment>, url: Option<String>) -> Self {
        self.image = file
            .map(|f| EmbedAttachment::File(f))
            .or(url.map(|u| EmbedAttachment::URL(u)))
            .into();
        self
    }

    fn author(mut self, value: Option<String>) -> Self {
        self.author = value.into();
        self
    }

    fn author_icon(mut self, file: Option<serenity::Attachment>, url: Option<String>) -> Self {
        self.author_icon = file
            .map(|f| EmbedAttachment::File(f))
            .or(url.map(|u| EmbedAttachment::URL(u)))
            .into();
        self
    }

    fn footer(mut self, value: Option<String>) -> Self {
        self.footer = value.into();
        self
    }

    fn footer_icon(mut self, file: Option<serenity::Attachment>, url: Option<String>) -> Self {
        self.footer_icon = file
            .map(|f| EmbedAttachment::File(f))
            .or(url.map(|u| EmbedAttachment::URL(u)))
            .into();
        self
    }
}

/// Creates a file on disk for an attachment submitted via discord API, then returns the name of the newly created file.
///
/// This method is 'safe', as in it ensures that any files created (including previous files which can be input with [`files_added`]) are cleaned up if an error occurs.
///
/// Since a discord attachment only contains a url to the content hosted on the discord CDN, this function will perform an HTTP request to download the content and write it to disk.
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
#[tracing::instrument(level = Level::TRACE, err(level = Level::WARN), skip(ctx))]
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

    let mut files_to_delete: Vec<String> = vec![];
    let mut files_added: Vec<String> = vec![];
    let (mut model, update) = match existing {
        Some(row) => (row.into_active_model(), true),
        None => (
            entities::member_notification_message::ActiveModel {
                guild_id: Set(id_to_string(guild_id.clone())),
                join: Set(is_join),
                ..Default::default()
            },
            false,
        ),
    };

    if let Option::<String>::Some(x) = request.content.into() {
        model.content = Set(x.replace("\\n", "\n"));
    }

    if let Option::<String>::Some(x) = request.title.into() {
        model.title = Set(x.replace("\\n", "\n"));
    }

    if let Option::<String>::Some(x) = request.description.into() {
        model.description = Set(x.replace("\\n", "\n"));
    }

    if let Option::<EmbedAttachment>::Some(x) = request.thumbnail.into() {
        if let Some(old_file) =
            active_model_file_attachment(model.thumbnail_is_file, model.thumbnail_url)
        {
            files_to_delete.push(old_file);
        }

        model.thumbnail_is_file = Set(x.is_file());
        model.thumbnail_url = Set(x
            .get_url_and_create_attachment(&guild_id, &mut files_added)
            .await?);
    }

    if let Option::<EmbedAttachment>::Some(x) = request.image.into() {
        if let Some(old_file) = active_model_file_attachment(model.image_is_file, model.image_url) {
            files_to_delete.push(old_file);
        }

        model.image_is_file = Set(x.is_file());
        model.image_url = Set(x
            .get_url_and_create_attachment(&guild_id, &mut files_added)
            .await?)
    }

    if let Option::<String>::Some(x) = request.author.into() {
        model.author = Set(x.replace("\\n", "\n"));
    }

    if let Option::<EmbedAttachment>::Some(x) = request.author_icon.into() {
        if let Some(old_file) =
            active_model_file_attachment(model.author_icon_is_file, model.author_icon_url)
        {
            files_to_delete.push(old_file);
        }

        model.author_icon_is_file = Set(x.is_file());
        model.author_icon_url = Set(x
            .get_url_and_create_attachment(&guild_id, &mut files_added)
            .await?)
    }

    if let Option::<String>::Some(x) = request.footer.into() {
        model.footer = Set(x.replace("\\n", "\n"));
    }

    if let Option::<EmbedAttachment>::Some(x) = request.footer_icon.into() {
        if let Some(old_file) =
            active_model_file_attachment(model.footer_icon_is_file, model.footer_icon_url)
        {
            files_to_delete.push(old_file);
        }

        model.footer_icon_is_file = Set(x.is_file());
        model.footer_icon_url = Set(x
            .get_url_and_create_attachment(&guild_id, &mut files_added)
            .await?)
    }

    if update {
        model.update(&ctx.data().db_pool).await?;
    } else {
        member_notification_message::Entity::insert(model)
            .exec(&ctx.data().db_pool)
            .await?;
    }

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
        title: Option<String>,
        description: Option<String>,
        thumbnail_file: Option<serenity::Attachment>,
        thumbnail_url: Option<String>,
        image_file: Option<serenity::Attachment>,
        image_url: Option<String>,
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
                    .content(content)
                    .title(title)
                    .description(description)
                    .thumbnail(thumbnail_file, thumbnail_url)
                    .image(image_file, image_url)
                    .author(author)
                    .author_icon(author_icon_file, author_icon_url)
                    .footer(footer)
                    .footer_icon(footer_icon_file, footer_icon_url),
            )
            .await
        })
    }

    member_cmd_impl!(content_impl, content, content);
    member_cmd_impl!(title_impl, description, title);
    member_cmd_impl!(description_impl, description, description);
    member_cmd_impl!(thumbnail_impl, thumbnail_file, thumbnail_url, thumbnail);
    member_cmd_impl!(image_impl, image_file, image_url, image);
    member_cmd_impl!(author_impl, author, author);
    member_cmd_impl!(
        author_icon_impl,
        author_icon_file,
        author_icon_url,
        author_icon
    );
    member_cmd_impl!(footer_impl, footer, footer);
    member_cmd_impl!(
        footer_icon_impl,
        footer_icon_file,
        footer_icon_url,
        footer_icon
    );
}
