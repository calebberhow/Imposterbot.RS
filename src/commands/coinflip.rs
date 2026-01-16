use poise::{CreateReply, serenity_prelude::CreateEmbed};
use rand::Rng;

use crate::{
    Context, Error,
    infrastructure::{
        colors,
        util::{DebuggableReply, defer_or_broadcast},
    },
    poise_instrument, record_ctx_fields,
};

fn do_flip(probability: Option<f64>) -> bool {
    let mut rand = rand::rng();
    let p = probability.unwrap_or(0.5);
    let value = rand.random_bool(p);
    value
}

poise_instrument! {
    /// Flips a coin
    #[poise::command(
        slash_command,
        prefix_command,
        category = "Fun",
        track_edits,
        track_deletion
    )]
    pub async fn coinflip(
        ctx: Context<'_>,
        #[description = "Visible to you only? (default: false)"] ephemeral: Option<bool>,
        #[description = "Probability of heads (default: 0.5)"] probability: Option<f64>,
    ) -> Result<(), Error> {
        record_ctx_fields!(ctx);
        let _typing = defer_or_broadcast(ctx, ephemeral.unwrap_or_default()).await?;

        if let Some(p) = probability
            && !matches!(p, 0.0..=1.0)
        {
            return Err("Probability out of range".into());
        }

        let result = do_flip(probability);
        let reply = CreateReply::default()
            .embed(
                CreateEmbed::new()
                    .title("Coin Flip")
                    .description(format!(
                        "It's {} {}",
                        if result { "heads" } else { "tails" },
                        if let Some(p) = probability {
                            format!("(p={})", if result { p } else { 1.0 - p })
                        } else {
                            "".into()
                        }
                    ))
                    .color(colors::slate()),
            )
            .ephemeral(ephemeral.unwrap_or(false));

        tracing::trace!("Sending reply: {:?}", DebuggableReply::new(&reply));
        ctx.send(reply).await?;
        Ok(())
    }

}
