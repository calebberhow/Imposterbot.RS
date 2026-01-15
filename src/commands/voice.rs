use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    Context, Error,
    infrastructure::{environment::get_media_directory, ids::require_guild_id},
};
use poise::CreateReply;
use poise::serenity_prelude::futures::{Stream, StreamExt};
use poise::serenity_prelude::prelude::TypeMapKey;
use poise::serenity_prelude::{ChannelId, CreateEmbed};
use poise::serenity_prelude::{CreateEmbedAuthor, GuildId};
use poise::serenity_prelude::{async_trait, futures};
use songbird::error::JoinError;
use songbird::events::{Event, EventContext, EventHandler as VoiceEventHandler, TrackEvent};
use songbird::input::{AuxMetadata, Compose, YoutubeDl};
use songbird::tracks::TrackHandle;
use tracing::trace;
use tracing::warn;
use tracing::{debug, error};

/// Set of commands to play/stop playing audio in voice channel
#[poise::command(
    slash_command,
    subcommands("mariah", "stop", "youtube"),
    required_permissions = "USE_SOUNDBOARD",
    default_member_permissions = "USE_SOUNDBOARD"
)]
pub async fn play(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn mariah(ctx: Context<'_>, channel: Option<ChannelId>) -> Result<(), Error> {
    let file = get_media_directory().join("opus").join("mariah.opus");
    let guild_id = require_guild_id(ctx)?;
    let channel_id = match channel {
        Some(x) => Ok(x),
        None => {
            let voice_state = guild_id
                .get_user_voice_state(&ctx.serenity_context().http, ctx.author().id)
                .await?;

            voice_state
                .channel_id
                .ok_or::<Error>("You must specify a channel or be in a voice channel.".into())
        }
    }?;

    let voice_manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice Client registered at startup")
        .clone();

    match voice_manager.join(guild_id, channel_id).await {
        Ok(_) => match play_from_file(ctx, file).await {
            Ok(track) => {
                track.add_event(
                    Event::Track(TrackEvent::End),
                    TrackEndNotifier {
                        guild_id,
                        manager: voice_manager.clone(),
                    },
                )?;
                ctx.send(
                    CreateReply::default()
                        .content("Playing mariah carey!")
                        .ephemeral(true)
                        .reply(true),
                )
                .await?;
            }
            Err(play_err) => {
                warn!(
                    guild_id = guild_id.get(),
                    channel_id = channel_id.get(),
                    "Voice manager had an error attempting to play mariah carey: {:?}",
                    play_err
                );
                ctx.send(
                    CreateReply::default()
                        .content("Cannot play mariah carey... :(")
                        .ephemeral(true)
                        .reply(true),
                )
                .await?;
            }
        },
        Err(join_err) => {
            warn!(
                guild_id = guild_id.get(),
                channel_id = channel_id.get(),
                "Voice manager had an error while joining channel: {:?}",
                join_err
            );
            ctx.send(
                CreateReply::default()
                    .content("Cannot join channel...")
                    .ephemeral(true)
                    .reply(true),
            )
            .await?;
        }
    }
    Ok(())
}

async fn youtube_search_autocomplete<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> impl Stream<Item = String> + 'a {
    // Get guild id
    debug!(
        partial = partial,
        "youtube_search_autocomplete executed with args"
    );

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let mut query = YoutubeDl::new_search(http_client, partial).user_args(vec![
        "--flat-playlist".into(),
        "--skip-download".into(),
        "--quiet".into(),
        "--ignore-errors".into(),
    ]);
    let results = query.search(Some(5)).await;

    match results {
        Ok(results) => futures::stream::iter(results.filter_map(|x| x.title.or(x.track)))
            .inspect(|x| trace!("Produced autocomplete value: {}", x))
            .boxed(),
        Err(_) => futures::stream::empty().boxed(),
    }
}

