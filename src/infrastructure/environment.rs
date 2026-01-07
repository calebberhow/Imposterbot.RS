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
