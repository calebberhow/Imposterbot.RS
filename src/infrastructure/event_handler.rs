use std::time::Duration;

use poise::serenity_prelude::{Context, FullEvent};
use tracing::{debug, info, warn};

use crate::{
    Error,
    events::{
        guild_member::{guild_member_add, guild_member_remove},
        message::on_message,
    },
    infrastructure::botdata::Data,
};

pub async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Ready { data_about_bot, .. } => {
            info!("Bot is ready. Logged in as {}", data_about_bot.user.name);
        }
        FullEvent::Message { new_message } => {
            let result = on_message(ctx, framework, data, new_message).await;
            if let Err(e) = result {
                warn!("Message handler produced an error: {:?}", e);
            }
        }
        FullEvent::GuildMemberAddition { new_member } => {
            let result = guild_member_add(ctx, data, new_member).await;
            if let Err(e) = result {
                warn!("Guild member added handler produced an error: {:?}", e);
            }
        }
        FullEvent::GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available: _,
        } => {
            let result = guild_member_remove(ctx, data, guild_id, user).await;
            if let Err(e) = result {
                warn!("Guild member removed handler produced an error: {:?}", e);
            }
        }
        FullEvent::InteractionCreate { interaction } => {
            let ping = match framework
                .shard_manager
                .runners
                .lock()
                .await
                .get(&ctx.shard_id)
            {
                Some(runner) => runner.latency.unwrap_or(std::time::Duration::ZERO),
                None => {
                    tracing::error!(
                        "current shard is not in shard_manager.runners, this shouldn't happen"
                    );
                    std::time::Duration::ZERO
                }
            };
            if ping > Duration::default() {
                debug!(
                    "Ping measured for interaction type {:?}: {:?} ",
                    interaction.kind(),
                    ping
                )
            }
        }
        _ => {}
    }
    Ok(())
}
