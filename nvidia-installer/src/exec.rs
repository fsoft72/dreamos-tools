use crate::InstallError;
use std::io::Write;
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
pub struct CmdOutput {
    pub stdout: String,
    pub stderr: String,
}

pub fn run_cmd(argv: &[&str]) -> Result<CmdOutput, InstallError> {
    let cmd_str = argv.join(" ");
    let output = Command::new(argv[0])
        .args(&argv[1..])
        .output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }
    Ok(CmdOutput { stdout, stderr })
}

/// Like `run_cmd`, but writes `stdin_data` to the child's stdin before
/// waiting on it - needed for `mokutil --import`, which reads a one-time
/// enrollment password twice from stdin rather than accepting it as an
/// argument (a password would otherwise leak into `/proc/<pid>/cmdline`
/// and shell history).
pub fn run_cmd_with_stdin(argv: &[&str], stdin_data: &str) -> Result<CmdOutput, InstallError> {
    let cmd_str = argv.join(" ");
    let mut child = Command::new(argv[0])
        .args(&argv[1..])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    child
        .stdin
        .take()
        .expect("stdin was piped")
        .write_all(stdin_data.as_bytes())
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let output = child
        .wait_with_output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr,
        });
    }
    Ok(CmdOutput { stdout, stderr })
}

pub fn apt_update_argv() -> Vec<String> {
    vec!["apt-get".into(), "update".into()]
}

pub fn apt_install_argv(package: &str) -> Vec<String> {
    vec!["apt-get".into(), "install".into(), "-y".into(), package.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_cmd_captures_stdout() {
        let out = run_cmd(&["echo", "hello"]).unwrap();
        assert_eq!(out.stdout.trim(), "hello");
    }

    #[test]
    fn run_cmd_reports_nonzero_exit() {
        let err = run_cmd(&["false"]).unwrap_err();
        assert!(matches!(err, InstallError::CommandFailed { .. }));
    }

    #[test]
    fn run_cmd_with_stdin_pipes_input() {
        let out = run_cmd_with_stdin(&["cat"], "piped data").unwrap();
        assert_eq!(out.stdout, "piped data");
    }

    #[test]
    fn apt_install_argv_includes_package_and_yes_flag() {
        let argv = apt_install_argv("nvidia-driver");
        assert_eq!(argv, vec!["apt-get", "install", "-y", "nvidia-driver"]);
    }
}
