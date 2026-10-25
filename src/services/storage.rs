use crate::state::Workspace;
use atomic_write_file::AtomicWriteFile;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct LoadedWorkspace {
    pub workspace: Workspace,
    pub warning: Option<String>,
}

pub fn load(path: &Path) -> io::Result<LoadedWorkspace> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(LoadedWorkspace {
                workspace: Workspace::default(),
                warning: None,
            });
        }
        Err(error) => return Err(error),
    };
    match serde_json::from_slice::<Workspace>(&bytes) {
        Ok(workspace) if workspace.version == 1 => Ok(LoadedWorkspace {
            workspace,
            warning: None,
        }),
        Ok(_) => Err(io::Error::other(
            "Unsupported workspace schema. Use a compatible app to avoid data loss.",
        )),
        Err(_) => {
            let backup = quarantine_path(path);
            // Do not overwrite a previous recovery file.
            fs::hard_link(path, &backup)?;
            fs::remove_file(path)?;
            Ok(LoadedWorkspace {
                workspace: Workspace::default(),
                warning: Some(format!(
                    "Unreadable workspace preserved at {}. Starting an empty board.",
                    backup.display()
                )),
            })
        }
    }
}

fn quarantine_path(path: &Path) -> PathBuf {
    path.with_extension("json.bad")
}

pub fn save(path: &Path, workspace: &Workspace) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("Workspace path has no parent directory"))?;
    fs::create_dir_all(parent)?;
    let mut file = AtomicWriteFile::open(path)?;
    serde_json::to_writer_pretty(&mut file, workspace)?;
    file.write_all(b"\n")?;
    file.commit()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "notion-gpui-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }

        fn path(&self) -> PathBuf {
            self.0.join("workspace.json")
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn missing_file_and_atomic_round_trip() {
        let directory = TestDirectory::new();
        let path = directory.path();
        let mut workspace = load(&path).unwrap().workspace;
        workspace.board.add("Ship desktop app", "Team").unwrap();
        workspace.route = crate::state::Route::Chat;
        save(&path, &workspace).unwrap();
        assert_eq!(load(&path).unwrap().workspace, workspace);
        workspace.board.advance(1);
        save(&path, &workspace).unwrap();
        assert_eq!(load(&path).unwrap().workspace, workspace);
    }

    #[test]
    fn corrupt_state_is_preserved_not_overwritten() {
        let directory = TestDirectory::new();
        let path = directory.path();
        fs::write(&path, b"not json").unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.warning.is_some());
        assert!(loaded.workspace.board.cards.is_empty());
        assert_eq!(fs::read(quarantine_path(&path)).unwrap(), b"not json");
        fs::write(&path, b"another corrupt file").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"another corrupt file");
        assert_eq!(fs::read(quarantine_path(&path)).unwrap(), b"not json");
    }

    #[test]
    fn future_schema_is_not_silently_reset() {
        let directory = TestDirectory::new();
        let path = directory.path();
        fs::write(&path, br#"{"version":99}"#).unwrap();
        assert!(load(&path).is_err());
        assert!(path.exists());
    }

    #[test]
    fn unreadable_paths_and_failed_saves_return_errors() {
        let directory = TestDirectory::new();
        assert!(load(&directory.0).is_err());
        let not_a_directory = directory.0.join("file");
        fs::write(&not_a_directory, b"keep this").unwrap();
        let invalid_path = not_a_directory.join("workspace.json");
        assert!(save(&invalid_path, &Workspace::default()).is_err());
        assert_eq!(fs::read(&not_a_directory).unwrap(), b"keep this");
    }
}
