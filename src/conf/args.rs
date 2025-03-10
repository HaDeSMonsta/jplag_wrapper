use std::process::exit;
use clap::Parser;

const BINARY_NAME: &str = env!("CARGO_PKG_NAME");
const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");


/// A jplag wrapper with sane defaults
///
/// Option priority is as follows (`-> == override`)
///
/// `cli-arg -> toml config -> default value`
///
/// While `--init` creates a toml file with all settings,
/// you only need to set the ones you want to change
#[derive(Clone, Debug, Parser)]
#[clap()]
pub struct Args {
    /// Print version
    #[clap(short, long)]
    version: bool,
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
    /// Keep all non ASCII characters from all submissions
    /// 
    /// jplag can't handle non ASCII characters properly, so we remove them by default.
    /// Set this flag to keep them
    #[clap(long)]
    keep_non_ascii: bool,
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

/// Print "{BINARY_NAME} {BINARY_VERSION}" and exit
pub fn version() {
    println!("{BINARY_NAME} {BINARY_VERSION}");
    exit(0);
}

#[allow(dead_code)]
impl Args {
    pub fn version(&self) -> bool {
        self.version
    }

    pub fn init(&self) -> bool {
        self.init
    }

    pub fn debug(&self) -> bool {
        self.debug
    }

    pub fn keep_non_ascii(&self) -> bool {
        self.keep_non_ascii
    }

    pub fn config(&self) -> &Option<String> {
        &self.config
    }

    pub fn source_zip(&self) -> &Option<String> {
        &self.source_zip
    }

    pub fn target_dir(&self) -> &Option<String> {
        &self.target_dir
    }

    pub fn tmp_dir(&self) -> &Option<String> {
        &self.tmp_dir
    }

    pub fn preserve_tmp_dir(&self) -> bool {
        self.preserve_tmp_dir
    }

    pub fn ignore_file(&self) -> &Option<String> {
        &self.ignore_file
    }

    pub fn ignore_output(&self) -> bool {
        self.ignore_output
    }

    pub fn jplag_jar(&self) -> &Option<String> {
        &self.jplag_jar
    }

    pub fn add_sub_dirs(&self) -> &Vec<String> {
        &self.add_sub_dirs
    }

    pub fn jplag_args(&self) -> &Vec<String> {
        &self.jplag_args
    }
}
