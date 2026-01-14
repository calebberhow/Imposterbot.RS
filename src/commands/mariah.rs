use poise::serenity_prelude::ChannelId;

use crate::{Context, Error};


#[poise::command(slash_command)]
pub async fn mariah(ctx: Context<'_>, channel: ChannelId) -> Result<(), Error> {
    let ch =channel.to_channel(ctx).await?;
    ch.
    Ok(())
}
