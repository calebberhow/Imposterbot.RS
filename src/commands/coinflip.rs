use log::trace;
use poise::{CreateReply, serenity_prelude::CreateEmbed};
use rand::Rng;

use crate::{
    Context, Error,
    infrastructure::{colors, util::DebuggableReply},
};

fn do_flip(probability: Option<f64>) -> bool {
    let mut rand = rand::rng();
    let p = probability.unwrap_or(0.5);
    trace!(probability=p; "Generating bool with probability");
    let value = rand.random_bool(p);
    trace!(value=value; "Generated");
    value
}

#[poise::command(slash_command, prefix_command, track_edits, track_deletion)]
pub async fn coinflip(
    ctx: Context<'_>,
    #[description = "Visible to you only? (default: false)"] ephemeral: Option<bool>,
    #[description = "Probability of heads (default: 0.5)"] probability: Option<f64>,
) -> Result<(), Error> {
    trace!(ephemeral=ephemeral, probability=probability; "Coinflip executed with args");

    if probability.is_some() && !matches!(probability.unwrap(), 0.0..=1.0) {
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

    trace!("Sending reply: {:?}", DebuggableReply::new(&reply));
    ctx.send(reply).await?;
    Ok(())
}
