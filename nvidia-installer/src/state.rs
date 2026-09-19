use crate::InstallError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const STATE_DIR: &str = "/var/lib/nvidia-installer";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct State {
    pub mok_enrollment_pending: bool,
}

impl Default for State {
    fn default() -> Self {
        State { mok_enrollment_pending: false }
    }
}

/// Test override: `NVIDIA_INSTALLER_STATE_DIR` redirects state.json
/// elsewhere so unit tests never touch real system state.
pub fn state_dir() -> PathBuf {
    match std::env::var_os("NVIDIA_INSTALLER_STATE_DIR") {
        Some(dir) => PathBuf::from(dir),
        None => PathBuf::from(STATE_DIR),
    }
}

pub fn state_path() -> PathBuf {
    state_dir().join("state.json")
}

pub fn load_state() -> State {
    let Ok(contents) = std::fs::read_to_string(state_path()) else {
        return State::default();
    };
    serde_json::from_str(&contents).unwrap_or_default()
}

pub fn save_state(state: &State) -> Result<(), InstallError> {
    let dir = state_dir();
    std::fs::create_dir_all(&dir).map_err(|source| InstallError::Spawn {
        cmd: format!("mkdir -p {}", dir.display()),
        detail: source.to_string(),
    })?;
    let json = serde_json::to_string_pretty(state).map_err(|source| InstallError::Parse {
        what: "state.json".into(),
        detail: source.to_string(),
    })?;
    std::fs::write(state_path(), json).map_err(|source| InstallError::Spawn {
        cmd: format!("write {}", state_path().display()),
        detail: source.to_string(),
    })
}

pub fn clear_state() -> Result<(), InstallError> {
    match std::fs::remove_file(state_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(InstallError::Spawn {
            cmd: format!("rm {}", state_path().display()),
            detail: source.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests mutate a process-wide env var, so they must not run concurrently.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_temp_state_dir<F: FnOnce()>(f: F) {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!("nvidia-installer-test-{}", std::process::id()));
        std::env::set_var("NVIDIA_INSTALLER_STATE_DIR", &tmp);
        f();
        let _ = std::fs::remove_dir_all(&tmp);
        std::env::remove_var("NVIDIA_INSTALLER_STATE_DIR");
    }

    #[test]
    fn missing_file_loads_default() {
        with_temp_state_dir(|| {
            assert_eq!(load_state(), State { mok_enrollment_pending: false });
        });
    }

    #[test]
    fn save_then_load_roundtrips() {
        with_temp_state_dir(|| {
            save_state(&State { mok_enrollment_pending: true }).unwrap();
            assert_eq!(load_state(), State { mok_enrollment_pending: true });
        });
    }

    #[test]
    fn clear_removes_file_and_is_idempotent() {
        with_temp_state_dir(|| {
            save_state(&State { mok_enrollment_pending: true }).unwrap();
            clear_state().unwrap();
            assert_eq!(load_state(), State::default());
            clear_state().unwrap(); // second call: file already gone, still Ok
        });
    }
}
