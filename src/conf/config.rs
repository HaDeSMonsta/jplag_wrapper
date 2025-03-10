use std::fmt::Debug;
use std::{fs, io};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::process::exit;
use std::sync::LazyLock;
use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
#[cfg(not(debug_assertions))]
use tracing::Level;
use crate::conf::args;
use crate::conf::args::Args;

const DEFAULT_CONFIG_FILE: &str = "config.toml";
const DEFAULT_SOURCE_FILE: &str = "submissions.zip";
const DEFAULT_JPLAG_FILE: &str = "jplag.jar";
const DEFAULT_TARGET_DIR: &str = "out/";
const DEFAULT_TMP_DIR: &str = "tmp/";
#[cfg(not(feature = "legacy"))]
const DEFAULT_RES_ZIP: &str = "results.zip";
#[cfg(not(feature = "legacy"))]
const DEFAULT_JAVA_VERSION: &str = "java";
#[cfg(feature = "legacy")]
const DEFAULT_JAVA_VERSION: &str = "java19";

static ARGS: LazyLock<Args> = LazyLock::new(|| Args::parse());

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    source_zip: Option<String>,
    target_dir: Option<String>,
    tmp_dir: Option<String>,
    ignore_file: Option<String>,
    jplag_jar: Option<String>,
    jplag_args: Option<Vec<String>>,
}

#[cfg(not(debug_assertions))]
pub fn get_log_level() -> Level {
    if ARGS.debug() {
        Level::DEBUG
    } else {
        Level::INFO
    }
}

/// Parse args for the bin, prioritizes cli over toml
///
/// Returns: (source, tmp_dir, preserve_tmp_dir, target_dir, keep_non_ascii,
/// jplag_jar, jplag_args, ignore_jplag_output, additional_submission_dirs)
pub fn parse_args() -> Result<(String, String, bool, String, bool, String, Vec<String>, bool, Vec<String>)> {
    debug!("Getting args");
    if ARGS.version() {
        args::version();
    }
    if ARGS.init() {
        debug!("Initializing config");
        dump_default_config()
            .with_context(|| "Unable to write default config")?;
        exit(0);
    };

    let config_name_overridden = ARGS.config().is_some();
    let config = parse_toml(
        &ARGS.config().clone().unwrap_or_else(|| DEFAULT_CONFIG_FILE.to_string()),
        config_name_overridden,
    ).with_context(|| "Unable to parse toml config")?;

    debug!("Successfully parsed toml");

    let source = ARGS.source_zip()
                     .clone()
                     .unwrap_or_else(|| {
                         config.source_zip
                               .unwrap_or_else(|| DEFAULT_SOURCE_FILE.to_string())
                     });

    debug!("Set source to {source}");

    let tmp_dir = ARGS.tmp_dir()
                      .clone()
                      .unwrap_or_else(|| {
                          config.tmp_dir
                                .unwrap_or_else(|| DEFAULT_TMP_DIR.to_string())
                      });

    debug!("Set tmp_dir to {tmp_dir}");

    let preserve_tmp_dir = ARGS.preserve_tmp_dir();

    debug!("Set preserve_tmp_dir to {preserve_tmp_dir}");

    let target_dir = ARGS.target_dir()
                         .clone()
                         .unwrap_or_else(|| {
                             config.target_dir
                                   .unwrap_or_else(|| DEFAULT_TARGET_DIR.to_string())
                         });

    debug!("Set target dir to {target_dir}");

    let ignore_jplag_out = ARGS.ignore_output();

    debug!("Ignore jplag output: {ignore_jplag_out}");

    let keep_non_ascii = ARGS.keep_non_ascii();

    debug!("Remove non ascii {keep_non_ascii}");

    let jplag_jar = ARGS.jplag_jar()
                        .clone()
                        .unwrap_or_else(|| {
                            config.jplag_jar
                                  .unwrap_or_else(|| DEFAULT_JPLAG_FILE.to_string())
                        });

    debug!("Set jplag_jar to {jplag_jar}");

    let mut jplag_args = ARGS.jplag_args()
                             .clone();
    let mut jplag_args_overridden = true;
    if jplag_args.is_empty() {
        let mut to_append = config.jplag_args
                                  .unwrap_or_else(|| {
                                      jplag_args_overridden = false;
                                      let v;
                                      #[cfg(not(feature = "legacy"))]
                                      {
                                          v = vec![
                                              tmp_dir.clone(),
                                              String::from("-r"),
                                              format!("{target_dir}/{DEFAULT_RES_ZIP}"),
                                              String::from("-l"),
                                              String::from(DEFAULT_JAVA_VERSION),
                                          ]
                                      }
                                      #[cfg(feature = "legacy")]
                                      {
                                          v = vec![
                                              String::from("-s"),
                                              tmp_dir.clone(),
                                              String::from("-r"),
                                              target_dir.clone(),
                                              String::from("-l"),
                                              String::from(DEFAULT_JAVA_VERSION),
                                          ]
                                      }
                                      v
                                  });
        jplag_args.append(&mut to_append);
    }

    if !jplag_args_overridden {
        debug!("Jplag args were not overridden, checking for ignore file");
        let ignore_file = ARGS.ignore_file()
                              .clone()
                              .or(config.ignore_file);

        if let Some(ignore_file) = ignore_file {
            debug!("Ignore file is set: {ignore_file}");

            if !fs::exists(&ignore_file)
                .with_context(|| format!("Unable to check if \"{ignore_file}\" exists"))? {
                bail!("Ignore file \"{ignore_file}\" not found");
            }

            jplag_args.push(String::from("-x"));
            jplag_args.push(ignore_file);
        } else {
            debug!("Ignore file not set");
        }
    } else {
        debug!("Jplag args were overridden, ignoring possible ignore file");
    }

    debug!("Set jplag args to {jplag_args:?}");

    let additional_submission_dirs = ARGS.add_sub_dirs().clone();

    debug!("Additional submission dirs: {additional_submission_dirs:?}");

    info!("Successfully parsed config");

    Ok((
        source,
        tmp_dir,
        preserve_tmp_dir,
        target_dir,
        keep_non_ascii,
        jplag_jar,
        jplag_args,
        ignore_jplag_out,
        additional_submission_dirs,
    ))
}

