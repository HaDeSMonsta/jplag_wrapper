mod config;
mod helper;

use std::fmt::Debug;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use tracing::{debug, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "setting default subscriber failed")?;
    debug!("Default subscriber is set");

    let (source_file, temp_dir, result_dir) = config::parse_args()
        .with_context(|| "Unable to parse args")?;

    init(&source_file, &temp_dir, result_dir)
        .with_context(|| "Initialization failed")?;

    Ok(())
}

fn init<P, Q, R>(source_file: P, result_dir: Q, tmp_dir: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path>,
    R: AsRef<Path> + Debug,
{
    debug!("Removing result dir");
    let _ = fs::remove_dir_all(&result_dir);

    debug!("Removing tmp dir");
    let _ = fs::remove_dir_all(&tmp_dir);

    helper::unzip_to(&source_file, &tmp_dir)
        .with_context(|| format!("Unable to extract {source_file:?} to {tmp_dir:?}"))?;

    Ok(())
}

fn run<P, Q>(result_dir: P, tmp_dir: Q) -> Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    todo!()
}

fn cleanup<P>(tmp_dir: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    #[cfg(not(debug_assertions))]
    fs::remove_dir_all(&tmp_dir)
        .with_context(|| format!("Could not cleanup tmp dir: {tmp_dir:?}"))?;

    Ok(())
}
