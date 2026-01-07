use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use log::trace;
use poise::{
    CreateReply,
    serenity_prelude::{CreateMessage, GuildId},
};

use crate::{Context, Error, infrastructure::environment};

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
pub struct DebuggableMessage(CreateMessage);

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
