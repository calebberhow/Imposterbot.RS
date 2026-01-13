use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use poise::{
    CreateReply,
    serenity_prelude::{ChannelId, CreateMessage, GuildId, Typing},
};
use tracing::trace;

use crate::{Context, Error, infrastructure::environment};

#[macro_export]
macro_rules! lazy_regex {
    ($name:ident, $value:expr) => {
        static $name: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new($value).expect("Regex contains body"));
    };
}

pub fn get_data_directory() -> PathBuf {
    let st: String =
        std::env::var(environment::DATA_DIRECTORY).unwrap_or_else(|_| "./data".to_string());
    Path::new(st.as_str()).to_owned()
}

pub fn get_media_directory() -> PathBuf {
    let st: String =
        std::env::var(environment::MEDIA_DIRECTORY).unwrap_or_else(|_| "./media".to_string());
    Path::new(st.as_str()).to_owned()
}

pub struct DebuggableReply(CreateReply);

#[derive(Clone, Debug, PartialEq)]
struct AttachmentMetadata {
    filename: String,
    description: Option<String>,
}

impl DebuggableReply {
    pub fn new(value: &CreateReply) -> Self {
        Self(value.clone())
    }

    fn get_attachments(&self) -> Vec<AttachmentMetadata> {
        self.0
            .attachments
            .iter()
            .map(|attachment| AttachmentMetadata {
                filename: attachment.filename.clone(),
                description: attachment.description.clone(),
            })
            .collect()
    }
}

impl Debug for DebuggableReply {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CreateReply")
            .field("content", &self.0.content)
            .field("embeds", &self.0.embeds)
            .field("attachments", &self.get_attachments())
            .field("ephemeral", &self.0.ephemeral)
            .field("components", &self.0.components)
            .field("allowed_mentions", &self.0.allowed_mentions)
            .field("reply", &self.0.reply)
            .finish()
    }
}

pub fn require_guild_id(ctx: Context<'_>) -> Result<GuildId, Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This function is only available in guilds")?;
    trace!("Found guild_id={:?}", guild_id);
    Ok(guild_id)
}

pub fn lossless_u64_to_i64(value: u64) -> i64 {
    i64::from_le_bytes(value.to_le_bytes())
}

pub fn lossless_i64_to_u64(value: i64) -> u64 {
    u64::from_le_bytes(value.to_le_bytes())
}

pub async fn send_message_from_reply(
    channel: &ChannelId,
    ctx: &poise::serenity_prelude::Context,
    reply: CreateReply,
) -> Result<(), Error> {
    let debuggable = DebuggableReply::new(&reply);
    let mut create_message = CreateMessage::new()
        .embeds(reply.embeds)
        .add_files(reply.attachments);
    if let Some(x) = reply.content {
        create_message = create_message.content(x);
    }
    if let Some(x) = reply.allowed_mentions {
        create_message = create_message.allowed_mentions(x);
    }
    if let Some(x) = reply.components {
        create_message = create_message.components(x);
    }

    trace!("Sending message: {:?}", debuggable);
    channel.send_message(ctx, create_message).await?;
    Ok(())
}

/// Appropriately indicates to the end user that imposterbot is working on a response.
/// - For Application (/) commands, this is a message in response to the interation that says "Imposterbot is thinking..."
/// - For prefix commands, this is indicated by "Imposterbot is typing" hint, as if a real person is typing a message.
///
/// Note:
/// When the returned result goes out of scope, is dropped, or Typing.stop() is called, the typing hint will disappear.
pub async fn defer_or_broadcast(
    ctx: Context<'_>,
    ephemeral: bool,
) -> Result<Option<Typing>, Error> {
    match ctx {
        poise::Context::Application(appctx) => {
            appctx.defer_response(ephemeral).await?;
            Ok(None)
        }
        poise::Context::Prefix(prefixctx) => Ok(Some(
            prefixctx
                .msg
                .channel_id
                .start_typing(&prefixctx.serenity_context.http),
        )),
    }
}
