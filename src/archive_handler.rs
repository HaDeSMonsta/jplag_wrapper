use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Context;
use tracing::debug;
use crate::helper;

// tmp dir: tmp/
// Student name dir path: tmp/name/
// archive file path: tmp/name/archive
// zip dir name: name/

pub fn zip<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> anyhow::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    let zip_dir_name = student_name_dir_path.file_name()
                                            .and_then(|f| f.to_str())
                                            .with_context(|| format!("Unable to get file name of {:?}", student_name_dir_path))?;

    // let zip_target_dir = format!("{zip_dir_name}/out");
    let zip_target_dir = zip_dir_name;
    let dest = tmp_dir.join(&zip_target_dir);

    debug!("Set destination of unzipped file to {dest:?}");

    fs::create_dir_all(&dest)
        .with_context(|| format!("Unable to create {tmp_dir:?}"))?;

    debug!("Created {dest:?}");

    helper::unzip_to(&archive_file_path, &dest)
        .with_context(|| format!("Unable to unzip {archive_file_path:?} to {dest:?}"))?;

    debug!("Unzipped {archive_file_path:?} to {dest:?}");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    debug!("Removed {archive_file_path:?}");

    Ok(())
}

pub fn rar<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> anyhow::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    let mut archive = unrar::Archive::new(&archive_file_path)
        .open_for_processing()
        .with_context(|| format!("Unable to open {archive_file_path:?}"))?;

    let rar_dir_name = student_name_dir_path.file_name()
                                            .and_then(|f| f.to_str())
                                            .with_context(|| format!("Unable to get file name of {:?}", student_name_dir_path))?;

    let dest = tmp_dir.join(format!("{}/", &rar_dir_name));

    fs::create_dir_all(&dest)
        .with_context(|| format!("Unable to create dest dir {dest:?}"))?;

    while let Some(header) = archive.read_header()
                                    .with_context(|| format!("Unable to read header of {archive_file_path:?}"))? {
        let src_name = header.entry().filename.to_string_lossy().to_string();
        let dest_name = format!("{}/{src_name}", dest.display());
        debug!("{} bytes: {}", header.entry().unpacked_size, src_name);

        archive = if header.entry().is_file() {
            debug!("Unpacking {} to {dest_name}", format!("{}{src_name}", tmp_dir.display()));
            header.extract_to(&dest_name)?
        } else {
            debug!("Skipping, is dir");
            header.skip()?
        }
    }

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    Ok(())
}

pub fn sz<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> anyhow::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    todo!("7z not implemented");
}

pub fn tar<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> anyhow::Result<()>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
    R: AsRef<Path>,
{
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    todo!("Tar not implemented");
}
