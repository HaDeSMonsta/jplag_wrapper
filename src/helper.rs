use std::{fs, io};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::Child;
use anyhow::Context;
use tracing::{debug, info, warn};
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

pub fn listen_for_output(program: &mut Child) -> anyhow::Result<()> {
    match program.stdout {
        Some(ref mut out) => {
            let reader = BufReader::new(out);
            for line in reader.lines() {
                let line = line.with_context(|| "Unable to parse line from jplag")?;
                if line.to_lowercase().contains("error") {
                    // Yes, jplag sends it errors to stdout
                    warn!("{line}");
                } else if line.to_lowercase().contains("submissions") {
                    info!("{line}");
                } else {
                    debug!("{line}");
                }
            }
        }
        None => warn!("No output :("),
    }
    Ok(())
}
