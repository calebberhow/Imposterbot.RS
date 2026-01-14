use crate::infrastructure::botdata;

pub mod entities;

pub mod commands {
    pub mod builtins;
    pub mod coinflip;
    pub mod member_management;
    pub mod minecraft;
    pub mod roll;
    #[cfg(feature = "voice")]
    pub mod voice;
}

pub mod infrastructure {
    pub mod botdata;
    pub mod colors;
    pub mod environment;
    pub mod event_handler;
    pub mod ids;
    pub mod util;
}

pub mod events {
    pub mod guild_member;
    pub mod message;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, botdata::Data, Error>;
