mod archive_handler;
mod conf;
mod helper;

use crate::conf::config::ARGS;
use anyhow::{Context, Result, anyhow, bail};
use conf::config;
use std::env;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use tracing::{Level, error, instrument, span};
use tracing::{debug, info, warn};
use tracing_subscriber::FmtSubscriber;
use walkdir::WalkDir;

fn main() -> Result<()> {
    let start = Instant::now();

    let log_level = ARGS
        .log_level()
        .parse::<Level>()
        .context("Unable to parse log level")?;

    let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
    tracing::subscriber::set_global_default(subscriber)
        .with_context(|| "setting default subscriber failed")?;
    debug!("Default subscriber is set");

    let (
        source_file,
        temp_dir,
        preserve_tmp_dir,
        result_dir,
        keep_non_ascii,
        jplag_jar,
        jplag_args,
        additional_submission_dirs,
    ) = config::parse_args().with_context(|| "Unable to parse args")?;
    debug!(
        "Full config: \
        source_file={source_file}, \
        temp_dir={temp_dir}, \
        preserve_tmp_dir={preserve_tmp_dir}, \
        results_dir={result_dir}, \
        keep_non_ascii={keep_non_ascii}, \
        jplag_jar={jplag_jar}, \
        jplag_args={jplag_args:?}, \
        additional_submission_dirs={additional_submission_dirs:?}"
    );

    info!("Checking if java is executable");

    helper::check_java_executable().with_context(|| "Check if java is executable failed")?;

    info!("Check successful");

    info!("Initializing project");
    init(
        &source_file,
        &result_dir,
        &temp_dir,
        &jplag_jar,
        &additional_submission_dirs,
    )
    .with_context(|| "Initialization failed")?;

    prepare(&temp_dir, keep_non_ascii).with_context(|| "Preparing submissions failed")?;

    let finished =
        run(&result_dir, &jplag_jar, jplag_args).with_context(|| "Running jplag failed")?;

    let runtime = finished - start;

    #[cfg(not(debug_assertions))]
    {
        if preserve_tmp_dir {
            info!("Not cleaning up, goodbye! ({} ms)", runtime.as_millis());
        } else {
            info!("Cleaning up");
            cleanup(&temp_dir).with_context(|| "Cleanup failed")?;
            info!("Finished cleanup, goodbye! ({} ms)", runtime.as_millis());
        }
    }

    #[cfg(debug_assertions)]
    info!("Finished program, goodbye! ({} ms)", runtime.as_millis());

    Ok(())
}

#[instrument]
fn init<P, Q, R>(
    source_file: P,
    result_dir: Q,
    tmp_dir: R,
    jplag_jar: &str,
    additional_submission_dirs: &Vec<String>,
) -> Result<()>
where
    P: AsRef<Path> + Debug + Into<String>,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("Checking if source zip file exist");
    if !fs::exists(&source_file)
        .with_context(|| format!("Unable to confirm if {source_file:?} exists"))?
    {
        bail!("Unable to find source zip file {source_file:?}");
    }

    debug!("Checking if jplag jar file exists");
    if !fs::exists(&jplag_jar)
        .with_context(|| format!("Unable to confirm if \"{jplag_jar}\" exists"))?
    {
        bail!("Unable to find jplag jar file \"{jplag_jar}\"");
    }

    debug!("Removing result dir");
    let _ = fs::remove_dir_all(&result_dir);
    fs::create_dir_all(&result_dir)?;

    debug!("Removing tmp dir");
    let _ = fs::remove_dir_all(&tmp_dir);

    helper::unzip_to(&source_file, &tmp_dir)
        .with_context(|| format!("Unable to extract {source_file:?} to {tmp_dir:?}"))?;

    helper::add_subs(&additional_submission_dirs, &tmp_dir).with_context(|| {
        format!(
            "Unable to copy additional submissions \
        {additional_submission_dirs:?} to {tmp_dir:?}"
        )
    })?;

    info!("Unzipped {source_file:?} to {tmp_dir:?}");

    Ok(())
}

