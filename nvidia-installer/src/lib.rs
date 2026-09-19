pub mod detect;
// pub mod exec;       // added in Task 4
pub mod recommend;
pub mod secureboot;
// pub mod state;      // added in Task 5

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum InstallError {
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
