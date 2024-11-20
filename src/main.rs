mod config;
mod helper;
mod custom_error;

use std::fmt::Debug;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Context, Result};
use tracing::{debug, info, warn};
#[cfg(debug_assertions)]
use tracing::Level;
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
    debug!("Full config: \
    source_file={source_file}, \
    temp_dir={temp_dir}, \
    results_dir={result_dir}, \
    jplag_jar={jplag_jar}, \
    jplag_args={jplag_args:?}");

    info!("Initializing project");
    init(&source_file, &result_dir, &temp_dir, &jplag_jar)
        .with_context(|| "Initialization failed")?;

    run(&result_dir, &temp_dir, &jplag_jar, jplag_args)
        .with_context(|| "Running jplag failed")?;

    #[cfg(not(debug_assertions))]
    {
        info!("Cleaning up");
        cleanup(&temp_dir)
            .with_context(|| "Cleanup failed")?;
    }

    Ok(())
}

fn init<P, Q, R>(source_file: P, result_dir: Q, tmp_dir: R, jplag_jar: &str) -> Result<()>
where
    P: AsRef<Path> + Debug + Into<String>,
    Q: AsRef<Path>,
    R: AsRef<Path> + Debug,
{
    debug!("Checking if source zip file exist");
    if !fs::exists(&source_file)
        .with_context(|| format!("Unable to confirm if {source_file:?} exists"))? {
        return Err(custom_error::FileNotFoundError::ZipFileNotFound(source_file.into()).into());
    }

    debug!("Checking if jplag jar file exists");
    if !fs::exists(&jplag_jar)
        .with_context(|| format!("Unable to confirm if \"{jplag_jar}\" exists"))? {
        return Err(custom_error::FileNotFoundError::JarFileNotFound(jplag_jar.into()).into());
    }

    debug!("Removing result dir");
    let _ = fs::remove_dir_all(&result_dir);

    debug!("Removing tmp dir");
    let _ = fs::remove_dir_all(&tmp_dir);

    helper::unzip_to(&source_file, &tmp_dir)
        .with_context(|| format!("Unable to extract {source_file:?} to {tmp_dir:?}"))?;

    info!("Unzipped {source_file:?} to {tmp_dir:?}");

    Ok(())
}

fn run<P>(result_dir: &str, tmp_dir: P, jplag_jar: &str, jplag_args: Vec<String>) -> Result<()>
where
    P: AsRef<Path>,
{
    info!("Extracting individual submissions");
    let tmp_dir = tmp_dir.as_ref();

    for dir in fs::read_dir(tmp_dir)
        .with_context(|| format!("Unable to read {tmp_dir:?}"))? {
        let dir = dir.with_context(|| format!("Unable to read a dir in {tmp_dir:?}"))?;
        let student_name_dir_path = dir.path();
        debug!("Processing path {student_name_dir_path:?}");

        assert!(student_name_dir_path.is_dir(), "Everything in {tmp_dir:?} should be a dir, found {student_name_dir_path:?}");

        let mut count = 0u8;
        for zip_entry in fs::read_dir(&student_name_dir_path)? {
            let zip_entry = zip_entry?;
            let zip_file_path = zip_entry.path();

            assert_eq!(count, 0, "Expected to find exactly one file, found more: {:?}", zip_file_path);
            count += 1;

            assert_eq!(
                Some(String::from("zip")),
                zip_file_path.extension()
                             .and_then(|e| e.to_str())
                             .and_then(|e| Some(e.to_ascii_lowercase())),
                "Expected to find a zip file, found {:?}", zip_file_path,
            );

            let zip_dir_name = student_name_dir_path.file_name()
                                                    .and_then(|f| f.to_str())
                                                    .with_context(|| format!("Unable to get file name of {:?}", student_name_dir_path))?;

            // let zip_target_dir = format!("{zip_dir_name}/out");
            let zip_target_dir = zip_dir_name;
            let macos_dir = tmp_dir.join(format!("{zip_target_dir}/__MACOSX/"));
            let dest = tmp_dir.join(&zip_target_dir);

            debug!("Set destination of unzipped file to {dest:?}");

            fs::create_dir_all(&dest)
                .with_context(|| format!("Unable to create {tmp_dir:?}"))?;

            debug!("Created {dest:?}");

            helper::unzip_to(&zip_file_path, &dest)
                .with_context(|| format!("Unable to unzip {zip_file_path:?} to {dest:?}"))?;

            debug!("Unzipped {zip_file_path:?} to {dest:?}");

            debug!("Removing macosx dir: {macos_dir:?}");
            let _ = fs::remove_dir_all(macos_dir); // I hate apple so much
            debug!("Successfully removed macosx dir");

            fs::remove_file(&zip_file_path)
                .with_context(|| format!("Unable to remove {zip_file_path:?}"))?;

            debug!("Removed {zip_file_path:?}");
        }
    }

    info!("Unzipped all submissions, running jplag (I hope you have java installed)");

    let mut dbg_cmd = format!("java -jar {jplag_jar}");

    for str in &jplag_args {
        dbg_cmd.push_str(&format!(" {str}"));
    }

    info!("Starting jplag");
    debug!("Raw command: {dbg_cmd}");

    let mut child = Command::new("java")
        .arg("-jar")
        .arg(&jplag_jar)
        .args(&jplag_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Unable to run jplag command {dbg_cmd}"))?;

    helper::listen_for_output(&mut child)
        .with_context(|| format!("Unable to listen to stdout of jplag, cmd: {dbg_cmd}"))?;

    info!("Finished running jplag");

    let status = child.wait()
                      .with_context(|| format!("Unable to wait for child process {dbg_cmd}"))?;

    if !status.success() {
        warn!("Command failed, {status}");
    } else {
        info!("Finished successfully, {status}");
        info!("Look at the results by opening {result_dir}index.html in your browser")
    }

    Ok(())
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
