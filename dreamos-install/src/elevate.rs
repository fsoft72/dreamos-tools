use std::ffi::OsString;
use std::path::Path;

/// Session variables pkexec drops but a GTK program needs to reach the
/// user's display (X11 or Wayland) once it runs as root.
pub const SESSION_ENV_VARS: &[&str] = &[
    "DISPLAY",
    "XAUTHORITY",
    "WAYLAND_DISPLAY",
    "XDG_RUNTIME_DIR",
    "XDG_SESSION_TYPE",
    "GDK_BACKEND",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// Arguments for `pkexec`: runs `self_path <root>` through `env` so the
/// session variables found in `lookup` survive the privilege switch.
/// Variables that `lookup` does not have are skipped.
pub fn pkexec_argv(self_path: &Path, root: &Path, lookup: impl Fn(&str) -> Option<OsString>) -> Vec<OsString> {
    let mut argv: Vec<OsString> = vec!["env".into()];
    for name in SESSION_ENV_VARS {
        if let Some(value) = lookup(name) {
            let mut assignment = OsString::from(format!("{name}="));
            assignment.push(value);
            argv.push(assignment);
        }
    }
    argv.push(self_path.into());
    argv.push(root.into());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwards_only_the_present_variables() {
        let lookup = |name: &str| match name {
            "DISPLAY" => Some(OsString::from(":0")),
            "XAUTHORITY" => Some(OsString::from("/tmp/xauth_x")),
            _ => None,
        };
        let argv = pkexec_argv(Path::new("/bin/app"), Path::new("/cat"), lookup);
        let expected: Vec<OsString> = ["env", "DISPLAY=:0", "XAUTHORITY=/tmp/xauth_x", "/bin/app", "/cat"]
            .iter()
            .map(OsString::from)
            .collect();
        assert_eq!(argv, expected);
    }

    #[test]
    fn without_session_variables_only_env_and_the_program() {
        let argv = pkexec_argv(Path::new("/bin/app"), Path::new("/cat"), |_| None);
        assert_eq!(argv.len(), 3);
        assert_eq!(argv[0], "env");
    }
}
