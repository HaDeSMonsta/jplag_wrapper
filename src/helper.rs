use color_eyre::Result;
use color_eyre::eyre::{Context, ContextCompat, bail};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{fs, io};
use tracing::{Level, debug, info_span, instrument, span, trace, warn};
use walkdir::WalkDir;
use zip::ZipArchive;

#[instrument]
pub fn check_java_executable() -> Result<()> {
    const CMD: &str = "java";
    const ARG: &str = "--version";

    let mut child = Command::new(CMD)
        .arg(ARG)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| format!("unable to start to run `{CMD} {ARG}`"))?;

    trace!("spawned child");

    if child
        .wait()
        .with_context(|| format!("unable to wait for`{CMD} {ARG}`"))?
        .success()
    {
        Ok(())
    } else {
        bail!("unable to run `{CMD} {ARG}`, {CMD} is probably not installed");
    }
}

#[instrument]
pub fn unzip_to<P, Q>(zip: P, dest: Q) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
{
    trace!("unzipping archive");
    let src_file = OpenOptions::new()
        .read(true)
        .open(&zip)
        .with_context(|| format!("unable to open src_file: {zip:?}"))?;

    trace!("opened zip");

    let mut archive = ZipArchive::new(BufReader::new(src_file))
        .with_context(|| format!("unable to parse {zip:?} to a zip archive"))?;

    trace!("created zip archive");

    let archive_len = archive.len();
    trace!("archive len: {archive_len}");

    for i in 0..archive_len {
        let mut file = archive.by_index(i).with_context(|| {
            format!(
                "unable to get file by index {i} \
                (should be impossible, as we iterate over len, len = {archive_len})"
            )
        })?;
        let span = span!(Level::DEBUG, "processing_file", file_name = %file.name());
        let _guard = span.enter();

        let out_path = dest.as_ref().join(file.enclosed_name().unwrap());

        trace!("set out path: {out_path:?}");

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("unable to create out dir: {out_path:?}"))?;
            trace!("created out_path");
        } else {
            if let Some(parent) = out_path.parent()
                && !parent.exists()
            {
                fs::create_dir_all(parent)
                    .with_context(|| format!("unable to create parent dir: {parent:?}"))?;
                trace!("created parent");
            }

            let mut out_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&out_path)
                .with_context(|| format!("unable to open/create out_file: {out_path:?}"))?;

            trace!("created/opened out_file {out_file:?}");

            io::copy(&mut file, &mut out_file).with_context(|| {
                format!("unable to io copy {src} to {out_file:?}", src = file.name())
            })?;
            trace!("io copied {src} to {out_file:?}", src = file.name());
        }
    }

    Ok(())
}

#[instrument]
pub fn add_subs<P>(sub_dir_vec: &Vec<String>, tmp_dir: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    let tmp_dir = tmp_dir.as_ref();
    debug!("adding additional submissions"); // CONSIDER Info
    for dir in sub_dir_vec {
        trace!("processing {dir}");
        if !fs::exists(dir).with_context(|| format!("unable to check if {dir} exists"))? {
            bail!("{dir} doesn't exist");
        }
        if !PathBuf::from(dir).is_dir() {
            bail!("{dir} is not a directory");
        }

        trace!("{dir} exists and is a dir, copying");

        let tmp_root = tmp_dir.join(&dir);
        fs::create_dir_all(&tmp_root).with_context(|| format!("unable to create {tmp_root:?}"))?;

        for entry in WalkDir::new(&dir) {
            let entry = entry.with_context(|| format!("error processing entry in {dir}"))?;
            let src_path = entry.path();
            let dest_path = tmp_dir.join(&src_path);

            trace!("copying {src_path:?} to {dest_path:?}");

            if src_path.is_dir() {
                fs::create_dir_all(&dest_path)
                    .with_context(|| format!("unable to create path {dest_path:?}"))?;
            } else {
                fs::copy(&src_path, &dest_path)
                    .with_context(|| format!("unable to copy {src_path:?} to {dest_path:?}"))?;
            }
        }
    }

    Ok(())
}

/// Fuck Apple
#[instrument(skip_all)]
pub fn sanitize_submissions<P>(path: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    #[cfg(feature = "minimal_rms")]
    const TO_REM_DIRS: &[&str] = &["__MACOSX", "target", "build"];
    #[cfg(feature = "minimal_rms")]
    const TO_REM_FILES: &[&str] = &[".DS_STORE"];
    #[cfg(not(feature = "minimal_rms"))]
    const TO_REM_DIRS: &[&str] = &[
        "__MACOSX",
        ".idea",
        "target",
        "build",
        "gradle",
        ".git",
        "out",
        "Prog1Tools", // Extracted Prog1Tools
    ];
    #[cfg(not(feature = "minimal_rms"))]
    const TO_REM_FILES: &[&str] = &[
        ".DS_STORE",
        ".gitignore",
        "gradlew",
        "gradlew.bat",
        "build.gradle.kts",
        "settings.gradle.kts",
        "pom.xml",
        ".md",
        ".iml",
        ".zip",   // Prog1Tools/templates/submissions
        ".class", // Extracted Prog1Tools
        ".mp3",
    ];

    debug!("removing files");

    'outer: for entry in WalkDir::new(&path) {
        let entry = entry.with_context(|| format!("invalid entry in {path:?}"))?;
        let path = entry.path();
        let is_dir = path.is_dir();
        let span = info_span!("checking file", ?path, is_dir);
        let _enter = span.enter();
        if is_dir {
            for dir in TO_REM_DIRS {
                if path.ends_with(dir) {
                    trace!("found match to remove");
                    fs::remove_dir_all(path)
                        .with_context(|| format!("unable to remove {path:?}"))?;
                    continue 'outer;
                }
            }
        } else {
            for file in TO_REM_FILES {
                // path.ends_with() only considers while parts, so we can't match extensions **and** file names with it
                if path
                    .to_str()
                    .with_context(|| format!("invalid file name: {path:?}"))?
                    .ends_with(file)
                {
                    trace!("found match to remove");
                    fs::remove_file(path).with_context(|| format!("unable to remove {path:?}"))?;
                    continue 'outer;
                }
            }
        }
        trace!("no match found");
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn listen_for_output(program: &mut Child) -> Result<()> {
    if let Some(ref mut out) = program.stdout {
        let reader = BufReader::new(out);

        for line in reader.lines() {
            let line = line.with_context(|| "unable to parse line from jplag")?;
            println!("{line}");
        }
    } else {
        warn!("no output :(");
    }
    Ok(())
}
