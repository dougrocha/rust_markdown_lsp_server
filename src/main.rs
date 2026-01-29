use std::fs;

use etcetera::{BaseStrategy, choose_base_strategy};
use miette::{IntoDiagnostic, Result};
use simplelog::*;

use lsp::run_lsp;

fn main() -> Result<()> {
    create_logger()?;

    run_lsp()?;

    Ok(())
}

fn create_logger() -> Result<(), miette::Error> {
    let strategy = choose_base_strategy().unwrap();
    let log_path = strategy.cache_dir().join(env!("CARGO_PKG_NAME"));

    fs::create_dir_all(&log_path).into_diagnostic()?;

    let _ = WriteLogger::init(
        LevelFilter::max(),
        Config::default(),
        fs::File::create(log_path.join("log.txt")).expect("Failed to create log file: {log_path}"),
    );

    Ok(())
}
