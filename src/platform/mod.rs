use std::{io, path::PathBuf};

pub fn state_path() -> io::Result<PathBuf> {
    directories::ProjectDirs::from("dev", "notion-gpui", "notion-gpui")
        .map(|dirs| dirs.data_local_dir().join("workspace.json"))
        .ok_or_else(|| {
            io::Error::other("The operating system did not provide a user data directory")
        })
}
