use std::path::Path;

use crate::error::{self, Result};

pub fn fullpath(path: &str) -> Result<String> {
    let cwd = std::env::current_dir().map_err(|e| error::file_error(e))?;
    Ok(format!("{}/{}", cwd.to_string_lossy(), path))
}

pub fn namespace<'a>(path: &'a str) -> Result<&'a str> {
    Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or(error::file_error("Couldn't determine namespace"))
}
