use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileNotFoundError {
    #[error("Config file \"{0}\" not found")]
    ConfigFileNotFound(String),
    #[error("Jplag jar file \"{0}\" not found")]
    JarFileNotFound(String),
    #[error("Submission zip file \"{0}\" not found")]
    ZipFileNotFound(String),
    #[error("Ignore file \"{0}\" not found")]
    IgnoreFileNotFound(String),
}

#[derive(Debug, Error)]
pub enum SubCmdError {
    #[error("Jplag failed with exit code {0}")]
    JplagExecFailure(i32),
}

