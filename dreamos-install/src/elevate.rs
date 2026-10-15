use std::ffi::OsString;
use std::path::Path;

/// Escape hatch for UI-only iteration (see scripts/run-dreamos-install.sh):
/// scripts run unprivileged, without any polkit prompt.
pub const SKIP_ROOT_ENV: &str = "DREAMOS_INSTALL_SKIP_ROOT";

/// Command line that runs `bash <script>` inside `app_dir` with the given
/// environment. When `elevate` is true the command goes through `pkexec`
/// (which resets the environment and the working directory, hence `env -C`),
/// so the password is asked only when a script actually runs.
pub fn script_argv(app_dir: &Path, script: &str, vars: &[(&str, &str)], elevate: bool) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();
    if elevate {
        argv.push("pkexec".into());
    }
    argv.push("env".into());
    argv.push("-C".into());
    argv.push(app_dir.into());
    for (name, value) in vars {
        argv.push(format!("{name}={value}").into());
    }
    argv.push("bash".into());
    argv.push(script.into());
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(argv: Vec<OsString>) -> Vec<String> {
        argv.into_iter().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    #[test]
    fn elevated_goes_through_pkexec() {
        let argv = script_argv(Path::new("/cat/app"), "install.sh", &[("DREAMOS_USER", "fabio")], true);
        assert_eq!(
            strings(argv),
            ["pkexec", "env", "-C", "/cat/app", "DREAMOS_USER=fabio", "bash", "install.sh"]
        );
    }

    #[test]
    fn unelevated_skips_pkexec() {
        let argv = script_argv(Path::new("/cat/app"), "install.sh", &[], false);
        assert_eq!(strings(argv), ["env", "-C", "/cat/app", "bash", "install.sh"]);
    }
}
