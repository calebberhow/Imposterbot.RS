use poise::samples::HelpConfiguration;

use crate::{Context, Error, poise_instrument, record_ctx_fields};

poise_instrument! {
    /// Registers/unregisters commands for this guild or all guilds.
    #[poise::command(
        slash_command,
        prefix_command,
        aliases("refresh"),
        owners_only,
        hide_in_help
    )]
    pub async fn register(ctx: Context<'_>) -> Result<(), Error> {
        record_ctx_fields!(ctx);
        poise::builtins::register_application_commands_buttons(ctx).await?;
        Ok(())
    }
}

poise_instrument! {
    /// Gets help on a command or all commands available.
    #[poise::command(
        slash_command,
        prefix_command,
        track_edits,
        track_deletion,
        hide_in_help
    )]
    pub async fn help(ctx: Context<'_>, command: Option<String>) -> Result<(), Error> {
        record_ctx_fields!(ctx);
        poise::builtins::help(ctx, command.as_deref(), HelpConfiguration::default()).await?;
        Ok(())
    }
}
