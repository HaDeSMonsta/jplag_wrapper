use std::{env, error};

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
    todo!()
}