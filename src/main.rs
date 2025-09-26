mod archive_handler;
mod conf;
mod helper;
#[macro_use]
mod macros;

use crate::conf::config::ARGS;
use color_eyre::Result;
use color_eyre::eyre::{Context, anyhow, bail};
use conf::config;
use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use std::{env, thread};
use tracing::{Level, debug_span, instrument, span, trace};
use tracing::{debug, info, warn};
use tracing_subscriber::FmtSubscriber;
use walkdir::WalkDir;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    color_eyre::install().context("Failed to install :(")?;
    let start = Instant::now();

    {
        let log_level = ARGS
            .log_level()
            .parse::<Level>()
            .context("Unable to parse log level")?;

        let subscriber = FmtSubscriber::builder().with_max_level(log_level).finish();
        tracing::subscriber::set_global_default(subscriber)
            .context("setting default subscriber failed")?;
    }
    debug!("Default subscriber is set");

    info!("JPlag-rs v{VERSION}");

    let parsed_args = config::parse_args().context("Unable to parse args")?;
    debug!(?parsed_args);

    info!("Checking if java is executable");

    helper::check_java_executable().context("Check if java is executable failed")?;

    info!("Check successful");

    info!("Initializing project");
    init(
        &parsed_args.source_file,
        &parsed_args.target_dir,
        &parsed_args.tmp_dir,
        &parsed_args.jplag_jar,
        &parsed_args.additional_submission_dirs,
    )
    .context("Initialization failed")?;

    let errs = prepare(
        &parsed_args.tmp_dir,
        parsed_args.keep_non_ascii,
        parsed_args.abort_on_error,
    )
    .context("Preparing submissions failed")?;

    let runtime = start.elapsed();

    run(
        &parsed_args.target_dir,
        &parsed_args.jplag_jar,
        &parsed_args.jplag_args,
    )
    .context("Running jplag failed")?;

    for err in errs {
        warn!(%err);
    }

    #[cfg(not(debug_assertions))]
    {
        if parsed_args.preserve_tmp_dir {
            info!("Not cleaning up, goodbye! ({} ms)", runtime.as_millis());
        } else {
            info!("Cleaning up");
            let tmp_dir = &parsed_args.tmp_dir;
            fs::remove_dir_all(&tmp_dir)
                .with_context(|| format!("Removing tmp dir {tmp_dir:?} failed"))?;
            info!("Finished cleanup, goodbye! ({} ms)", runtime.as_millis());
        }
    }

    #[cfg(debug_assertions)]
    info!("Finished program, goodbye! ({} ms)", runtime.as_millis());

    Ok(())
}

/// Initializes the file structure and prerequisite setup for the program to execute.
///
/// This function performs the following steps:
/// 1. Verifies the existence of the source zip file and the JPlag JAR file.
/// 2. Removes and recreates the result directory.
/// 3. Removes the temporary directory if it exists.
/// 4. Unzips the source file into the temporary directory.
/// 5. Adds additional submissions from specified directories to the temporary directory.
///
/// # Parameters
/// - `source_file`: The path to the zip file containing the source submissions.
/// - `result_dir`: The directory path where the results will be stored.
/// - `tmp_dir`: The temporary directory path where the contents of the source file
///              will be unzipped and processed.
/// - `jplag_jar`: The path to the JPlag JAR file
/// - `additional_submission_dirs`: A vector of directory paths containing additional
///                                 submission files to be incorporated.
///
/// # Errors
/// - Returns an error if:
///   - The `source_file` does not exist or cannot be verified to exist.
///   - The `jplag_jar` file does not exist or cannot be verified to exist.
///   - The `result_dir` cannot be created.
///   - The `tmp_dir` cannot be removed or unzipped to.
///   - Adding additional submissions to the temporary directory fails.
#[instrument(skip_all)]
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
    debug!(?source_file, "Checking if source zip file exist");
    if !fs::exists(&source_file)
        .with_context(|| format!("Unable to confirm if {source_file:?} exists"))?
    {
        bail!("Unable to find source zip file {source_file:?}");
    }

    debug!(?jplag_jar, "Checking if jplag jar file exists");
    if !fs::exists(&jplag_jar)
        .with_context(|| format!("Unable to confirm if {jplag_jar:?} exists"))?
    {
        bail!("Unable to find jplag jar file {jplag_jar:?}");
    }

    debug!(?result_dir, "Recreating result dir");
    let _ = fs::remove_dir_all(&result_dir);
    fs::create_dir_all(&result_dir)?;

    debug!(?tmp_dir, "Removing tmp dir");
    let _ = fs::remove_dir_all(&tmp_dir);

    debug!("Unzipping {source_file:?} to {tmp_dir:?}");
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