#[poise::command(slash_command, guild_only)]
pub async fn youtube(
    ctx: Context<'_>,
    #[autocomplete = "youtube_search_autocomplete"] video: String,
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;
    let guild_id = require_guild_id(ctx)?;
    let channel_id = match channel {
        Some(x) => Ok(x),
        None => {
            let voice_state = guild_id
                .get_user_voice_state(&ctx.serenity_context().http, ctx.author().id)
                .await?;

            voice_state
                .channel_id
                .ok_or::<Error>("You must specify a channel or be in a voice channel.".into())
        }
    }?;

    let voice_manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice Client registered at startup")
        .clone();

    match voice_manager.join(guild_id, channel_id).await {
        Ok(_) => match play_from_youtube(ctx, video.into()).await {
            Ok((meta, track)) => {
                track.add_event(
                    Event::Track(TrackEvent::End),
                    TrackEndNotifier {
                        guild_id,
                        manager: voice_manager.clone(),
                    },
                )?;
                let reply = match meta {
                    Some(meta) => CreateReply::default().embed(get_track_embed(meta)),
                    None => CreateReply::default().content("Playing from youtube"),
                };
                ctx.send(reply.ephemeral(true).reply(true)).await?;
            }
            Err(play_err) => {
                warn!(
                    guild_id = guild_id.get(),
                    channel_id = channel_id.get(),
                    "Voice manager had an error attempting to play video: {:?}",
                    play_err
                );
                ctx.send(
                    CreateReply::default()
                        .content("Cannot play video... :(")
                        .ephemeral(true)
                        .reply(true),
                )
                .await?;
            }
        },
        Err(join_err) => {
            warn!(
                guild_id = guild_id.get(),
                channel_id = channel_id.get(),
                "Voice manager had an error while joining channel: {:?}",
                join_err
            );
            ctx.send(
                CreateReply::default()
                    .content("Cannot join channel...")
                    .ephemeral(true)
                    .reply(true),
            )
            .await?;
        }
    }
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    let voice_manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice Client registered at startup")
        .clone();
    let guild_id = require_guild_id(ctx)?;
    match voice_manager.remove(guild_id).await {
        Ok(_) => Ok::<(), Error>(()),
        Err(join_error) => match join_error {
            JoinError::NoCall => {
                ctx.send(
                    CreateReply::default()
                        .content("I am not in any voice channel...")
                        .ephemeral(true)
                        .reply(true),
                )
                .await?;
                return Ok(());
            }
            e => Err(e.into()),
        },
    }?;

    ctx.send(
        CreateReply::default()
            .content("Stopping!")
            .ephemeral(true)
            .reply(true),
    )
    .await?;

    Ok(())
}

async fn play_from_youtube(
    ctx: Context<'_>,
    url: String,
) -> Result<(Option<AuxMetadata>, TrackHandle), Error> {
    let guild_id = require_guild_id(ctx)?;
    let do_search = !url.starts_with("http");

    let http_client = {
        let data = ctx.serenity_context().data.read().await;
        data.get::<HttpKey>()
            .cloned()
            .expect("Guaranteed to exist in the typemap.")
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);

        let mut meta_src = if do_search {
            YoutubeDl::new_search(http_client.clone(), url.clone())
        } else {
            YoutubeDl::new(http_client.clone(), url.clone())
        };
        let play_src = if do_search {
            YoutubeDl::new_search(http_client, url)
        } else {
            YoutubeDl::new(http_client, url)
        };

        let res = tokio::join!(async { meta_src.aux_metadata().await.ok() }, async {
            handler.play_only_input(play_src.into())
        });
        Ok(res)
    } else {
        Err("Not in voice channel".into())
    }
}

fn get_track_embed(metadata: AuxMetadata) -> CreateEmbed {
    let mut embd =
        CreateEmbed::default().title(metadata.track.or(metadata.title).unwrap_or_default());
    if let Some(x) = metadata.thumbnail {
        embd = embd.thumbnail(x);
    }

    if let Some(x) = metadata.source_url {
        embd = embd.url(x);
    }

    if let Some(x) = metadata.artist.or(metadata.channel) {
        embd = embd.author(CreateEmbedAuthor::new(x));
    }

    embd
}

async fn play_from_file(ctx: Context<'_>, file: PathBuf) -> Result<TrackHandle, Error> {
    let guild_id = require_guild_id(ctx)?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        handler.add_global_event(TrackEvent::Error.into(), TrackErrorNotifier);
        let source = songbird::input::File::new(file);
        Ok(handler.play_only_input(source.into()))
    } else {
        Err("Not in voice channel".into())
    }
}

struct TrackErrorNotifier;

#[async_trait]
impl VoiceEventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(track_list) = ctx {
            for (state, handle) in *track_list {
                error!(
                    "Track {:?} encountered an error: {:?}",
                    handle.uuid(),
                    state.playing
                );
            }
        }

        None
    }
}

pub struct HttpKey;

impl TypeMapKey for HttpKey {
    type Value = reqwest::Client;
}

struct TrackEndNotifier {
    guild_id: GuildId,
    manager: Arc<songbird::Songbird>,
}

#[async_trait]
impl VoiceEventHandler for TrackEndNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        trace!("Track end event fired");
        if let EventContext::Track(track_list) = ctx {
            // This fires when the track finishes naturally
            if let Some((_state, _handle)) = track_list.first() {
                if let Some(handler_lock) = self.manager.get(self.guild_id) {
                    let handler = handler_lock.lock().await;

                    // Only leave if nothing else is playing
                    if handler.queue().is_empty() {
                        trace!("Queue is empty.. leaving voice channel.");
                        drop(handler); // lock must be released before calling remove...
                        match self.manager.remove(self.guild_id).await {
                            Err(err) => {
                                error!("Failed to leave voice channel: {:?}", err)
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        None
    }
}
