use std::num::ParseIntError;

use poise::serenity_prelude::{GuildId, UserId};
use tracing::trace;

use crate::{Context, Error};

pub const KHAZAARI_ID: UserId = UserId::new(193136312759353344);
pub const CRESSY_ID: UserId = UserId::new(318195473364156419);

pub fn require_guild_id(ctx: Context<'_>) -> Result<GuildId, Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or("This function is only available in guilds")?;
    trace!("Found guild_id={:?}", guild_id);
    Ok(guild_id)
}

pub fn id_to_string<T>(value: T) -> String
where
    T: Into<u64>,
{
    let int: u64 = value.into();
    int.to_string()
}

pub fn id_from_string<T>(value: &str) -> Result<T, ParseIntError>
where
    T: From<u64>,
{
    value.parse::<u64>().map(|int| T::from(int))
}
