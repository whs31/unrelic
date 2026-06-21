use std::{io, path::PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, UnrelicError>;

#[derive(Debug, Error)]
pub enum UnrelicError {
    #[error("input does not exist: {path}")]
    InputMissing { path: PathBuf },

    #[error("input is neither a file nor a directory: {path}")]
    InputUnsupported { path: PathBuf },

    #[error("directory output path exists but is not a directory: {path}")]
    OutputMustBeDirectory { path: PathBuf },

    #[error("no .mpg or .mpeg files found in directory: {path}")]
    NoInputFiles { path: PathBuf },

    #[error(
        "missing required executable `{name}`; pass --{name}, place it next to unrelic, or add it to PATH"
    )]
    MissingTool { name: &'static str },

    #[error("configured {name} path does not exist: {path}")]
    ToolPathMissing { name: &'static str, path: PathBuf },

    #[error("configured {name} path is not a file: {path}")]
    ToolPathNotFile { name: &'static str, path: PathBuf },

    #[error("output already exists, skipping: {path}")]
    OutputExists { path: PathBuf },

    #[error("failed to walk directory `{path}`: {source}")]
    WalkDir {
        path: PathBuf,
        #[source]
        source: walkdir::Error,
    },

    #[error("failed to create directory `{path}`: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to remove file `{path}`: {source}")]
    RemoveFile {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to rename `{from}` to `{to}`: {source}")]
    Rename {
        from: PathBuf,
        to: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to spawn `{program}`: {source}")]
    Spawn {
        program: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed while waiting for `{program}`: {source}")]
    Wait {
        program: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to read {stream} from `{program}`: {source}")]
    ReadPipe {
        program: PathBuf,
        stream: &'static str,
        #[source]
        source: io::Error,
    },

    #[error("failed to capture {stream} from `{program}`")]
    PipeUnavailable {
        program: PathBuf,
        stream: &'static str,
    },

    #[error("ffprobe failed for `{input}`: {stderr}")]
    Probe { input: PathBuf, stderr: String },

    #[error("ffmpeg failed for `{input}`: {stderr}")]
    Convert { input: PathBuf, stderr: String },
}
