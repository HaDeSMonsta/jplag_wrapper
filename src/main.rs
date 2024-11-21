mod config;
mod helper;
mod custom_errors;
mod archive_handler;

use std::fmt::Debug;
use std::fs;
#[cfg(feature = "legacy")]
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;
use anyhow::{Context, Result};
use tracing::{debug, info, warn};
#[cfg(debug_assertions)]
use tracing::Level;
use tracing_subscriber::FmtSubscriber;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let start = Instant::now();
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

    info!("Checking if java is executable");

    helper::check_java_executable()
        .with_context(|| "Check if java is executable failed")?;

    info!("Check successful");

    let (
        source_file,
        temp_dir,
        result_dir,
        jplag_jar,
        jplag_args,
        ignore_jplag_output,
    ) = config::parse_args()
        .with_context(|| "Unable to parse args")?;
    debug!("Full config: \
    source_file={source_file}, \
    temp_dir={temp_dir}, \
    results_dir={result_dir}, \
    jplag_jar={jplag_jar}, \
    jplag_args={jplag_args:?}, \
    ignore_jplag_output={ignore_jplag_output}");

    info!("Initializing project");
    init(&source_file, &result_dir, &temp_dir, &jplag_jar)
        .with_context(|| "Initialization failed")?;

    run(
        &result_dir,
        &temp_dir,
        &jplag_jar,
        jplag_args,
        ignore_jplag_output,
    )
        .with_context(|| "Running jplag failed")?;

    #[cfg(not(debug_assertions))]
    {
        info!("Cleaning up");
        cleanup(&temp_dir)
            .with_context(|| "Cleanup failed")?;
        info!("Finished cleanup, goodbye! ({} ms)", start.elapsed().as_millis());
    }

    #[cfg(debug_assertions)]
    info!("Finished program, goodbye! ({} ms)", start.elapsed().as_millis());

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
        return Err(custom_errors::FileNotFoundError::ZipFileNotFound(source_file.into()).into());
    }

    debug!("Checking if jplag jar file exists");
    if !fs::exists(&jplag_jar)
        .with_context(|| format!("Unable to confirm if \"{jplag_jar}\" exists"))? {
        return Err(custom_errors::FileNotFoundError::JarFileNotFound(jplag_jar.into()).into());
    }

    debug!("Removing result dir");
    let _ = fs::remove_dir_all(&result_dir);
    #[cfg(not(feature = "legacy"))]
    fs::create_dir_all(&result_dir)?;

    debug!("Removing tmp dir");
    let _ = fs::remove_dir_all(&tmp_dir);

    helper::unzip_to(&source_file, &tmp_dir)
        .with_context(|| format!("Unable to extract {source_file:?} to {tmp_dir:?}"))?;

    info!("Unzipped {source_file:?} to {tmp_dir:?}");

    Ok(())
}

fn run<P>(
    result_dir: &str,
    tmp_dir: P,
    jplag_jar: &str,
    jplag_args: Vec<String>,
    ignore_jplag_output: bool,
) -> Result<()>
where
    P: AsRef<Path>,
{
    info!("Extracting individual submissions");
    let tmp_dir = tmp_dir.as_ref();

    let mut no_zip = vec![];

    for dir in fs::read_dir(tmp_dir)
        .with_context(|| format!("Unable to read {tmp_dir:?}"))? {
        let dir = dir.with_context(|| format!("Unable to read a dir in {tmp_dir:?}"))?;
        let student_name_dir_path = dir.path();
        debug!("Processing path {student_name_dir_path:?}");

        assert!(student_name_dir_path.is_dir(), "Everything in {tmp_dir:?} should be a dir, found {student_name_dir_path:?}");

        let mut archive_count = 0u8;
        for archive in WalkDir::new(&student_name_dir_path) {
            let archive = archive?;
            let archive_file_path = archive.path();

            let archive_extension = archive_file_path.extension()
                                                     .and_then(|e| e.to_str())
                                                     .and_then(|e| Some(e.to_ascii_lowercase()));

            let fun = match archive_extension {
                Some(ref s) if s == "zip" => archive_handler::zip,
                Some(ref s) if s == "rar" => archive_handler::rar,
                Some(ref s) if s == "7z" => archive_handler::sz,
                Some(ref s) if s == "tar" => archive_handler::tar,
                Some(ref s) if s == "gz" => archive_handler::gz, // NOTE We assume, that all files ending in `.gz` are `.tar.gz` files
                _ => {
                    if archive.path().is_file() {
                        debug!("Found non archive file {archive:?}, removing");
                        fs::remove_file(&archive_file_path)
                            .with_context(|| format!("Unable to remove non archive file\
                            {archive:?}"))?;
                    }
                    continue;
                }
            };
            fun(&tmp_dir, &student_name_dir_path, &archive_file_path)
                .with_context(|| format!("Unable to extract {archive_file_path:?}"))?;

            assert_eq!(
                archive_count,
                0,
                "Expected to find exactly one archive file, found more: {:?}",
                archive_file_path
            );

            archive_count += 1;
        }
        if archive_count != 1 {
            no_zip.push(student_name_dir_path.to_owned());
        }
    }

    for no_zip_student in no_zip {
        warn!("No zip file found in {no_zip_student:?}, removing path");
        fs::remove_dir_all(&no_zip_student)
            .with_context(|| format!("Unable to remove path o student who didn't \
            hand in an assignment: {no_zip_student:?}"))?;
    }

    info!("Unzipped all submissions, Sanitizing output");
    helper::sanitize_submissions(&tmp_dir)
        .with_context(|| "Unable to sanitize output")?;

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

    helper::listen_for_output(&mut child, ignore_jplag_output)
        .with_context(|| format!("Unable to listen to stdout of jplag, cmd: {dbg_cmd}"))?;

    info!("Finished running jplag");

    let status = child.wait()
                      .with_context(|| format!("Unable to wait for child process {dbg_cmd}"))?;

    if !status.success() {
        warn!("Command failed, {status}");
        // Do not clean up on purpose, wwe want to see what caused the error
        Err(custom_errors::SubCmdError::JplagExecFailure(status.code().unwrap()).into())
    } else {
        info!("{status}");
        #[cfg(not(feature = "legacy"))]
        info!("Look at the results by uploading the file in {result_dir} to \
        https://jplag.github.io/JPlag/");
        #[cfg(feature = "legacy")]
        {
            let current_dir = env::current_dir()
                .with_context(|| "Unable to get current dir")?;
            let result_file = current_dir.join(format!("{result_dir}/index.html"));
            info!("Look at the results by opening file://{} in your browser", result_file.display());
        }
        Ok(())
    }
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
