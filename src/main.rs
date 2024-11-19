mod config;

use anyhow::{Context, Result};
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");
    let (source_file, target_dir, temp_dir) = config::parse_args()
        .with_context(|| "Unable to parse args")?;

    Ok(())
}

fn init() -> Result<()> {
    todo!()
}

fn run() -> Result<()> {
    todo!()
}

fn cleanup() -> Result<()> {
    todo!()
}