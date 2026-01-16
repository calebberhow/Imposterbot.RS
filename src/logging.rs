use std::path::PathBuf;

use imposterbot::infrastructure::environment::{self, get_data_directory};
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initializes the logger and returns a boxed reference to resources that if dropped will stop the logger.
pub fn init_logger() -> Box<dyn std::any::Any> {
    let env_file = load_env_file();

    let guard = init_tracing();

    info!("Starting Imposterbot...");
    log_env_file_result(env_file);

    guard
}

fn get_log_path_var() -> bool {
    match std::env::var(environment::LOG_PATH) {
        Ok(path) => match path.parse::<bool>() {
            Ok(value) => value,
            Err(e) => {
                error!("Failed to parse {}: {:?}", environment::LOG_PATH, e);
                false
            }
        },
        Err(_) => false,
    }
}

fn init_tracing() -> Box<dyn std::any::Any> {
    // Rotate daily; options: Rotation::NEVER, Rotation::HOURLY, Rotation::DAILY
    let log_dir = get_data_directory().join("logs");
    std::fs::create_dir_all(&log_dir).expect("Log directory should be createable.");
    let file_appender = tracing_appender::rolling::daily(log_dir, "imposterbot.log");

    // Optional: keep last N files (needs extra code, not built-in)
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

    let env_filter = EnvFilter::try_from_env(environment::LOG_LEVEL)
        .unwrap_or_else(|_| EnvFilter::new("warn,imposterbot=info"));

    let do_log_path = get_log_path_var();
    tracing_subscriber::registry()
        .with(env_filter)
        // file layer
        .with(
            fmt::layer()
                .with_writer(non_blocking_writer)
                .with_ansi(false)
                .with_file(do_log_path)
                .with_line_number(do_log_path)
                .with_target(!do_log_path)
                .with_span_events(fmt::format::FmtSpan::CLOSE),
        )
        // stdout layer
        .with(
            fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true)
                .with_file(do_log_path)
                .with_line_number(do_log_path)
                .with_target(!do_log_path)
                .with_span_events(fmt::format::FmtSpan::CLOSE),
        )
        .init();
    Box::new(guard)
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
