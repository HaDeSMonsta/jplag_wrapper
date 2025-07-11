use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::Shell::*;
use clap_complete::generate_to;
use std::fs;

const BINARY_NAME: &str = env!("CARGO_PKG_NAME");

include!("src/conf/args.rs");
// Somehow, this breaks rr (It acts, as if args.rs wouldn't exist in the normal project)
// But only, if you open the project the first time when the build.rs file already exists
// To solve, first run this
// mv build.rs build.rs.off
// Make sure rr rebuilds/the error is gone
// mv build.rs.off build.rs
// Now it should work

const COMPLETIONS_OUT_DIR: &str = "completions/";

fn main() -> Result<()> {
    generate_completions().with_context(|| {
        format!(
            "Unable to generate completions and write to \
            {COMPLETIONS_OUT_DIR}"
        )
    })?;

    Ok(())
}

pub fn generate_completions() -> Result<()> {
    fs::create_dir_all(COMPLETIONS_OUT_DIR)
        .with_context(|| format!("Unable to create completions directory {COMPLETIONS_OUT_DIR}"))?;

    for shell in [Bash, Fish, Zsh, Elvish, PowerShell] {
        let mut cmd = Args::command();
        generate_to(shell, &mut cmd, BINARY_NAME, COMPLETIONS_OUT_DIR)
            .with_context(|| format!("Unable to generate completions for shell {shell}"))?;
    }

    Ok(())
}
