use anyhow::{Context, Result, anyhow};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{fs, io};
use tracing::{debug, warn};
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn check_java_executable() -> Result<()> {
    let mut child = Command::new("java")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "Unable to start to run `java --version`")?;

    if child
        .wait()
        .with_context(|| "Unable to wait for `java --version`")?
        .success()
    {
        Ok(())
    } else {
        Err(anyhow!(
            "Unable to run `java --version`, java is probably not installed"
        ))
    }
}

pub fn unzip_to<P, Q>(zip: P, dest: Q) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path>,
{
    debug!(
        "Unzipping {} to {}",
        zip.as_ref().display(),
        dest.as_ref().display()
    );
    let src_file = OpenOptions::new()
        .read(true)
        .open(&zip)
        .with_context(|| format!("Unable to open src_file: {zip:?}"))?;

    debug!("Opened {zip:?}");

    let mut archive = ZipArchive::new(BufReader::new(src_file))
        .with_context(|| format!("Unable to parse {zip:?} to a ZipArchive"))?;

    debug!("Created zip archive");

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        debug!("Processing file: {}", file.name());

        let out_path = dest.as_ref().join(file.enclosed_name().unwrap());

        debug!("Set out path: {out_path:?}");

        if file.is_dir() {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("Unable to create out dir: {out_path:?}"))?;
            debug!("Created out_path");
        } else {
            if let Some(parent) = out_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .with_context(|| format!("Unable to create parent dir: {parent:?}"))?;
                    debug!("Created parent");
                }
            }
            let mut out_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&out_path)
                .with_context(|| format!("Unable to open/create out_file: {out_path:?}"))?;

            debug!("Created/opened out_file {out_file:?}");

            io::copy(&mut file, &mut out_file).with_context(|| {
                format!("Unable to io copy {src} to {out_file:?}", src = file.name())
            })?;
            debug!("IO copied {src} to {out_file:?}", src = file.name());
        }
    }

    Ok(())
}

pub fn add_subs<P>(sub_dir_vec: &Vec<String>, tmp_dir: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    let tmp_dir = tmp_dir.as_ref();
    debug!("Adding additional submissions"); // CONSIDER Info
    for dir in sub_dir_vec {
        debug!("Processing {dir}");
        if !fs::exists(dir).with_context(|| format!("Unable to check if {dir} exists"))? {
            return Err(anyhow!("{dir} doesn't exist"));
        }
        if !PathBuf::from(dir).is_dir() {
            return Err(anyhow!("{dir} is not a directory"));
        }

        debug!("{dir} exists and is a dir, copying");

        let tmp_root = tmp_dir.join(&dir);
        fs::create_dir_all(&tmp_root).with_context(|| format!("Unable to create {tmp_root:?}"))?;

        for entry in WalkDir::new(&dir) {
            let entry = entry.with_context(|| format!("Error processing entry in {dir}"))?;
            let src_path = entry.path();
            let dest_path = tmp_dir.join(&src_path);

            debug!("Copying {src_path:?} to {dest_path:?}");

            if src_path.is_dir() {
                fs::create_dir_all(&dest_path)
                    .with_context(|| format!("Unable to create path {dest_path:?}"))?;
            } else {
                fs::copy(&src_path, &dest_path)
                    .with_context(|| format!("Unable to copy {src_path:?} to {dest_path:?}"))?;
            }
        }
    }

    Ok(())
}

// NOTE The logging in here might be a little bit ambiguous (especially logging all files that aren't a match)
/// Fuck Apple
pub fn sanitize_submissions<P>(path: P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    let path = path.as_ref();
    let mut to_remove = HashSet::new();

    debug!("Removing MACOSX paths");

    for entry in WalkDir::new(&path) {
        let entry = entry.with_context(|| format!("Invalid entry in {path:?}"))?;
        let path = entry.path();

        if !path.is_dir() || path.file_name() != Some(OsStr::new("__MACOSX")) {
            continue;
        }

        debug!("Removing MACOSX path {path:?}");

        fs::remove_dir_all(&path)
            .with_context(|| format!("Unable to remove MACOSX path {path:?}"))?;
    }

    debug!("Removed MACOSX paths, now searching for .DS_Store");

    for entry in WalkDir::new(&path) {
        let entry = entry.with_context(|| format!("Invalid entry in {path:?}"))?;
        let entry_name = entry.path().to_string_lossy().to_lowercase();
        debug!("Checking entry: {entry_name}");

        if entry_name.ends_with(".ds_store") {
            to_remove.insert(entry.path().to_path_buf());
        }
        debug!("No match found")
    }

    debug!("Set to remove: {to_remove:?}");

    for entry in to_remove {
        fs::remove_file(&entry).with_context(|| format!("Unable to remove {entry:?}"))?;
    }

    Ok(())
}

/// Replace diacritics and remove all non ASCII characters
pub fn clean_non_ascii<P>(path: P, keep_non_ascii: bool) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    let replacements = [
        ('Ä', "Ae"),
        ('ä', "ae"),
        ('Ö', "Oe"),
        ('ö', "oe"),
        ('Ü', "Ue"),
        ('ü', "ue"),
        ('ß', "ss"),
    ];

    for entry in WalkDir::new(&path) {
        let entry = entry.with_context(|| format!("Invalid entry in {path:?}"))?;

        let file_path = entry.path();

        if file_path.is_dir()
            || file_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("java"))
                != Some(true)
        {
            continue;
        }

        debug!("Checking {file_path:?} for diacritics");

        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Unable to read {file_path:?}"))?;

        let mut sanitized_content = replacements
            .iter()
            .fold(content.clone(), |acc, &(from, to)| acc.replace(from, to));

        if !keep_non_ascii {
            sanitized_content = sanitized_content.replace(|c: char| !c.is_ascii(), "");
        }

        if sanitized_content == content {
            debug!("{file_path:?} did not contain diacritics");
            continue;
        }

        debug!("{file_path:?} did contained diacritics, replacing content");
        fs::write(&file_path, sanitized_content)
            .with_context(|| format!("Unable to write to file {file_path:?}"))?
    }

    Ok(())
}

pub fn listen_for_output(program: &mut Child) -> Result<()> {
    match program.stdout {
        Some(ref mut out) => {
            let reader = BufReader::new(out);

            for line in reader.lines() {
                let line = line.with_context(|| "Unable to parse line from jplag")?;
                println!("{line}");
            }
        }
        None => warn!("No output :("),
    }
    Ok(())
}
