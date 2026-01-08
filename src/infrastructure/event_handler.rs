use log::{info, warn};
use poise::serenity_prelude::{Context, FullEvent};

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
            let result = on_message(ctx, new_message, framework, data).await;
            if let Err(e) = result {
                warn!("Message handler produced an error: {:?}", e);
            }
        }
        FullEvent::GuildMemberAddition { new_member } => {
            let result = guild_member_add(new_member).await;
            if let Err(e) = result {
                warn!("Guild member added handler produced an error: {:?}", e);
            }
        }
        FullEvent::GuildMemberRemoval {
            guild_id,
            user,
            member_data_if_available,
        } => {
            let result = guild_member_remove(guild_id, user, member_data_if_available).await;
            if let Err(e) = result {
                warn!("Guild member removed handler produced an error: {:?}", e);
            }
        }
        _ => {}
    }
    Ok(())
}
