use crate::infrastructure::botdata;

pub mod commands {
    pub mod builtins;
    pub mod coinflip;
    pub mod minecraft;
    pub mod roll;
}

pub mod infrastructure {
    pub mod botdata;
    pub mod colors;
    pub mod environment;
    pub mod events;
    pub mod util;
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, botdata::Data, Error>;
