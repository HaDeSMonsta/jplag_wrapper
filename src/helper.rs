use std::{fs, io};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Child;
use anyhow::Context;
use tracing::{debug, info, warn};
use walkdir::WalkDir;
use zip::ZipArchive;

pub fn unzip_to<P, Q>(zip: P, dest: Q) -> anyhow::Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path>,
{
    debug!("Unzipping {} to {}", zip.as_ref().display(), dest.as_ref().display());
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

        let out_path = dest
            .as_ref()
            .join(file.enclosed_name().unwrap());

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

            io::copy(&mut file, &mut out_file)
                .with_context(|| format!("Unable to io copy {src} to {out_file:?}",
                                         src = file.name()))?;
            debug!("IO copied {src} to {out_file:?}", src = file.name());
        }
    }

    Ok(())
}

/// Fuck Apple
pub fn sanitize_submissions<P>(path: P) -> anyhow::Result<()>
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
        fs::remove_file(&entry)
            .with_context(|| format!("Unable to remove {entry:?}"))?;
    }

    /*for entry in fs::read_dir(&path)
        .with_context(|| format!("Unable to read dir: {path:?}"))? {
        let entry = entry.with_context(|| format!("Invalid entry in {path:?}"))?;
        let mac_dir = entry.path().join("__MACOSX");
        let _ = fs::remove_dir_all(&mac_dir);
    }*/

    Ok(())
}

pub fn listen_for_output(program: &mut Child, ignore_output: bool) -> anyhow::Result<()> {
    match program.stdout {
        Some(ref mut out) => {
            let reader = BufReader::new(out);

            #[cfg(not(feature = "legacy"))]
            let mut warn = false;
            for line in reader.lines() {
                let line = line
                    .with_context(|| "Unable to parse line from jplag")?;
                if ignore_output { continue; }
                let lower = line.to_lowercase();

                #[cfg(not(feature = "legacy"))]
                if warn {
                    warn!("{line}");
                    if lower.contains("^") { warn = false; }
                    continue;
                }

                if lower.contains("error") ||
                    lower.contains("warn") ||
                    lower.contains("fail") {
                    // Yes, jplag sends it errors to stdout
                    #[cfg(not(feature = "legacy"))]
                    if line.contains("error:") {
                        warn = true;
                    }

                    warn!("{line}");
                    continue;
                }
                if lower.contains("submissions") {
                    info!("{line}");
                    continue;
                }
                debug!("{line}");
            }
        }
        None => warn!("No output :("),
    }
    Ok(())
}
