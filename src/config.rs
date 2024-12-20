use std::fmt::Debug;
use std::{fs, io};
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::process::exit;
use anyhow::{Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
#[cfg(not(debug_assertions))]
use tracing::Level;

use crate::custom_errors;

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


/// A jplag wrapper with sane defaults
///
/// Option priority is as follows (`-> == override`)
///
/// `cli-arg -> toml config -> default value`
///
/// While `--init` creates a toml file with all settings,
/// you only need to se the ones you want to change
#[derive(Clone, Debug, Parser)]
#[clap(
    version,
    about = "A jplag wrapper with sane defaults",
    long_about = "A jplag wrapper with sane defaults\n\n\
    Option priority for each individual option is as follows ('-> == override')\n\n\
    `cli-arg -> toml config -> default value`\n\n\
    While `--init` creates a toml file with all settings, \
    you only need to set the ones you want to change"
)]
struct Args {
    /// Initialize the config,
    /// will create (or override!) `config.toml` with all values
    /// and fill it with the defaults
    ///
    /// Except `ignore_file`, because the default is `None`
    #[clap(long)]
    init: bool,
    /// Set to use log level `debug`
    ///
    /// Otherwise, `info` will be used
    #[clap(short, long)]
    debug: bool,
    /// Remove all non ASCII characters from all submissions
    #[clap(long)]
    remove_non_ascii: bool,
    /// Specify the config toml file to look for
    /// if you don't want to use the default config.toml
    ///
    /// Will panic, if file does not exist
    #[clap(short, long)]
    config: Option<String>,
    /// Where the input file can be found
    ///
    /// Defaults to `submissions.zip`
    #[clap(short, long)]
    source_zip: Option<String>,
    /// Where to put the results
    ///
    /// Defaults to `out/`
    ///
    /// Warning: This directory will be deleted at application start, if it exists
    #[clap(short, long)]
    target_dir: Option<String>,
    /// Where to put the temporary files
    ///
    /// Defaults to `tmp/`
    ///
    /// Warning: This directory will be deleted at application start, if it exists
    #[clap(long)]
    tmp_dir: Option<String>,
    /// Set to not remove {{tmp_dir}}
    /// when the program finishes
    #[clap(short, long)]
    preserve_tmp_dir: bool,
    /// Where to find the ignore file
    ///
    /// Will be passed to jplag as an arg
    /// `-x {{ignore_file}}`
    ///
    /// Defaults to None
    ///
    /// Will panic, if arg is set and file doesn't exist
    ///
    /// Argument will be ignored if jplag args are manually set
    #[clap(short, long)]
    ignore_file: Option<String>,
    /// Set to ignore the output of jplag
    ///
    /// The programm will still wait for the child process to exit
    /// and process the output, but it will just ignore it
    #[clap(long)]
    ignore_output: bool,
    /// Where the jplag jar can be found
    ///
    /// Defaults to `jplag.jar`
    ///
    /// Will panic, if file does not exist
    #[clap(short, long)]
    jplag_jar: Option<String>,
    /// Additional submission directories (if you read this with -h,
    /// use --help for full docs)
    ///
    /// A list of additional submissions
    /// which will be treated exactly like normal submissions
    ///
    /// This means no validation will be performed,
    /// except for checking that each input exists and is a directory
    ///
    /// In practise, we will just copy all directories into {{tmp_dir}}
    /// after extracting the {{source_zip}} file
    ///
    /// Expected structure: foo/bar[.zip|.tar|.tar.gz|.rawr]
    ///
    /// Expected input: foo/
    add_sub_dirs: Vec<String>,
    #[cfg(not(feature = "legacy"))]
    /// Will be passed directly to jplag as arguments
    ///
    /// Defaults to `{{tmp_dir}} -r {{target_dir}}/results.zip -l java`
    #[clap(last = true)]
    jplag_args: Vec<String>,
    #[cfg(feature = "legacy")]
    /// Everything after `--`
    ///
    /// Will be passed directly to jplag as arguments
    ///
    /// Defaults to `-s {{tmp_dir}} -r {{target_dir}} -l java19`
    #[clap(last = true)]
    jplag_args: Vec<String>,
}

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
    if Args::parse().debug {
        Level::DEBUG
    } else {
        Level::INFO
    }
}

/// Parse args for the bin, prioritizes cli over toml
///
/// Returns: (source, tmp_dir, preserve_tmp_dir, target_dir, remove_non_ascii,
/// jplag_jar, jplag_args, ignore_jplag_output, additional_submission_dirs)
pub fn parse_args() -> Result<(String, String, bool, String, bool, String, Vec<String>, bool, Vec<String>)> {
    debug!("Getting args");
    let args = Args::parse();
    if args.init {
        debug!("Initializing config");
        dump_default_config()
            .with_context(|| "Unable to write default config")?;
        exit(0);
    };

    let config_name_overridden = args.config.is_some();
    let config = parse_toml(
        &args.config.unwrap_or_else(|| DEFAULT_CONFIG_FILE.to_string()),
        config_name_overridden,
    ).with_context(|| "Unable to parse toml config")?;

    debug!("Successfully parsed toml");

    let source = args.source_zip
                     .unwrap_or_else(|| {
                         config.source_zip
                               .unwrap_or_else(|| DEFAULT_SOURCE_FILE.to_string())
                     });

    debug!("Set source to {source}");

    let tmp_dir = args.tmp_dir
                      .unwrap_or_else(|| {
                          config.tmp_dir
                                .unwrap_or_else(|| DEFAULT_TMP_DIR.to_string())
                      });

    debug!("Set tmp_dir to {tmp_dir}");

    let preserve_tmp_dir = args.preserve_tmp_dir;

    debug!("Set preserve_tmp_dir to {preserve_tmp_dir}");

    let target_dir = args.target_dir
                         .unwrap_or_else(|| {
                             config.target_dir
                                   .unwrap_or_else(|| DEFAULT_TARGET_DIR.to_string())
                         });

    debug!("Set target dir to {target_dir}");

    let ignore_jplag_out = args.ignore_output;

    debug!("Ignore jplag output: {ignore_jplag_out}");

    let remove_non_ascii = args.remove_non_ascii;

    debug!("Remove non ascii {remove_non_ascii}");

    let jplag_jar = args.jplag_jar
                        .unwrap_or_else(|| {
                            config.jplag_jar
                                  .unwrap_or_else(|| DEFAULT_JPLAG_FILE.to_string())
                        });

    debug!("Set jplag_jar to {jplag_jar}");

    let mut jplag_args = args.jplag_args;
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
        let ignore_file = args.ignore_file
                              .or(config.ignore_file);

        if let Some(ignore_file) = ignore_file {
            debug!("Ignore file is set: {ignore_file}");

            if !fs::exists(&ignore_file)
                .with_context(|| format!("Unable to check if \"{ignore_file}\" exists"))? {
                return Err(custom_errors::FileNotFoundError::IgnoreFileNotFound(ignore_file).into());
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

    let additional_submission_dirs = args.add_sub_dirs;

    debug!("Additional submission dirs: {additional_submission_dirs:?}");

    info!("Successfully parsed config");

    Ok((
        source,
        tmp_dir,
        preserve_tmp_dir,
        target_dir,
        remove_non_ascii,
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
            return Err(custom_errors::FileNotFoundError::ConfigFileNotFound(
                file.to_string()
            ).into());
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

