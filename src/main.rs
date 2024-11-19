mod config;
mod helper;

use std::fmt::Debug;
use std::fs;
use std::path::Path;
use anyhow::{Context, Result};
use tracing::{debug, info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    #[cfg(not(debug_assertions))]
    let log_level = config::get_log_level();
    #[cfg(debug_assertions)]
    let log_level = Level::DEBUG;
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "setting default subscriber failed")?;
    debug!("Default subscriber is set");

    let (
        source_file,
        temp_dir,
        result_dir,
        jplag_jar,
        jplag_args,
    ) = config::parse_args()
        .with_context(|| "Unable to parse args")?;
    debug!("Full config: source_file={source_file}, temp_dir={temp_dir}, results_dir={result_dir}");

    info!("Initializing project");
    init(&source_file, &result_dir, &temp_dir)
        .with_context(|| "Initialization failed")?;

    info!("Running jplag");
    run(&result_dir, &temp_dir)
        .with_context(|| "Running jplag failed")?;

    info!("Cleaning up");
    #[cfg(not(debug_assertions))]
    cleanup(&temp_dir)
        .with_context(|| "Cleanup failed")?;

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

#[cfg(not(debug_assertions))]
fn cleanup<P>(tmp_dir: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    fs::remove_dir_all(&tmp_dir)
        .with_context(|| format!("Could not cleanup tmp dir: {tmp_dir:?}"))?;

    Ok(())
}
