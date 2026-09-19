use crate::DiskOpError;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, DiskOpError> {
    let cmd_str = argv.join(" ");
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|source| DiskOpError::Spawn {
            cmd: cmd_str.clone(),
            detail: source.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(DiskOpError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }

    Ok(CmdOutput { stdout, stderr })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_captures_stdout_on_success() {
        let out = run_cmd(&["echo", "hello"]).unwrap();
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn run_cmd_returns_command_failed_on_nonzero_exit() {
        let err = run_cmd(&["false"]).unwrap_err();
        match err {
            DiskOpError::CommandFailed { cmd, code, .. } => {
                assert_eq!(cmd, "false");
                assert_eq!(code, Some(1));
            }
            other => panic!("expected CommandFailed, got {other:?}"),
        }
    }

    #[test]
    fn run_cmd_returns_spawn_error_for_missing_binary() {
        let err = run_cmd(&["definitely-not-a-real-binary-xyz"]).unwrap_err();
        assert!(matches!(err, DiskOpError::Spawn { .. }));
    }
}