fn parse_toml(file: &str, conf_file_name_overridden: bool) -> Result<Config> {
    debug!("Parsing toml, source: {file}");
    if !fs::exists(&file)
        .with_context(|| format!("Unable to check if {file} exists"))? {
        debug!("{file} does not exist");
        if conf_file_name_overridden {
            bail!("Config file \"{file}\" not found");
        }
        debug!("Returning empty config");
        return Ok(Config {
            source_zip: None,
            target_dir: None,
            tmp_dir: None,
            ignore_file: None,
            jplag_jar: None,
            jplag_args: None,
        });
    }
    let toml = fs::read_to_string(&file)
        .with_context(|| format!("Failed to read from config file {file}"))?;
    debug!("Parsing toml, raw: {toml}");
    Ok(
        toml::from_str::<Config>(&toml)
            .with_context(|| format!(
                "Unable to parse to Config, raw string:\
            \n\"\"\"\n\
            {toml}\
            \"\"\"\
            "))?
    )
}

fn dump_default_config() -> Result<()> {
    if fs::exists(DEFAULT_CONFIG_FILE)
        .with_context(|| format!("Unable to check if \"{DEFAULT_CONFIG_FILE}\" exists"))? {
        warn!("\"{DEFAULT_CONFIG_FILE}\" already exists, do you want to override it? [Y/n]");
        let mut input = String::new();
        io::stdin().read_line(&mut input).with_context(|| "Unable to read stdin")?;
        if input.to_lowercase().trim() != "y" {
            info!("Aborting");
            return Ok(());
        }
    }

    let conf = Config {
        source_zip: Some(String::from(DEFAULT_SOURCE_FILE)),
        target_dir: Some(String::from(DEFAULT_TARGET_DIR)),
        tmp_dir: Some(String::from(DEFAULT_TMP_DIR)),
        ignore_file: None, // Don't like it, but if we set something the next run might fail
        jplag_jar: Some(String::from(DEFAULT_JPLAG_FILE)),
        #[cfg(not(feature = "legacy"))]
        jplag_args: Some(vec![
            String::from(DEFAULT_TMP_DIR),
            String::from("-r"),
            format!("{DEFAULT_TARGET_DIR}/{DEFAULT_RES_ZIP}"),
            String::from("-l"),
            String::from(DEFAULT_JAVA_VERSION),
        ]),
        #[cfg(feature = "legacy")]
        jplag_args: Some(vec![
            String::from("-s"),
            String::from(DEFAULT_TMP_DIR),
            String::from("-r"),
            String::from(DEFAULT_TARGET_DIR),
            String::from("-l"),
            String::from(DEFAULT_JAVA_VERSION),
        ]),
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

    let conf_str = toml::to_string_pretty(&conf)
        .with_context(|| format!("Unable to parse default config (how???) {conf:?}"))?;

    debug!("Writing default config:\
    \"\"\"\n\
    {conf_str}\
    \"\"\"\
    ");

    writeln!(writer, "{conf_str}")
        .with_context(|| format!("Unable to write default config to {DEFAULT_CONFIG_FILE}"))?;
    writer.flush()
          .with_context(|| format!("Unable to flush config file {DEFAULT_CONFIG_FILE}"))?;

    info!("Created default config");

    Ok(())
}

