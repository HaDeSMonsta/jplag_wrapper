use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[cfg(debug_assertions)]
const DEFAULT_LOG_LEVEL_STR: &str = "debug";
#[cfg(not(debug_assertions))]
const DEFAULT_LOG_LEVEL_STR: &str = "info";

/// A jplag wrapper with sane defaults
///
/// Option priority is as follows (`-> == override`)
///
/// `cli-arg -> toml config -> default value`
///
/// While `--init` creates a toml file with all settings,
/// you only need to set the ones you want to change
#[derive(Clone, Debug, Parser)]
#[clap(version)]
// Complains that `jplag_args` ends in `args`
#[allow(clippy::struct_field_names, clippy::struct_excessive_bools)]
pub struct Args {
    /// Docs
    ///
    /// More
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Initialize the config,
    /// will create (or override!) `config.toml` with all values
    /// and fill it with the defaults
    ///
    /// Except `ignore_file`, because the default is `None`
    #[clap(long)]
    init: bool,
    /// Log Level to use
    ///
    /// Possible values are: trace (5), debug (4), info (3), warn (2), error (1).
    #[clap(short, long, default_value_t = String::from(DEFAULT_LOG_LEVEL_STR))]
    log_level: String,
    /// Set to abort on any extraction related error
    ///
    /// Default is to continue and display errors after viewing jplag output
    #[clap(long)]
    abort_on_err: bool,
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
    /// Warning: This directory will be deleted at application start if it exists
    #[clap(short, long)]
    target_dir: Option<String>,
    /// Where to put the temporary files
    ///
    /// Defaults to `tmp/`
    ///
    /// Warning: This directory will be deleted at application start if it exists
    #[clap(long)]
    tmp_dir: Option<String>,
    /// Set to not remove `{{tmp_dir}}`
    /// when the program finishes
    #[clap(short, long)]
    preserve_tmp_dir: bool,
    /// Where to find the ignore-file
    ///
    /// Will be passed to jplag as an arg
    /// `-x {{ignore_file}}`
    ///
    /// Defaults to None
    ///
    /// Will panic if arg is set and the file doesn't exist
    ///
    /// Argument will be ignored if jplag args are manually set
    #[clap(short, long)]
    ignore_file: Option<String>,
    /// Set to ignore the output of jplag
    ///
    /// The program will still wait for the child process to exit
    /// and process the output, but it will just ignore it
    #[clap(long)]
    ignore_output: bool,
    /// Where the jplag jar can be found
    ///
    /// Defaults to `jplag.jar`
    ///
    /// Will panic if the file does not exist
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
    /// In practice, we will just copy all directories into `{{tmp_dir}}`
    /// after extracting the `{{source_zip}}` file
    ///
    /// Expected structure: `foo/bar[.zip|.tar|.tar.gz|.rawr]`
    ///
    /// Expected input: `foo/`
    add_sub_dirs: Vec<String>,
    /// Will be passed directly to jplag as arguments
    ///
    /// Defaults to `{{tmp_dir}} -r {{target_dir}}/results.zip -l java`
    #[clap(last = true)]
    jplag_args: Vec<String>,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Cmd {
    Complete {
        /// The shell to generate completions for
        shell: Shell,
    },
}

#[allow(dead_code)]
impl Args {
    pub const fn init(&self) -> bool {
        self.init
    }

    pub fn log_level(&self) -> &str {
        &self.log_level
    }

    pub const fn abort_on_err(&self) -> bool {
        self.abort_on_err
    }

    pub const fn config(&self) -> Option<&String> {
        if let Some(ref conf) = self.config {
            Some(conf)
        } else {
            None
        }
    }

    pub const fn source_zip(&self) -> Option<&String> {
        if let Some(ref zip) = self.source_zip {
            Some(zip)
        } else {
            None
        }
    }

    pub const fn target_dir(&self) -> Option<&String> {
        if let Some(ref target) = self.target_dir {
            Some(target)
        } else {
            None
        }
    }

    pub const fn tmp_dir(&self) -> Option<&String> {
        if let Some(ref tmp) = self.tmp_dir {
            Some(tmp)
        } else {
            None
        }
    }

    pub const fn preserve_tmp_dir(&self) -> bool {
        self.preserve_tmp_dir
    }

    pub const fn ignore_file(&self) -> Option<&String> {
        if let Some(ref ignored) = self.ignore_file {
            Some(ignored)
        } else {
            None
        }
    }

    pub const fn ignore_output(&self) -> bool {
        self.ignore_output
    }

    pub const fn jplag_jar(&self) -> Option<&String> {
        if let Some(ref jar) = self.jplag_jar {
            Some(jar)
        } else {
            None
        }
    }

    pub fn add_sub_dirs(&self) -> &[String] {
        &self.add_sub_dirs
    }

    pub fn jplag_args(&self) -> &[String] {
        &self.jplag_args
    }

    pub const fn cmd(&self) -> Option<&Cmd> {
        if let Some(ref cmd) = self.cmd {
            Some(cmd)
        } else {
            None
        }
    }
}
