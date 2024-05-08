use std::{env, error, fs, io, thread};
use std::ffi::OsStr;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;

use walkdir::WalkDir;
use zip::ZipArchive;

fn load_env() -> (String, String) {
    dotenv::dotenv()
        .expect("Unable to finde \".env\" file");

    let target_dir = env::var("TARGET_DIR")
        .expect("TARGET_DIR must be set");
    let temp_dir = env::var("TEMP_DIR").unwrap_or(String::from("temp/"));
    (target_dir, temp_dir)
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let (target_dir, temp_dir) = load_env();
    rm_tmp_dir(&temp_dir);
    prepare_subs(&temp_dir)?;

    run(&target_dir)?;

    rm_tmp_dir(&temp_dir);

    Ok(())
}

fn prepare_subs(temp_dir: &str) -> Result<(), Box<dyn error::Error>> {
    let root = Path::new("./");
    let tmp_dir_root = Path::new(temp_dir);
    generate_root(root, temp_dir)?;
    unzip_r(&tmp_dir_root)?;

    Ok(())
}

fn generate_root(dir: &Path, temp_dir: &str) -> Result<(), Box<dyn error::Error>> {
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
    fs::create_dir_all(temp_dir)?;

    // TODO remove duplication
    let file = File::open(&zip_file)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let out_path = Path::new(temp_dir).join(file.enclosed_name()
                                                    .expect("Unable to get sanitized name!"));

        if (&*file.name()).ends_with("/") {
            fs::create_dir_all(&out_path)?;
            continue;
        }

        if let Some(p) = out_path.parent() {
            if !p.exists() { fs::create_dir_all(&p)?; }
        }

        let mut out_file = File::create(&out_path)?;
        io::copy(&mut file, &mut out_file)?;
    }


    Ok(())
}

fn unzip_r(dir: &Path) -> Result<(), Box<dyn error::Error>> {
    let mut paths = vec![];

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

    // Handle MacOS
    if let Some(path_str) = dir.to_str() {
        let target = "MACOSX";
        if path_str.contains(target) {
            let macos_idx = path_str.find(target).unwrap();
            let macos_path_slice = &path_str[..macos_idx + target.len()];

            let macos_path = Path::new(macos_path_slice);

            if !macos_path.exists() { return Ok(vec![]); }
            if let Err(e) = fs::remove_dir_all(&macos_path) {
                panic!("Unable to remove MacOS-Path: {macos_path:?} with err {e}\n\
                Original path was: {dir:?}");
            }
            return Ok(vec![]);
        }
    }

    let file = File::open(dir)?;
    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Unable to extract {dir:?}: {e}");
            return Ok(vec![]);
        }
    };
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

        // Add to queue
        if full_out_path.extension() == zip_extension {
            // Skip Prog1Tools (and stuff like Prog1Tools (1).zip
            if let Some(p) = full_out_path.to_str() {
                if p.to_lowercase().contains("prog1tools") { continue; }
            }
            unzipped_files.push(full_out_path);
        } else if let Some(s) = full_out_path.to_str() {
            // Make sure to later remove all __MACOSX
            if s.contains("__MACOSX") { unzipped_files.push(full_out_path) }
        }
    }

    fs::remove_file(dir)?;

    Ok(unzipped_files)
}

fn rm_tmp_dir(temp_dir: &str) {
    let _ = fs::remove_dir_all(temp_dir);
}

fn run(target_dir: &str) -> Result<(), Box<dyn error::Error>> {
    let target_str = String::from(target_dir);

    thread::spawn(move || {
       Command::new("java")
           .args(["-jar", ""])
           .spawn()
           .unwrap();
    });

    Ok(())
}
