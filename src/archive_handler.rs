use crate::helper;
use color_eyre::{
    Result,
    eyre::{Context, ContextCompat},
};
use flate2::read::GzDecoder;
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::hint::unreachable_unchecked;
use std::io::BufReader;
use std::path::Path;
use tracing::{debug, instrument, trace};

// tmp dir: tmp/
// Student name dir path: tmp/name/
// archive file path: tmp/name/archive
// zip dir name: name/

// Both are set in a span before calling one of these functions
#[instrument(skip(tmp_dir, student_name_dir_path))]
pub fn zip<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("processing");
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    let zip_dir_name = student_name_dir_path
        .file_name()
        .with_context(|| format!("unable to get file name of {student_name_dir_path:?}"))?;

    let dest = tmp_dir.join(&zip_dir_name);

    trace!("set destination of unzipped file to {dest:?}");

    fs::create_dir_all(&dest).with_context(|| format!("unable to create {dest:?}"))?;

    trace!("created {dest:?}");

    helper::unzip_to(&archive_file_path, &dest)
        .with_context(|| format!("unable to unzip {archive_file_path:?} to {dest:?}"))?;

    debug!("successfully decompressed");
    trace!("removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("unable to remove {archive_file_path:?}"))?;

    trace!("successfully removed source");

    Ok(())
}

#[instrument(skip(tmp_dir, student_name_dir_path))]
pub fn rar<P, Q, R>(tmp_dir: P, student_name_dir_path: Q, archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("processing");
    let tmp_dir = tmp_dir.as_ref();
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    let mut archive = unrar::Archive::new(&archive_file_path)
        .open_for_processing()
        .with_context(|| format!("unable to open {archive_file_path:?}"))?;

    let rar_dir_name = student_name_dir_path
        .file_name()
        .and_then(|f| f.to_str())
        .with_context(|| format!("unable to get file name of {student_name_dir_path:?}"))?;

    let dest = tmp_dir.join(format!("{rar_dir_name}/"));

    fs::create_dir_all(&dest).with_context(|| format!("unable to create dest dir {dest:?}"))?;

    while let Some(header) = archive
        .read_header()
        .with_context(|| format!("unable to read header of {archive_file_path:?}"))?
    {
        let src_name = header.entry().filename.to_string_lossy().to_string();
        let dest_name = format!("{}/{src_name}", dest.display());
        trace!("{} bytes: {src_name}", header.entry().unpacked_size);

        archive = if header.entry().is_file() {
            trace!("unpacking {}{src_name} to {dest_name}", tmp_dir.display());
            header
                .extract_to(&dest_name)
                .with_context(|| format!("unable to unrar {src_name} to {dest_name}"))?
        } else {
            trace!("skipping {src_name}, is dir");
            header
                .skip()
                .with_context(|| format!("unable to skip rar {src_name}"))?
        }
    }

    debug!("successfully unrawred");
    trace!("removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("unable to remove {archive_file_path:?}"))?;

    trace!("successfully removed source");

    Ok(())
}

#[instrument(skip(_tmp_dir, student_name_dir_path))]
pub fn sz<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("processing");
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    sevenz_rust::decompress_file(archive_file_path, student_name_dir_path)
        .with_context(|| format!("unable to decompress {student_name_dir_path:?}"))?;

    debug!("successfully decompressed");
    trace!("removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("unable to remove {archive_file_path:?} after extracting"))?;

    trace!("successfully removed source");

    Ok(())
}

#[instrument(skip(_tmp_dir, student_name_dir_path))]
pub fn tar<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("processing");
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    tar::Archive::new(BufReader::new(
        File::open(&archive_file_path)
            .with_context(|| format!("unable to open tar {archive_file_path:?}"))?,
    ))
    .unpack(&student_name_dir_path)
    .with_context(|| {
        format!(
            "unable to untar {archive_file_path:?} \
            into {student_name_dir_path:?}"
        )
    })?;

    debug!("successfully untared");
    trace!("removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("unable to remove {archive_file_path:?}"))?;

    trace!("successfully removed source");

    Ok(())
}

#[instrument(skip(_tmp_dir, student_name_dir_path))]
pub fn gz<P, Q, R>(_tmp_dir: P, student_name_dir_path: Q, archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    debug!("processing");
    let student_name_dir_path = student_name_dir_path.as_ref();
    let archive_file_path = archive_file_path.as_ref();

    tar::Archive::new(GzDecoder::new(BufReader::new(
        File::open(&archive_file_path).with_context(|| {
            format!(
                "unable to open tar.gz file \
                {archive_file_path:?}"
            )
        })?,
    )))
    .unpack(&student_name_dir_path)
    .with_context(|| {
        format!(
            "unable to extract {archive_file_path:?} \
            to {student_name_dir_path:?}"
        )
    })?;

    debug!("successfully ungzipped");
    trace!("removing source");

    fs::remove_file(&archive_file_path)
        .with_context(|| format!("unable to remove {archive_file_path:?}"))?;

    trace!("successfully removed source");

    Ok(())
}

#[instrument]
pub fn dummy<P, Q, R>(_tmp_dir: P, _student_name_dir_path: Q, _archive_file_path: R) -> Result<()>
where
    P: AsRef<Path> + Debug,
    Q: AsRef<Path> + Debug,
    R: AsRef<Path> + Debug,
{
    unsafe {
        unreachable_unchecked();
    }
}
