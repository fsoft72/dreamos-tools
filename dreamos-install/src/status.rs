use crate::catalog::CHECK_SCRIPT;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const CHECK_TIMEOUT: Duration = Duration::from_secs(10);
pub const APP_DIR_ENV: &str = "DREAMOS_APP_DIR";
/// Login name of the user who started dreamos-install, for scripts that
/// must install per-user files (the scripts themselves run as root).
pub const USER_ENV: &str = "DREAMOS_USER";
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Install state of an app as reported by its check script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Checking,
    Installed,
    NotInstalled,
    CheckFailed(String),
}

impl Status {
    /// Short human readable label for lists and the action bar.
    pub fn label(&self) -> &'static str {
        match self {
            Status::Checking => "Checking...",
            Status::Installed => "Installed",
            Status::NotInstalled => "Not installed",
            Status::CheckFailed(_) => "Check failed",
        }
    }
}

/// Picks the invoking user's uid: pkexec's, then sudo's, then `fallback`.
fn _pick_uid(pkexec: Option<&str>, sudo: Option<&str>, fallback: u32) -> u32 {
    [pkexec, sudo].into_iter().flatten().find_map(|v| v.parse().ok()).unwrap_or(fallback)
}

/// Login name of the user who started the tool, even after the pkexec
/// re-exec (pkexec exports `PKEXEC_UID`). None when it cannot be resolved.
pub fn invoking_user() -> Option<String> {
    // SAFETY: geteuid() is a pure syscall with no preconditions.
    let fallback = unsafe { libc::geteuid() };
    let uid = _pick_uid(
        std::env::var("PKEXEC_UID").ok().as_deref(),
        std::env::var("SUDO_UID").ok().as_deref(),
        fallback,
    );
    // SAFETY: getpwuid() returns NULL or a pointer to a static passwd
    // entry; pw_name is copied out immediately.
    unsafe {
        let entry = libc::getpwuid(uid);
        if entry.is_null() {
            return None;
        }
        Some(std::ffi::CStr::from_ptr((*entry).pw_name).to_string_lossy().into_owned())
    }
}

/// Runs `bash check.sh` inside `app_dir` and maps its exit code to a
/// status (0 = installed). The script runs in its own process group so a
/// timeout kills it together with any children it started.
pub fn run_check(app_dir: &Path, timeout: Duration) -> Status {
    let spawned = Command::new("bash")
        .arg(CHECK_SCRIPT)
        .current_dir(app_dir)
        .env(APP_DIR_ENV, app_dir)
        .env(USER_ENV, invoking_user().unwrap_or_default())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
    let mut child = match spawned {
        Ok(child) => child,
        Err(e) => return Status::CheckFailed(format!("cannot run {CHECK_SCRIPT}: {e}")),
    };
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(exit)) if exit.success() => return Status::Installed,
            Ok(Some(_)) => return Status::NotInstalled,
            Ok(None) => {}
            Err(e) => return Status::CheckFailed(format!("waiting for {CHECK_SCRIPT}: {e}")),
        }
        if started.elapsed() >= timeout {
            // SAFETY: kill() is a plain syscall; the negative pid targets the
            // process group created above via process_group(0).
            unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL) };
            let _ = child.wait();
            return Status::CheckFailed(format!("{CHECK_SCRIPT} timed out after {}s", timeout.as_secs()));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Creates a temp app dir whose check.sh has the given body.
    fn app_with_check(name: &str, body: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dreamos-install-status-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(CHECK_SCRIPT), body).unwrap();
        dir
    }

    #[test]
    fn pick_uid_priority() {
        assert_eq!(_pick_uid(Some("1000"), Some("1001"), 0), 1000);
        assert_eq!(_pick_uid(None, Some("1001"), 0), 1001);
        assert_eq!(_pick_uid(Some("bad"), None, 7), 7);
        assert_eq!(_pick_uid(None, None, 0), 0);
    }

    #[test]
    fn exit_zero_is_installed() {
        let dir = app_with_check("ok", "exit 0\n");
        assert_eq!(run_check(&dir, CHECK_TIMEOUT), Status::Installed);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn non_zero_is_not_installed() {
        let dir = app_with_check("ko", "exit 3\n");
        assert_eq!(run_check(&dir, CHECK_TIMEOUT), Status::NotInstalled);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn runs_in_app_dir_with_env() {
        let dir = app_with_check("env", "[ \"$PWD\" = \"$DREAMOS_APP_DIR\" ] && [ -f check.sh ]\n");
        assert_eq!(run_check(&dir, CHECK_TIMEOUT), Status::Installed);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn timeout_is_check_failed() {
        let dir = app_with_check("slow", "sleep 30\n");
        let started = Instant::now();
        let status = run_check(&dir, Duration::from_millis(300));
        assert!(matches!(status, Status::CheckFailed(ref msg) if msg.contains("timed out")));
        assert!(started.elapsed() < Duration::from_secs(5));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn missing_dir_is_check_failed() {
        let status = run_check(Path::new("/nonexistent/dreamos-install-app"), CHECK_TIMEOUT);
        assert!(matches!(status, Status::CheckFailed(_)));
    }
}