/// Prepares a given temporary directory by processing and extracting student submissions.
///
/// This function iterates through the provided directory, expecting each entry to represent a student's submission.
/// It identifies, validates, and extracts archives (e.g., `.zip`, `.rar`, `.7z`, etc.) inside each student directory,
/// applies sanitization, and optionally replaces or retains non-ASCII characters in filenames.
/// Any errors encountered during this process are collected and returned.
///
/// # Returns
/// - `Ok(Vec<color_eyre::eyre::Error>)`: A vector of errors encountered during the processing, if no critical errors occurred.
/// - `Err(color_eyre::eyre::Error)`: A critical error that stops the process entirely, such as being unable to read the provided directory.
///
/// # Workflow
/// The function undertakes the following operations:
/// 1. Reads and iterates over the directories representing individual student submissions.
/// 2. For each student directory:
///     - Validates that the entry is a directory.
///     - Identifies the archive file within the directory.
///         - Archives are recognized by their file extensions (e.g., `.zip`, `.rar`, `.7z`, `.tar`, `.gz`).
///         - Non-archive files are removed.
///         - If multiple archived files are found, the submission is rejected, and the directory is removed.
///         - Submissions without any archive file are also rejected.
///     - Extracts the contents of the archive if valid.
///         - Supports archive handling using specific functions for `.zip`, `.rar`, `.7z`, `.tar`, and `.gz` file types.
///         - Cleans up the directory if extraction fails.
/// 3. Sanitizes the extracted submission files:
///     - Removes or replaces invalid/diacritic characters in filenames.
///     - Optionally cleans non-ASCII characters based on the `keep_non_ascii` flag.
/// 4. Logs the total errors and processes all submissions or halts early if `abort_on_err` is set to `true`.
///
/// # Error Handling
/// - Errors can occur in the following scenarios:
///     - Unable to read or access the provided `tmp_dir` directory.
///     - Submission directories containing invalid or non-archive files.
///     - Multiple archive files found within a directory.
///     - Failure during archive extraction.
///     - Failure during sanitization or non-ASCII cleaning.
/// - All such errors are either logged or included in the returned error list (`Ok(Vec<color_eyre::eyre::Error>)`).
///
/// # Logging
/// - The function logs details about its operations at various levels (INFO, DEBUG, TRACE).
/// - Examples include processing directories, detecting archive types, and detailing errors.
///
/// # Panics
/// This function does not panic. All errors are captured using `color_eyre::eyre::Result` and encapsulated for handling.
///
/// # Note
/// - The function assumes that all valid archive files are correctly formatted and extractable.
/// - Submission directories must only contain one valid archive file. Multiple archives are not supported.
#[instrument(skip(keep_non_ascii, abort_on_err))]
fn prepare<P>(
    tmp_dir: P,
    keep_non_ascii: bool,
    abort_on_err: bool,
) -> Result<Vec<color_eyre::eyre::Error>>
where
    P: AsRef<Path> + Debug,
{
    info!("Extracting individual submissions");
    let tmp_dir = tmp_dir.as_ref();

    let mut processed_cnt = 0;
    let mut errs = vec![];
    let mut workers = vec![];

    'outer: for dir in
        fs::read_dir(tmp_dir).with_context(|| format!("Unable to read {tmp_dir:?}"))?
    {
        let dir = dir.with_context(|| format!("Unable to read a dir in {tmp_dir:?}"))?;
        let student_name_dir_path = dir.path();
        let span =
            span!(Level::INFO, "processing submissions", submission = ?student_name_dir_path);
        let _guard = span.enter();
        debug!("Processing student submission");

        if !student_name_dir_path.is_dir() {
            trace!("Found non dir");
            handle_sub_err!(
                "Everything in {tmp_dir:?} should be a dir, found {student_name_dir_path:?}",
                fs::remove_file(&student_name_dir_path),
                errs,
                abort_on_err
            );
            continue;
        }

        let mut archive_file = None;
        let mut fun: fn(_, _, _) -> Result<()> = archive_handler::dummy;
        for archive in WalkDir::new(&student_name_dir_path) {
            let archive =
                archive.with_context(|| format!("Invalid archive in {student_name_dir_path:?}"))?;
            let archive_file_path = archive.path();

            let span = span!(Level::INFO, "student archive", ?archive_file_path);
            let _guard = span.enter();
            trace!("Processing file for student");

            if archive.path().is_dir() {
                trace!("Archive is dir, skipping");
                continue;
            }

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
                    trace!("Found non archive file {archive:?}, removing");
                    fs::remove_file(&archive_file_path).with_context(|| {
                        format!(
                            "Unable to remove non archive file \
                            {archive:?}"
                        )
                    })?;
                    continue;
                }
            };
            processed_cnt += 1;
            if let Some(file) = archive_file {
                debug!("Multiple archives found");
                handle_sub_err!(
                    "Found at least two archive files for student {student_name_dir_path:?}, \
                        expected one:\n\
                        \tFirst: {file:?}\n\
                        \tSecond: {archive_file_path:?}",
                    fs::remove_dir_all(&student_name_dir_path),
                    errs,
                    abort_on_err
                );
                continue 'outer;
            }
            archive_file = Some(archive_file_path.to_owned());
        }

        let Some(archive_file) = archive_file else {
            debug!("No archive found");
            handle_sub_err!(
                "No archive for student {student_name_dir_path:?}",
                fs::remove_dir_all(&student_name_dir_path),
                errs,
                abort_on_err
            );
            continue;
        };

        // CONSIDER Add sender receiver to send errors. Every thread gets sender, later we collect after joining
        let tmp_dir = tmp_dir.to_owned();
        let handle = thread::spawn(move || {
            // Fuck it, don't want to fight the compiler because it picks a lifetime for references, this will not be the bottleneck
            // Btw. I was right, the multithreading as is cut the time of `prepare` from 11.6 to 4.5 seconds
            let res = fun(tmp_dir, student_name_dir_path.clone(), archive_file.clone());
            (res, student_name_dir_path, archive_file)
        });
        workers.push(handle);
    }

    for worker in workers {
        let (res, student_name_dir_path, archive_file) = worker
            .join()
            .map_err(|e| anyhow!("Unable to join worker: {e:?}"))?;
        if let Err(e) = res {
            debug!(?e, "Error extracting {archive_file:?}");
            handle_sub_err!(
                "Error extracting {archive_file:?} \
                         for {student_name_dir_path:?}: {e:?}",
                fs::remove_file(&student_name_dir_path),
                errs,
                abort_on_err
            );
        }
    }

    info!("Unzipped all submissions, Sanitizing output files");
    helper::sanitize_submissions(&tmp_dir).with_context(|| "Unable to sanitize output files")?;

    info!("Sanitized output files, replacing diacritics");
    helper::clean_non_ascii(&tmp_dir, keep_non_ascii)
        .with_context(|| "Unable to replace diacritics")?;

    let err_cnt = errs.len();

    match processed_cnt {
        0 => bail!("processed zero entries"),
        1 => info!("processed one entry"),
        n => info!("processed {n} entries"),
    }
    match processed_cnt - err_cnt {
        0 => bail!("no successful preparations"),
        1 => info!("successfully prepared one submission"),
        n => info!("successfully prepared {n} submissions"),
    }
    match err_cnt {
        0 => {}
        1 => warn!("There was 1 error"),
        n => warn!("There were {n} errors"),
    }

    Ok(errs)
}

