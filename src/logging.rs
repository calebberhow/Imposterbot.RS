use std::path::PathBuf;

use env_logger::{Builder, Env};
use imposterbot::infrastructure::environment;
use tracing::{error, info};

pub fn init_logger() {
    let env_file = load_env_file();
    init_env_logger();
    info!("Starting Imposterbot...");
    log_env_file_result(env_file);
}

fn get_log_path_var() -> Option<bool> {
    match std::env::var(environment::LOG_PATH) {
        Ok(path) => match path.parse::<bool>() {
            Ok(value) => Some(value),
            Err(e) => {
                error!("Failed to parse the value: {:?}", e);
                None
            }
        },
        Err(_) => None,
    }
}

fn init_env_logger() {
    let env = Env::default()
        .filter_or(environment::LOG_LEVEL, "warn,imposterbot=info")
        .write_style_or(environment::LOG_STYLE, "always");
    Builder::from_env(env)
        .default_format()
        .format_source_path(get_log_path_var().unwrap_or(false))
        .format_timestamp_millis()
        .init();
}

fn load_env_file() -> Option<PathBuf> {
    dotenvy::dotenv().ok()
}

fn log_env_file_result(env_file: Option<PathBuf>) {
    if let Some(path) = env_file {
        info!("Loaded environment variables from {}", path.display());
    } else {
        info!("No .env file found, proceeding with system environment variables.");
    }
}
