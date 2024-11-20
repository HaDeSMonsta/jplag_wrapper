use thiserror::Error;

#[derive(Debug, Error)]
pub enum FileNotFoundError {
    #[error("Config file \"{0}\" not found")]
    ConfigFileNotFound(String),
    #[error("Jplag jar file \"{0}\" not found")]
    JarFileNotFound(String),
    #[error("Submission zip file \"{0}\" not found")]
    ZipFileNotFound(String),
}