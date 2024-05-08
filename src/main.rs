use std::{env, error, fs};

fn main() -> Result<(), Box<dyn error::Error>> {
    let target_dir = load_env();

    Ok(())
}

fn load_env() -> String {
    dotenv::dotenv()
        .expect("Unable to finde \".env\" file");
   
    let target_dir = env::var("TARGET_DIR")
        .expect("TARGET_DIR must be set");
    target_dir
}

fn prepare_subs() {
    let current =fs::read_dir("./");
}

fn unzip_r(root: &str) {
    
}