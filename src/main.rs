use std::{env, error, fs, io};
use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;
use walkdir::WalkDir;
use zip::ZipArchive;

fn main() -> Result<(), Box<dyn error::Error>> {
    let target_dir = load_env();
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
    unzip_r(&root)?;

    Ok(())
}

fn unzip_r(dir: &Path) -> Result<(), Box<dyn error::Error>> {
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.path().extension() == Some(OsStr::new("zip")) {
            unzip(entry.path())?;
            unzip_r(entry.path())?;
        }
    }
    Ok(())
}

fn unzip(dir: &Path) -> Result<(), Box<dyn error::Error>> {

    let file = File::open(dir)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        
        let out_path = file.enclosed_name()
            .expect("Unable to get sanitized name");
        
        if (&*file.name()).ends_with("/") {
            fs::create_dir_all(out_path)?;
            return Ok(());
        }
        
        if let Some(p) = out_path.parent() {
            if !p.exists() {fs::create_dir_all(p)?;}
        }
        let mut out_file = fs::File::create(&out_path)?;
        io::copy(&mut file, &mut out_file)?;
    }

    Ok(())
}
