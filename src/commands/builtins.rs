use poise::samples::HelpConfiguration;

use crate::{Context, Error};

#[poise::command(prefix_command, aliases("refresh"), owners_only)]
pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
    poise::builtins::register_application_commands_buttons(ctx).await?;
    Ok(())
}

#[poise::command(slash_command, prefix_command, track_edits, track_deletion)]
pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
    poise::builtins::help(ctx, command.as_deref(), HelpConfiguration::default()).await?;
    Ok(())
}
