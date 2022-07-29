
use std::{fmt::Display, error::Error, path::PathBuf};

#[derive(Debug)]
pub(crate) struct DownloadError {}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "clangd download failed")
    }
}

impl Error for DownloadError {}

#[derive(Debug)]
pub(crate) struct UnsupportOsError {
    pub(crate) os: String,
}

impl Display for UnsupportOsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Your os \"{}\" is unsupported", self.os)
    }
}

impl Error for UnsupportOsError {}

#[derive(Debug)]
pub(crate) struct FileNotFound {
    pub(crate) path: PathBuf,
}

impl Display for FileNotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "File \"{}\" not found.", self.path.display())
    }
}

impl Error for FileNotFound {}

