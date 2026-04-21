use std::fs;

use etcetera::{BaseStrategy, choose_base_strategy};
use miette::{IntoDiagnostic, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use lib_lsp::run_lsp;

fn main() -> Result<()> {
    let _guard = init_tracing()?;

    run_lsp()?;

    Ok(())
}

fn init_tracing() -> Result<WorkerGuard> {
    let strategy = choose_base_strategy().unwrap();
    let log_path = strategy.cache_dir().join(env!("CARGO_PKG_NAME"));
    fs::create_dir_all(&log_path).into_diagnostic()?;

    let file_appender = tracing_appender::rolling::never(&log_path, "log.txt");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,rust_markdown_lsp=debug,lib_lsp=debug,lib_core=debug,lib_parser=debug")
    });

    tracing_subscriber::registry()
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false),
        )
        .init();

    Ok(guard)
}
