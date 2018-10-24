use super::Result;
use super::error;

use std::io::Read;
use std::fs::File;
use std::path::{Path, PathBuf};

pub struct Loader {
    directory: Vec<PathBuf>,
}

impl Loader {
    pub fn new() -> Self {
        Self {
            directory: Vec::new(),
        }
    }

    pub fn load(&self, path: &str) -> Result<String> {
        debug!("Loading file: {}", path);

        let path = Path::new(&self.current_dir()).join(path);

        debug!("Loading path: {}", path.to_string_lossy());

        let mut file = File::open(path).map_err(|e| error::file_error(e))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| error::file_error(e))?;

        Ok(contents)
    }

    pub fn enter_dir(&mut self, dir: &str) -> Result<()> {
        let path = Path::new(&self.current_dir()).join(dir);

        let path = path.canonicalize().map_err(|e| error::file_error(e))?;

        self.directory.push(path);

        debug!("Entered to directory: {}", self.current_dir());

        Ok(())
    }

    pub fn exit_dir(&mut self) {
        self.directory.pop();

        debug!("Exited to directory: {}", self.current_dir());
    }

    pub fn current_dir(&self) -> String {
        self.directory
            .last()
            .map(|p| {
                let mut p = p.clone();
                p.pop();
                p.to_string_lossy().to_string()
            })
            .unwrap_or("".into())
    }
}
