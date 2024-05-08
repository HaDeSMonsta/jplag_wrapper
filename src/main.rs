use std::{env, error, fs, io};
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use zip::ZipArchive;

const TMP_DIR: &'static str = "./temp/";

fn main() -> Result<(), Box<dyn error::Error>> {
    let target_dir = load_env();
    rm_tmp_dir();
    prepare_subs()?;

    Ok(())
}

fn load_env() -> String {
    dotenv::dotenv()
        .expect("Unable to finde \".env\" file");

    let target_dir = env::var("TARGET_DIR")
        .expect("TARGET_DIR must be set");
    target_dir
}

fn prepare_subs() -> Result<(), Box<dyn error::Error>> {
    let root = Path::new("./");
    let tmp_dir_root = Path::new(TMP_DIR);
    generate_root(root)?;
    unzip_r(&tmp_dir_root)?;

    Ok(())
}

fn generate_root(dir: &Path) -> Result<(), Box<dyn error::Error>> {
    let mut zip_file = None;

    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.path().extension() == Some(OsStr::new("zip")) {
            if zip_file.is_some() {
                return Err(Box::new(io::Error::new(
                    io::ErrorKind::Other,
                    "Multiple zip files found!",
                )));
            }
            zip_file = Some(entry.path().to_path_buf());
        }
    }

    let zip_file = zip_file.ok_or("No zip files found!")?;
    fs::create_dir_all(TMP_DIR)?;

    // TODO remove duplication
    let file = File::open(&zip_file)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = Path::new(TMP_DIR).join(file.enclosed_name()
                                                   .expect("Unable to get sanitized name!"));

        if (&*file.name()).ends_with("/") {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(p) = out_path.parent() {
            if !p.exists() { fs::create_dir_all(&p)?; }
        }

        let mut out_file = fs::File::create(&out_path)?;
        io::copy(&mut file, &mut out_file)?;
    }


    Ok(())
}

fn unzip_r(dir: &Path) -> Result<(), Box<dyn error::Error>> {
    let mut paths = vec![dir.to_path_buf()];
    
    let zip_extension = Some(OsStr::new("zip"));
    
    for entry in WalkDir::new(&dir) {
        let entry = entry?;
        if entry.path().extension() == zip_extension {
            paths.push(entry.into_path());
        }
    }
    
    while !paths.is_empty() {
        let new_paths = unzip(
            paths
                .pop()
                .unwrap()
                .as_path()
        )?;
        paths.extend(new_paths);
    }
    Ok(())
}

fn unzip(dir: &Path) -> Result<Vec<PathBuf>, Box<dyn error::Error>> {
    let mut unzipped_files = vec![];
    let zip_extension = Some(OsStr::new("zip"));

    let file = File::open(dir)?;
    let mut archive = ZipArchive::new(file)?;
    let parent_dir = dir.parent().expect("No parent dir (how?)");

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

        let out_path = file.enclosed_name()
                           .expect("Unable to get sanitized name!");

        let full_out_path = parent_dir.join(&out_path);

        if (&*file.name()).ends_with("/") {
            fs::create_dir_all(&full_out_path)?;
            continue;
        }

        if let Some(p) = full_out_path.parent() {
            if !p.exists() { fs::create_dir_all(p)?; }
        }
        let mut out_file = File::create(&full_out_path)?;
        io::copy(&mut file, &mut out_file)?;

        if full_out_path.extension() == zip_extension {
            unzipped_files.push(full_out_path);
        }
    }

    Ok(unzipped_files)
}

fn rm_tmp_dir() {
    let _ = fs::remove_dir_all(TMP_DIR);
}
