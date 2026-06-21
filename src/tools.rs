use std::path::{Path, PathBuf};

use crate::{Result, UnrelicError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ToolPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

pub fn resolve_tools(ffmpeg: Option<&Path>, ffprobe: Option<&Path>) -> Result<ToolPaths> {
    Ok(ToolPaths {
        ffmpeg: resolve_tool("ffmpeg", ffmpeg)?,
        ffprobe: resolve_tool("ffprobe", ffprobe)?,
    })
}

pub(crate) fn resolve_tool(name: &'static str, explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if !path.exists() {
            return Err(UnrelicError::ToolPathMissing {
                name,
                path: path.to_path_buf(),
            });
        }
        if !path.is_file() {
            return Err(UnrelicError::ToolPathNotFile {
                name,
                path: path.to_path_buf(),
            });
        }
        return Ok(path.to_path_buf());
    }

    if let Some(path) = bundled_tool_path(name) {
        return Ok(path);
    }

    which::which(name).map_err(|_| UnrelicError::MissingTool { name })
}

fn bundled_tool_path(name: &str) -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let candidate = directory.join(tool_file_name(name));
    candidate.is_file().then_some(candidate)
}

fn tool_file_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_missing_tool_is_typed_error() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing-ffmpeg");

        let error = resolve_tool("ffmpeg", Some(&missing)).unwrap_err();

        assert!(matches!(error, UnrelicError::ToolPathMissing { .. }));
    }

    #[test]
    fn explicit_directory_tool_is_typed_error() {
        let temp = tempfile::tempdir().unwrap();

        let error = resolve_tool("ffmpeg", Some(temp.path())).unwrap_err();

        assert!(matches!(error, UnrelicError::ToolPathNotFile { .. }));
    }
}
