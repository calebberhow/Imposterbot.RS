use crate::{
    Error,
    infrastructure::{botdata::Data, ids, util::send_message_from_reply},
    lazy_regex,
};
use poise::{
    CreateReply,
    serenity_prelude::{Context, Emoji, GuildId, Http, Message, ReactionType},
};
use rand::seq::IndexedRandom;
use tracing::{info, warn};

lazy_regex! { BODY_REGEX, r"\bbody+\b"}
lazy_regex! { RED_SUS_REGEX, r"\bred sus\b"}
lazy_regex! { BLUE_SUS_REGEX, r"\bblue sus\b"}
lazy_regex! { NAV_REGEX, r"\bnav\b"}
lazy_regex! { BLITZCRANK_REGEX, r"\bblitzcrank\b"}
lazy_regex! { MEETING_REGEX, r"\bmeeting\b"}
lazy_regex! { IMPOSTERBOT_REGEX, r"\bimposterbot\b"}
lazy_regex! { SAD_REGEX, r"\bi(('*m)|( am)) sad\b"}
lazy_regex! { OWO_REGEX, r"\bowo\b"}
lazy_regex! { VENTED_REGEX, r"\bvented\b"}
lazy_regex! { SUSPICIOUS_REGEX, r"\bsuspicious\b"}
lazy_regex! { WHO_YOU_GONNA_CALL_REGEX, r"\bwho you gonna call\b"}
lazy_regex! { PAIN_REGEX, r"\bpain\b"}

async fn get_emote_by_name(
    ctx: impl AsRef<Http>,
    guild: Option<GuildId>,
    emote_name: &str,
) -> Option<Emoji> {
    if let Some(gid) = guild {
        return match gid.emojis(ctx).await {
            Ok(emojis) => match emojis.iter().find(|emoji| {
                emoji
                    .name
                    .to_lowercase()
                    .contains(emote_name.to_lowercase().as_str())
            }) {
                Some(emoji) => Some(emoji.clone()),
                _ => None,
            },
            _ => None,
        };
    }

    None
}

fn rand_message(messages: &[&str]) -> String {
    messages.choose(&mut rand::rng()).unwrap_or(&"").to_string()
}

fn matches_prefix(framework: poise::FrameworkContext<'_, Data, Error>, content: &String) -> bool {
    if let Some(p) = &framework.options.prefix_options.prefix
        && content.starts_with(p)
    {
        return true;
    }

    return false;
}

async fn send_reaction(
    message: &Message,
    ctx: &Context,
    emote_name: &str,
    guild_id: Option<GuildId>,
    on_guild_string: &String,
) -> Result<(), Error> {
    let emote_option = get_emote_by_name(ctx, guild_id, emote_name).await;
    if let Some(emote) = emote_option {
        let reaction = ReactionType::Custom {
            animated: emote.animated,
            id: emote.id,
            name: Some(emote.name),
        };
        message.react(ctx, reaction).await?;
    } else {
        warn!("Emoji 'pain' was not found {}", on_guild_string);
    }

    Ok(())
}