/// Runs JPlag with the specified arguments and logs the results.
#[instrument(skip(jplag_jar, jplag_args))]
fn run(result_dir: &str, jplag_jar: &str, jplag_args: &Vec<String>) -> Result<()> {
    let mut jplag_cmd = format!("java -jar {jplag_jar}");

    for str in jplag_args {
        jplag_cmd.push_str(&format!(" {str}"));
    }

    info!(cmd = jplag_cmd, "Starting jplag");
    let span = debug_span!("execute", cmd = jplag_cmd);
    let _guard = span.enter();

    let mut child = Command::new("java")
        .arg("-jar")
        .arg(&jplag_jar)
        .args(jplag_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("Unable to run jplag command {jplag_cmd}"))?;

    helper::listen_for_output(&mut child)
        .with_context(|| format!("Unable to listen to stdout of jplag, cmd: {jplag_cmd}"))?;

    info!("Finished running jplag");

    let status = child
        .wait()
        .with_context(|| format!("Unable to wait for child process {jplag_cmd:?}"))?;

    if !status.success() {
        warn!("Command failed, {status}");
        warn!("To debug manually, run \"{jplag_cmd}\" in the current directory");
        // Do not clean up on purpose, wwe want to see what caused the error
        bail!("Java jplag command failed, {status}");
    } else {
        debug!("{status}");
        let current_dir = env::current_dir().context("Unable to get current dir")?;
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
        Ok(())
    }
}
