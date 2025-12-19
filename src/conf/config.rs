use crate::conf::args::{Args, Cmd};
use clap::{CommandFactory, Parser};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::process::exit;
use std::sync::LazyLock;
use std::{fs, io};
use tracing::{debug, info, instrument, warn};

const DEFAULT_CONFIG_FILE: &str = "config.toml";
const DEFAULT_SOURCE_FILE: &str = "submissions.zip";
const DEFAULT_JPLAG_FILE: &str = "jplag.jar";
const DEFAULT_TARGET_DIR: &str = "out/";
const DEFAULT_TMP_DIR: &str = "tmp/";
const DEFAULT_RES_ZIP: &str = "results";
const DEFAULT_JAVA_VERSION: &str = "java";

pub static ARGS: LazyLock<Args> = LazyLock::new(Args::parse);
static CONFIG: LazyLock<Config> = LazyLock::new(|| match parse_toml() {
    Ok(c) => c,
    Err(e) => panic!("unable to parse config: {e:?}"),
});

#[derive(Debug)]
pub struct ParsedArgs {
    pub source_file: String,
    pub tmp_dir: String,
    #[cfg(not(debug_assertions))]
    pub preserve_tmp_dir: bool,
    pub target_dir: String,
    pub abort_on_error: bool,
    pub jplag_jar: String,
    pub jplag_args: Vec<String>,
    pub additional_submission_dirs: Vec<String>,
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

// TODO Scratch this whole parsing and cloning and use take
// Until then:
#[allow(clippy::too_many_lines)]
/// Parse args for the bin, prioritizes cli over toml
///
/// Returns: `(source, tmp_dir, preserve_tmp_dir, target_dir, keep_non_ascii,
/// jplag_jar, jplag_args, additional_submission_dirs)`
#[instrument]
pub fn parse_args() -> Result<ParsedArgs> {
    debug!("checking completions");
    if let Some(Cmd::Complete { shell }) = ARGS.cmd() {
        let mut cmd = Args::command();
        clap_complete::generate(*shell, &mut cmd, crate::PROGRAM_NAME, &mut io::stdout());
        exit(0);
    }
    debug!("getting args");
    if ARGS.init() {
        debug!("initializing config");
        dump_default_config().with_context(|| "unable to write default config")?;
        exit(0);
    }

    debug!("successfully parsed toml");

    let source = ARGS.source_zip().map_or_else(
        || {
            CONFIG
                .source_zip
                .clone()
                .unwrap_or_else(|| DEFAULT_SOURCE_FILE.to_string())
        },
        ToOwned::to_owned,
    );

    debug!("set source to {source}");

    let tmp_dir = ARGS.tmp_dir().map_or_else(
        || {
            CONFIG
                .tmp_dir
                .clone()
                .unwrap_or_else(|| DEFAULT_TMP_DIR.to_string())
        },
        ToOwned::to_owned,
    );

    debug!("set tmp_dir to {tmp_dir}");

    #[cfg(not(debug_assertions))]
    let preserve_tmp_dir = ARGS.preserve_tmp_dir();

    #[cfg(not(debug_assertions))]
    debug!("set preserve_tmp_dir to {preserve_tmp_dir}");

    let target_dir = ARGS.target_dir().map_or_else(
        || {
            CONFIG
                .target_dir
                .clone()
                .unwrap_or_else(|| DEFAULT_TARGET_DIR.to_string())
        },
        ToOwned::to_owned,
    );

    debug!("set target dir to {target_dir}");

    let jplag_jar = ARGS.jplag_jar().map_or_else(
        || {
            CONFIG
                .jplag_jar
                .clone()
                .unwrap_or_else(|| DEFAULT_JPLAG_FILE.to_string())
        },
        ToOwned::to_owned,
    );

    debug!("set jplag_jar to {jplag_jar}");

    let mut jplag_args = ARGS.jplag_args().to_vec();
    let jplag_args_overridden = !jplag_args.is_empty();

    if jplag_args_overridden {
        debug!("jplag args were overridden, ignoring possible ignore file");
    } else {
        let mut to_append = CONFIG.jplag_args.clone().unwrap_or_else(|| {
            // If you change this, change the default args in in `dump_default_config()` too
            vec![
                tmp_dir.clone(),
                String::from("-r"),
                format!("{target_dir}/{DEFAULT_RES_ZIP}"),
                String::from("-l"),
                String::from(DEFAULT_JAVA_VERSION),
                String::from("--encoding"),
                String::from("utf-8"),
                String::from("--skip-version-check"),
            ]
        });
        jplag_args.append(&mut to_append);

        debug!("jplag args were not overridden, checking for ignore file");
        let ignore_file = ARGS
            .ignore_file()
            .map(ToOwned::to_owned)
            .or_else(|| CONFIG.ignore_file.clone());

        if let Some(ignore_file) = ignore_file {
            debug!("ignore file is set: {ignore_file}");

            if !fs::exists(&ignore_file)
                .with_context(|| format!("unable to check if \"{ignore_file}\" exists"))?
            {
                bail!("ignore file \"{ignore_file}\" not found");
            }

            jplag_args.push(String::from("-x"));
            jplag_args.push(ignore_file);
        } else {
            debug!("ignore file not set");
        }
    }

    debug!("set jplag args to {jplag_args:?}");

    let additional_submission_dirs = ARGS.add_sub_dirs().to_vec();

    debug!("additional submission dirs: {additional_submission_dirs:?}");

    info!("successfully parsed config");

    let parsed_args = ParsedArgs {
        source_file: source,
        tmp_dir,
        #[cfg(not(debug_assertions))]
        preserve_tmp_dir,
        target_dir,
        abort_on_error: ARGS.abort_on_err(),
        jplag_jar,
        jplag_args,
        additional_submission_dirs,
    };

    Ok(parsed_args)
}

#[instrument]
fn parse_toml() -> Result<Config> {
    let conf_file = ARGS
        .config()
        .map_or_else(|| DEFAULT_CONFIG_FILE.to_string(), ToOwned::to_owned);

    debug!("parsing toml, source: {conf_file}");
    if !fs::exists(&conf_file).with_context(|| format!("unable to check if {conf_file} exists"))? {
        debug!("{conf_file} does not exist");
        if ARGS.config().is_some() {
            bail!("overridden config file \"{conf_file}\" not found");
        }

        debug!("returning empty config");
        return Ok(Config {
            source_zip: None,
            target_dir: None,
            tmp_dir: None,
            ignore_file: None,
            jplag_jar: None,
            jplag_args: None,
        });
    }

    let toml = fs::read_to_string(&conf_file)
        .with_context(|| format!("failed to read from config file {conf_file}"))?;

    debug!("parsing toml, raw: {toml}");
    toml::from_str::<Config>(&toml).with_context(|| {
        format!(
            "unable to parse to Config, raw string:\
            \n\"\"\"\n\
            {toml}\
            \"\"\""
        )
    })
}

#[instrument]
fn dump_default_config() -> Result<()> {
    if fs::exists(DEFAULT_CONFIG_FILE)
        .with_context(|| format!("unable to check if \"{DEFAULT_CONFIG_FILE}\" exists"))?
    {
        warn!("\"{DEFAULT_CONFIG_FILE}\" already exists, do you want to override it? [Y/n]");
        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .with_context(|| "unable to read stdin")?;
        if input.to_lowercase().trim() != "y" {
            info!("aborting");
            return Ok(());
        }
    }

    let conf = Config {
        source_zip: Some(String::from(DEFAULT_SOURCE_FILE)),
        target_dir: Some(String::from(DEFAULT_TARGET_DIR)),
        tmp_dir: Some(String::from(DEFAULT_TMP_DIR)),
        ignore_file: None, // Don't like it, but if we set something, the next run might fail
        jplag_jar: Some(String::from(DEFAULT_JPLAG_FILE)),
        // If you change this, change the default args in in `parse_args()` too
        jplag_args: Some(vec![
            String::from(DEFAULT_TMP_DIR),
            String::from("-r"),
            format!("{DEFAULT_TARGET_DIR}/{DEFAULT_RES_ZIP}"),
            String::from("-l"),
            String::from(DEFAULT_JAVA_VERSION),
            String::from("--encoding"),
            String::from("utf-8"),
            String::from("--skip-version-check"),
        ]),
    };
    debug!("created default config struct");
    let file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(DEFAULT_CONFIG_FILE)
        .with_context(|| {
            format!("failed to open/create/truncate config file: {DEFAULT_CONFIG_FILE}")
        })?;
    debug!("opened default config file");

    let mut writer = BufWriter::new(file);

    let conf_str = toml::to_string_pretty(&conf)
        .with_context(|| format!("unable to parse default config (how???) {conf:?}"))?;

    debug!(
        "writing default config:\
        \"\"\"\n\
        {conf_str}\
        \"\"\""
    );

    writeln!(writer, "{conf_str}")
        .with_context(|| format!("unable to write default config to {DEFAULT_CONFIG_FILE}"))?;
    writer
        .flush()
        .with_context(|| format!("unable to flush config file {DEFAULT_CONFIG_FILE}"))?;

    info!("created default config");

    Ok(())
}
