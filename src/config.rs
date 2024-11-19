use std::cell::LazyCell;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::process::exit;
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, Level};

const ARGS: LazyCell<Args> = LazyCell::new(|| Args::parse());
const DEFAULT_CONFIG_FILE: &str = "config.toml";
const DEFAULT_SOURCE_FILE: &str = "submissions.zip";
const DEFAULT_TARGET_DIR: &str = "out/";
const DEFAULT_TMP_DIR: &str = "tmp/";


/// TODO Lol
#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Initialize the config,
    /// will create or override(!) `config.toml`
    /// and fill it with the default values
    #[clap(short, long)]
    init: bool,
    /// Specify the config toml file to look for
    ///
    /// Will panic, if file does not exist
    #[clap(short, long)]
    config: Option<String>,
    /// Where the input file can be found
    ///
    /// Defaults to `submissions.zip`
    #[clap(short, long)]
    source: Option<String>,
    /// Where to put the results
    ///
    /// Defaults to `out/`
    ///
    /// Warning, this directory will be deleted at application start, if it exists
    #[clap(short, long)]
    target_dir: Option<String>,
    /// Where to put the temporary files
    ///
    /// Defaults to `tmp/`
    ///
    /// Warning, this directory will be deleted at application start, if it exists
    #[clap(long)]
    tmp_dir: Option<String>,
    /// Set to use log level `debug`
    ///
    /// Otherwise, `info` will be used
    #[clap(short, long)]
    debug: bool,
    _remaining: Vec<String>,
    #[clap(last = true)]
    _ignored: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    source: Option<String>,
    target_dir: Option<String>,
    tmp_dir: Option<String>,
}

pub fn get_log_level() -> Level {
    if ARGS.debug {
        Level::DEBUG
    } else {
        Level::INFO
    }
}

/// Parse args for the bin, prioritizes cli over toml
///
/// Returns: (source, tmp_dir, target_dir)
pub fn parse_args() -> Result<(String, String, String)> {
    debug!("Getting args");
    let args = ARGS.clone();
    if args.init {
        debug!("Initializing config");
        dump_default_config()
            .with_context(|| "Unable to write default config")?;
    };

    let config = parse_toml(
        &args.config.unwrap_or_else(|| DEFAULT_CONFIG_FILE.to_string()),
    ).with_context(|| "Unable to parse toml config")?;

    debug!("Successfully parsed toml");

    let source = args.source
                     .unwrap_or_else(|| {
                         config.source
                               .unwrap_or_else(|| DEFAULT_SOURCE_FILE.to_string())
                     });

    debug!("Set source to {source}");

    let tmp_dir = args.tmp_dir
                      .unwrap_or_else(|| {
                          config.tmp_dir
                                .unwrap_or_else(|| DEFAULT_TMP_DIR.to_string())
                      });

    debug!("Set tmp_dir to {tmp_dir}");

    let target_dir = args.target_dir
                         .unwrap_or_else(|| {
                             config.target_dir
                                   .unwrap_or_else(|| DEFAULT_TARGET_DIR.to_string())
                         });

    debug!("Set target dir to {target_dir}");

    info!("Successfully parsed config");

    Ok((source, tmp_dir, target_dir))
}

fn parse_toml(file: &str) -> Result<Config> {
    debug!("Parsing toml, source: {file}");
    if !fs::exists(&file)
        .with_context(|| format!("Unable to check if {file} exists"))? {
        debug!("{file} does not exist");
        if file != DEFAULT_CONFIG_FILE {
            panic!("{file} does not exist");
        }
        debug!("Returning empty config");
        return Ok(Config {
            source: None,
            target_dir: None,
            tmp_dir: None,
        });
    }
    let toml = fs::read_to_string(&file)
        .with_context(|| format!("Failed to read from config file {file}"))?;
    debug!("Parsing toml, raw: {toml}");
    Ok(
        toml::from_str::<Config>(&toml)
            .with_context(|| format!(
                "Unable to parse to Config, raw string:\
            \n\"\"\"\
            {toml}\
            \"\"\"\
            "))?
    )
}

fn dump_default_config() -> Result<()> {
    let conf = Config {
        source: Some(String::from(DEFAULT_SOURCE_FILE)),
        target_dir: Some(String::from(DEFAULT_TARGET_DIR)),
        tmp_dir: Some(String::from(DEFAULT_TMP_DIR)),
    };
    debug!("Created default config struct");
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(DEFAULT_CONFIG_FILE)
        .with_context(|| format!("Failed to open/create/truncate config file: {DEFAULT_CONFIG_FILE}"))?;
    debug!("Opened default config file");

    let mut writer = BufWriter::new(file);

    writeln!(writer, "{}", toml::to_string_pretty(&conf)?)
        .with_context(|| format!("Unable to write default config to {DEFAULT_CONFIG_FILE}"))?;

    info!("Created default config");

    exit(0);
}