#[instrument]
fn prepare<P>(tmp_dir: P, keep_non_ascii: bool) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    info!("Extracting individual submissions");
    let tmp_dir = tmp_dir.as_ref();

    let mut no_zip = vec![];

    let mut cnt = 0u8;
    for dir in fs::read_dir(tmp_dir).with_context(|| format!("Unable to read {tmp_dir:?}"))? {
        let dir = dir.with_context(|| format!("Unable to read a dir in {tmp_dir:?}"))?;
        let student_name_dir_path = dir.path();
        debug!("Processing path {student_name_dir_path:?}");

        if !student_name_dir_path.is_dir() {
            return Err(anyhow!(
                "Everything in {tmp_dir:?} should be a dir, found {student_name_dir_path:?}"
            ));
        }

        let mut archive_file = None;
        let mut fun: fn(_, _, _) -> Result<()> = archive_handler::dummy;
        for archive in WalkDir::new(&student_name_dir_path) {
            let archive = archive?;
            let archive_file_path = archive.path();

            let span = span!(
                Level::INFO,
                "student_archive",
                ?student_name_dir_path,
                ?archive,
                cnt
            );
            let _guard = span.enter();
            debug!("Processing student");
            cnt += 1;
            if cnt == 3 {
                panic!("Done")
            };

            let archive_extension = archive_file_path
                .extension()
                .and_then(|e| e.to_str())
                .and_then(|e| Some(e.to_ascii_lowercase()));

            fun = match archive_extension {
                Some(ref s) if s == "zip" => archive_handler::zip,
                Some(ref s) if s == "rar" => archive_handler::rar,
                Some(ref s) if s == "7z" => archive_handler::sz,
                Some(ref s) if s == "tar" => archive_handler::tar,
                Some(ref s) if s == "gz" => archive_handler::gz, // NOTE We assume, that all files ending in `.gz` are `.tar.gz` files
                _ => {
                    if archive.path().is_file() {
                        debug!("Found non archive file {archive:?}, removing");
                        fs::remove_file(&archive_file_path).with_context(|| {
                            format!(
                                "Unable to remove non archive file\
                            {archive:?}"
                            )
                        })?;
                    }
                    continue;
                }
            };
            if let Some(file) = archive_file {
                error!(first = ?file, second = ?archive_file_path, "Fuck that guy");
                archive_file = None;
                break;
                return Err(anyhow!(
                    "Found multiple archive files, expected one:\n\
                \tFirst: {file:?}\n\
                \tSecond: {archive_file_path:?}"
                ));
            }
            archive_file = Some(archive_file_path.to_owned());
        }

        let Some(archive_file) = archive_file else {
            no_zip.push(student_name_dir_path.to_owned());
            continue;
        };

        if let Err(e) = fun(&tmp_dir, &student_name_dir_path, &archive_file) {
            error!(?archive_file, error = ?e, "Unable to extract");
        }
    }

    for no_zip_student in no_zip {
        warn!("No zip file found in {no_zip_student:?}, removing path");
        fs::remove_dir_all(&no_zip_student).with_context(|| {
            format!(
                "Unable to remove path of student who didn't \
            hand in an assignment: {no_zip_student:?}"
            )
        })?;
    }

    info!("Unzipped all submissions, Sanitizing output files");
    helper::sanitize_submissions(&tmp_dir).with_context(|| "Unable to sanitize output files")?;

    info!("Sanitized output files, replacing diacritics");
    helper::clean_non_ascii(&tmp_dir, keep_non_ascii)
        .with_context(|| "Unable to replace diacritics")?;

    Ok(())
}

#[instrument]
fn run(result_dir: &str, jplag_jar: &str, jplag_args: Vec<String>) -> Result<Instant> {
    let mut jplag_cmd = format!("java -jar {jplag_jar}");

    for str in &jplag_args {
        jplag_cmd.push_str(&format!(" {str}"));
    }

    info!(cmd = jplag_cmd, "Starting jplag");

    let jplag_start_ts = Instant::now();

    let mut child = Command::new("java")
        .arg("-jar")
        .arg(&jplag_jar)
        .args(&jplag_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Unable to run jplag command {jplag_cmd}"))?;

    helper::listen_for_output(&mut child)
        .with_context(|| format!("Unable to listen to stdout of jplag, cmd: {jplag_cmd}"))?;

    info!("Finished running jplag");

    let status = child
        .wait()
        .with_context(|| format!("Unable to wait for child process {jplag_cmd}"))?;

    if !status.success() {
        warn!("Command failed, {status}");
        warn!("To debug manually, run \"{jplag_cmd}\" in the current directory");
        // Do not clean up on purpose, wwe want to see what caused the error
        bail!("Java jplag command failed, {status}");
    } else {
        debug!("{status}");
        let current_dir = env::current_dir().with_context(|| "Unable to get current dir")?;
        let result_dir = current_dir.join(result_dir);

        let mut result_file = PathBuf::from(format!(
            "Something went wrong, \
            there seems to be no result in {result_dir:?}"
        ));

        // This dir should only contain exactly one file
        for file in fs::read_dir(&result_dir)
            .with_context(|| format!("Unable to read result dir {result_dir:?}"))?
        {
            let file = file.with_context(|| format!("Invalid file in {result_dir:?}"))?;
            result_file = file.path();
        }

        info!("The results are also saved in {result_file:?}");
        Ok(jplag_start_ts)
    }
}

#[cfg(not(debug_assertions))]
#[instrument]
fn cleanup<P>(tmp_dir: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    fs::remove_dir_all(&tmp_dir)
        .with_context(|| format!("Could not cleanup tmp dir: {tmp_dir:?}"))?;

    Ok(())
}
