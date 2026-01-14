use poise::samples::HelpConfiguration;

use crate::{Context, Error};

/// Registers/unregisters commands for this guild or all guilds.
#[poise::command(
    slash_command,
    prefix_command,
    aliases("refresh"),
    owners_only,
    hide_in_help
)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

/// Gets help on a command or all commands available.
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    track_deletion,
    hide_in_help
)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    poise::builtins::help(ctx, command.as_deref(), HelpConfiguration::default()).await?;
    Ok(())
}
