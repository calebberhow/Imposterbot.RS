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

/// Fills user / guild_id / channel_id fields
#[macro_export]
macro_rules! record_ctx_fields {
    ($ctx:expr) => {{
        let span = tracing::Span::current();
        span.record("user", $ctx.author().name.as_str());
        span.record("guild_id", $ctx.guild_id().map(|g| g.get()));
        span.record("channel_id", $ctx.channel_id().get());
    }};
}

/// Fills user / guild_id / channel_id fields
#[macro_export]
macro_rules! record_member_fields {
    ($member:expr) => {{
        let span = tracing::Span::current();
        span.record("user", $member.user.name.as_str());
        span.record("guild_id", $member.guild_id.get());
    }};
    ($user:expr, $guild:expr) => {{
        let span = tracing::Span::current();
        span.record("user", $user.name.as_str());
        span.record("guild_id", $guild.get());
    }};
}

/// Attach standard user/guild/channel fields to a span for a command
#[macro_export]
macro_rules! poise_instrument {
    ($fn:item) => {
        #[tracing::instrument(level = tracing::Level::INFO, err(level = tracing::Level::WARN), skip(ctx), fields(user = tracing::field::Empty, guild_id = tracing::field::Empty, channel_id = tracing::field::Empty))]
        $fn
    };
    ( $( $fn:item )+ ) => {
        $(
            #[tracing::instrument(level = tracing::Level::INFO, err(level = tracing::Level::WARN), skip(ctx), fields(user = tracing::field::Empty, guild_id = tracing::field::Empty, channel_id = tracing::field::Empty))]
            $fn
        )+
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