pub async fn on_message(
    ctx: &Context,
    framework: poise::FrameworkContext<'_, Data, Error>,
    _data: &Data,
    message: &Message,
) -> Result<(), Error> {
    if message.author.bot || matches_prefix(framework, &message.content) {
        return Ok(());
    }

    // Gathering metadata about message...
    let guild_id = message.guild_id;
    let guild_name = guild_id.and_then(|id| id.name(&ctx.cache));

    let username = &message.author.name;
    let display_name = if let Some(gid) = guild_id {
        message.author.nick_in(ctx, gid).await
    } else {
        None
    }
    .or(message.author.global_name.clone())
    .unwrap_or(username.clone());

    let on_guild_string = if let Some(x) = guild_name {
        format!("on guild '{}'", x)
    } else {
        "".into()
    };

    let content_lower = message.content.to_lowercase();
    if BODY_REGEX.is_match(&message.content) {
        info!("User '{}' said 'body' {}", display_name, on_guild_string);
        let reply = CreateReply::default().content("where");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if RED_SUS_REGEX.is_match(&message.content) {
        info!("User '{}' said 'red sus' {}", display_name, on_guild_string);
        let reply = CreateReply::default().content("I agree, vote red.");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if BLUE_SUS_REGEX.is_match(&message.content) {
        info!(
            "User '{}' said 'blue sus' {}",
            display_name, on_guild_string
        );
        let reply =
            CreateReply::default().content("I think blue is safe, I saw them do a med scan.");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if NAV_REGEX.is_match(&message.content) {
        info!("User '{}' said 'nav' {}", display_name, on_guild_string);
        let reply = CreateReply::default().content("I was just in nav, didn't see anyone.");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if BLITZCRANK_REGEX.is_match(&message.content) {
        info!(
            "User '{}' said 'blitzcrank' {}",
            display_name, on_guild_string
        );
        message
            .react(ctx, ReactionType::Unicode("👍".to_string()))
            .await?;
    } else if MEETING_REGEX.is_match(&message.content) {
        info!("User '{}' said 'meeting' {}", display_name, on_guild_string);
        send_reaction(message, ctx, "deny", guild_id, &on_guild_string).await?;
        let reply = CreateReply::default().content("**Loud meeting button noise**");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if IMPOSTERBOT_REGEX.is_match(&message.content) {
        info!(
            "User '{}' said 'imposterbot' {}",
            display_name, on_guild_string
        );
        let responses = [
            "Not me, vote cyan.",
            "I was in admin.",
            "Didn't see orange at O2..",
            "It wasn't me, vote lime.",
        ];
        let reply = CreateReply::default().content(rand_message(&responses));
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if SAD_REGEX.is_match(&message.content) {
        info!(
            "User '{}' said they are sad {}",
            display_name, on_guild_string
        );
        let responses = ["Don't be sad 😢", "Cheer up!"]; // Simplified emoji
        let reply = CreateReply::default().content(rand_message(&responses));
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if OWO_REGEX.is_match(&content_lower) {
        info!("User '{}' said 'owo' {}", display_name, on_guild_string);
        let reply = CreateReply::default().content("OwO?");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if VENTED_REGEX.is_match(&message.content) {
        info!("User '{}' said 'vented' {}", display_name, on_guild_string);
        let responses = [
            "Was it green? I thought I saw them vent.",
            "I was in storage.. no where near any vents.",
        ];
        let reply = CreateReply::default().content(rand_message(&responses));
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
        let emote_option = get_emote_by_name(ctx, guild_id, "deny").await;
        if let Some(emote) = emote_option {
            let reaction = ReactionType::Custom {
                animated: emote.animated,
                id: emote.id,
                name: Some(emote.name),
            };
            message.react(ctx, reaction).await?;
        }
    } else if SUSPICIOUS_REGEX.is_match(&message.content) {
        info!(
            "User '{}' said 'suspicious' {}",
            display_name, on_guild_string
        );
        let reply = CreateReply::default().content("Very sus.");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
        let reply = CreateReply::default().content("👀");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if WHO_YOU_GONNA_CALL_REGEX.is_match(&message.content) {
        info!("User '{}' said 'pain' {}", display_name, on_guild_string);
        let reply = CreateReply::default().content("ghost busters!");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    } else if PAIN_REGEX.is_match(&message.content) {
        info!("User '{}' said 'pain' {}", display_name, on_guild_string);
        let emote_option = get_emote_by_name(ctx, guild_id, "pain").await;
        if let Some(emote) = emote_option {
            let reaction = ReactionType::Custom {
                animated: emote.animated,
                id: emote.id,
                name: Some(emote.name),
            };
            message.react(ctx, reaction).await?;
        } else {
            warn!("Emoji 'pain' was not found {}", on_guild_string);
        }
    } else if message.content == "<:doggoban:802308677737381948>"
        && [ids::KHAZAARI_ID, ids::CRESSY_ID].contains(&message.author.id)
    {
        info!(
            "User '{}' sent doggoban emoji {}",
            display_name, on_guild_string
        );
        let reply = CreateReply::default().content("Banning **MoustachioMario#2067**");
        send_message_from_reply(&message.channel_id, ctx, reply).await?;
    }

    Ok(())
}
