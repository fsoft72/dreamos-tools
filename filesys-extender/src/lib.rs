pub mod disk;
pub mod exec;

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DiskOpError {
    #[error("command '{cmd}' failed with exit code {code:?}: {stderr}")]
    CommandFailed {
        cmd: String,
        code: Option<i32>,
        stderr: String,
    },
    #[error("failed to spawn '{cmd}': {detail}")]
    Spawn { cmd: String, detail: String },
    #[error("failed to parse {what}: {detail}")]
    Parse { what: String, detail: String },
}
