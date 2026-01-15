use poise::{
    CreateReply,
    serenity_prelude::{Colour, CreateAttachment, CreateEmbed, CreateEmbedAuthor},
};
use rand::Rng;
use tracing::trace;

use crate::{
    Context, Error,
    infrastructure::{
        environment::get_media_directory,
        util::{DebuggableReply, defer_or_broadcast},
    },
};

#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
enum Dice {
    D4 = 4,
    D6 = 6,
    D8 = 8,
    D10 = 10,
    D12 = 12,
    D20 = 20,
}

impl Dice {
    fn as_str(&self) -> &'static str {
        match self {
            Dice::D4 => "d4",
            Dice::D6 => "d6",
            Dice::D8 => "d8",
            Dice::D10 => "d10",
            Dice::D12 => "d12",
            Dice::D20 => "d20",
        }
    }
}

fn dice_number(dice: &Dice) -> u8 {
    *dice as u8
}

fn roll_dice(dice: &Dice) -> u8 {
    let mut rng = rand::rng();
    let value = rng.random_range(1..=dice_number(dice)) as u8;
    trace!(value = value, "Generated");
    value
}

async fn get_dice_attachment(
    dice: &Dice,
    side: u8,
) -> Result<CreateAttachment, poise::serenity_prelude::Error> {
    let path = get_media_directory().join(dice.as_str()).join(format!(
        "{}-{}.png",
        dice.as_str(),
        side.to_string(),
    ));
    CreateAttachment::path(path).await
}

fn make_color(dice: &Dice, side: u8) -> Colour {
    let max = dice_number(dice) as u32;
    let u32_side = side as u32;
    let green = std::cmp::min(255 * (u32_side - 1) / (max / 2), 255);
    let red = std::cmp::min(255 * (max - u32_side) / (max / 2), 255);
    Colour::from_rgb(red as u8, green as u8, 0)
}

fn make_description(side: u8) -> String {
    if side == 1 {
        return "Critical **FAIL**".into();
    }
    format!("It rolled {}", side)
}

// TODO: add modifier and quantity optional parameters
/// Rolls a dice
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    track_deletion,
    category = "Fun",
    aliases("dice")
)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "The type of die to roll"] dice: Dice,
    #[description = "Visible to you only? (default: false)"] ephemeral: Option<bool>,
) -> Result<(), Error> {
    trace!(
        dice = dice.as_str(),
        ephemeral = ephemeral,
        "Coinflip executed with args"
    );
    let _typing = defer_or_broadcast(ctx, ephemeral.unwrap_or_default()).await?;

    let side = roll_dice(&dice);
    let attachment = get_dice_attachment(&dice, side).await?;

    let mut author = CreateEmbedAuthor::new(format!(
        "{} rolls 1{:?}",
        ctx.author()
            .member
            .as_ref()
            .and_then(|m| m.nick.clone())
            .unwrap_or(ctx.author().display_name().to_string()),
        dice
    ));
    let avatar_url = ctx.author().avatar_url();
    if let Some(s) = avatar_url {
        author = author.icon_url(s);
    }

    let embed = CreateEmbed::new()
        .thumbnail(format!("attachment://{}", attachment.filename))
        .author(author)
        .color(make_color(&dice, side))
        .description(make_description(side));

    let reply = CreateReply::default()
        .embed(embed)
        .attachment(attachment)
        .ephemeral(ephemeral.unwrap_or_default());
    trace!("Sending reply: {:?}", DebuggableReply::new(&reply));
    ctx.send(reply).await?;
    Ok(())
}
