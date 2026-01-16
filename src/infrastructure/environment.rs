use anyhow::Context as _;
use poise::serenity_prelude::GuildId;
use std::{
    env::var,
    path::{Path, PathBuf},
};

macro_rules! const_str {
    ($name:ident) => {
        pub const $name: &str = stringify!($name);
    };
}

const_str!(MEDIA_DIRECTORY);
const_str!(DATA_DIRECTORY);
const_str!(DISCORD_TOKEN);

const_str!(LOG_LEVEL);
const_str!(LOG_STYLE);
const_str!(LOG_PATH);

const_str!(OWNERS);

const_str!(DATABASE_URL);

pub fn env_var_with_context<K: AsRef<std::ffi::OsStr> + std::fmt::Display>(
    key: K,
) -> anyhow::Result<String> {
    var(&key).context(format!("Failed to load environment variable {}", key))
}

pub fn get_data_directory() -> PathBuf {
    let st: String = var(DATA_DIRECTORY).unwrap_or_else(|_| "./data".to_string());
    Path::new(st.as_str()).to_owned()
}

pub fn get_media_directory() -> PathBuf {
    let st: String = var(MEDIA_DIRECTORY).unwrap_or_else(|_| "./media".to_string());
    Path::new(st.as_str()).to_owned()
}

pub fn get_guild_user_content_directory(guild_id: GuildId) -> PathBuf {
    get_data_directory()
        .join("user_content")
        .join(crate::infrastructure::ids::id_to_string(guild_id))
}
