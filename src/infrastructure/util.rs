use std::fmt::Debug;

use poise::{
    CreateReply,
    serenity_prelude::{ChannelId, CreateMessage, Typing},
};
use tracing::trace;

use crate::{Context as ImposterbotContext, Error};

/// Creates a lazily initialized static regex variable with a constant regex expression.
#[macro_export]
macro_rules! lazy_regex {
    ($name:ident, $value:expr) => {
        static $name: once_cell::sync::Lazy<regex::Regex> =
            once_cell::sync::Lazy::new(|| regex::Regex::new($value).expect("Regex contains body"));
    };
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

/// Converts a `CreateReply` into a `CreateMessage` and sends it with the `ChannelId`.
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
    ctx: ImposterbotContext<'_>,
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
