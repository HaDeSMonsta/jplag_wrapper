use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use anyhow::{bail, Context, Result};
use flate2::read::GzDecoder;
use tracing::{debug, instrument};
use crate::helper;

// tmp dir: tmp/
// Student name dir path: tmp/name/
// archive file path: tmp/name/archive
// zip dir name: name/

#[instrument]
pub fn zip<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
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

    debug!("Successfully decompressed, removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    debug!("Successfully removed source");

    Ok(())
}

#[instrument]
pub fn rar<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
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

    let dest = tmp_dir.join(format!("{rar_dir_name}/"));

    fs::create_dir_all(&dest)
        .with_context(|| format!("Unable to create dest dir {dest:?}"))?;

    while let Some(header) = archive.read_header()
                                    .with_context(|| format!("Unable to read header of {archive_file_path:?}"))? {
        let src_name = header.entry().filename.to_string_lossy().to_string();
        let dest_name = format!("{}/{src_name}", dest.display());
        debug!("{} bytes: {src_name}", header.entry().unpacked_size);

        archive = if header.entry().is_file() {
            debug!("Unpacking {} to {dest_name}", format!("{}{src_name}", tmp_dir.display()));
            header.extract_to(&dest_name)
                  .with_context(|| format!("Unable to unrar {src_name} to {dest_name}"))?
        } else {
            debug!("Skipping {src_name}, is dir");
            header.skip()
                  .with_context(|| format!("Unable to skip rar {src_name}"))?
        }
    }

    debug!("Successfully unrawred, removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    debug!("Successfully removed source");

    Ok(())
}

#[instrument(skip(_tmp_dir))]
pub fn sz<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    debug!("Extracting 7z archive");

    sevenz_rust::decompress_file(archive_file_path, student_name_dir_path)
        .with_context(|| format!("Unable to decompress {student_name_dir_path:?}"))?;
    
    debug!("Successfully decompressed, removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?} after extracting"))?;

    debug!("Successfully removed source");
    
    Ok(())
}

#[instrument(skip(_tmp_dir))]
pub fn tar<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    debug!("Untaring archive");

    tar::Archive::new(
        BufReader::new(
            File::open(&archive_file_path)
                .with_context(|| format!("Unable to open tar {archive_file_path:?}"))?
        )).unpack(&student_name_dir_path)
          .with_context(|| format!("Unable to untar {archive_file_path:?} \
        into {student_name_dir_path:?}"))?;

    debug!("Successfully untared, removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    debug!("Successfully removed source");

    Ok(())
}

#[instrument(skip(_tmp_dir))]
pub fn gz<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    debug!("Ungzipping archive");

    tar::Archive::new(
        GzDecoder::new(
            BufReader::new(
                File::open(&archive_file_path)
                    .with_context(|| format!("Unable to open tar.gz file \
                    {archive_file_path:?}"))?
            )
        )
    ).unpack(&student_name_dir_path)
     .with_context(|| format!("Unable to extract {archive_file_path:?} \
        to {student_name_dir_path:?}"))?;

    debug!("Successfully ungzipped, removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("Unable to remove {archive_file_path:?}"))?;

    debug!("Successfully removed source");

    Ok(())
}

#[instrument]
pub fn dummy<P, Q, R>(_tmp_dir: P, _student_name_dir_path: Q, _archive_file_path: R)
    -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
   bail!("Called dummy extract function => `fun` was not initialized")
}
