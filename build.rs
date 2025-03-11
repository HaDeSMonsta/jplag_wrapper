use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::generate_to;
use clap_complete::Shell::*;
use std::fs;

include!("src/conf/args.rs");

const COMPLETIONS_OUT_DIR: &str = "completions/";

fn main() -> Result<()> {
    generate_completions()
        .with_context(|| format!("Unable to generate completions and write to \
            {COMPLETIONS_OUT_DIR}"))?;

    Ok(())
}

pub fn generate_completions() -> Result<()> {
    fs::create_dir_all(COMPLETIONS_OUT_DIR)
        .with_context(|| format!("Unable to create completions directory {COMPLETIONS_OUT_DIR}"))?;

    for shell in [Bash, Fish, Zsh, Elvish, PowerShell] {
        let mut cmd = Args::command();
        generate_to(
            shell,
            &mut cmd,
            BINARY_NAME,
            COMPLETIONS_OUT_DIR,
        )
            .with_context(|| format!("Unable to generate completions for shell {shell}"))?;
    }

    Ok(())
}
