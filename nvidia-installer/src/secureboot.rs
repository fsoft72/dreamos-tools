use crate::InstallError;
use std::path::Path;
use std::process::Command;

pub const MOK_KEY_PATH: &str = "/var/lib/dkms/mok.key";
pub const MOK_CERT_PATH: &str = "/var/lib/dkms/mok.pub";
const MOK_SUBJECT: &str = "dreamos nvidia-installer MOK";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureBootState {
    Enabled,
    Disabled,
    Unknown,
}

pub fn parse_sb_state(output: &str) -> SecureBootState {
    let text = output.to_lowercase();
    if text.contains("secureboot enabled") {
        SecureBootState::Enabled
    } else if text.contains("secureboot disabled") {
        SecureBootState::Disabled
    } else {
        SecureBootState::Unknown
    }
}

pub fn secure_boot_state() -> SecureBootState {
    match Command::new("mokutil").arg("--sb-state").output() {
        Ok(out) => parse_sb_state(&String::from_utf8_lossy(&out.stdout)),
        Err(_) => SecureBootState::Unknown,
    }
}

pub fn mok_keypair_exists() -> bool {
    Path::new(MOK_KEY_PATH).exists() && Path::new(MOK_CERT_PATH).exists()
}

/// argv for generating the MOK signing key pair at the fixed paths dkms
/// itself checks. DER-encoded cert (`-outform DER`) because that's what
/// `mokutil --import` and the kernel's MOK verifier expect; a 100-year
/// validity avoids ever needing key rotation for this internal tool's key.
pub fn openssl_genkey_argv() -> Vec<String> {
    vec![
        "openssl".into(), "req".into(), "-new".into(), "-x509".into(),
        "-newkey".into(), "rsa:2048".into(),
        "-keyout".into(), MOK_KEY_PATH.into(),
        "-outform".into(), "DER".into(),
        "-out".into(), MOK_CERT_PATH.into(),
        "-nodes".into(), "-days".into(), "36500".into(),
        "-subj".into(), format!("/CN={MOK_SUBJECT}/"),
    ]
}

pub fn mokutil_import_argv() -> Vec<String> {
    vec!["mokutil".into(), "--import".into(), MOK_CERT_PATH.into()]
}

pub fn is_key_enrolled(list_enrolled_output: &str) -> bool {
    list_enrolled_output
        .split("[key ")
        .any(|block| block.contains("Subject:") && block.contains(MOK_SUBJECT))
}

pub fn ensure_mok_keypair() -> Result<(), InstallError> {
    if mok_keypair_exists() {
        return Ok(());
    }
    let argv = openssl_genkey_argv();
    let argv_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let cmd_str = argv.join(" ");
    let output = Command::new(argv_refs[0])
        .args(&argv_refs[1..])
        .output()
        .map_err(|source| InstallError::Spawn { cmd: cmd_str.clone(), detail: source.to_string() })?;
    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: cmd_str,
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_enabled() {
        let fixture = include_str!("../tests/fixtures/mokutil_sb_enabled.txt");
        assert_eq!(parse_sb_state(fixture), SecureBootState::Enabled);
    }

    #[test]
    fn parses_disabled() {
        let fixture = include_str!("../tests/fixtures/mokutil_sb_disabled.txt");
        assert_eq!(parse_sb_state(fixture), SecureBootState::Disabled);
    }

    #[test]
    fn unparseable_is_unknown() {
        assert_eq!(parse_sb_state("garbage\n"), SecureBootState::Unknown);
    }

    #[test]
    fn finds_our_key_among_others() {
        let fixture = include_str!("../tests/fixtures/mokutil_list_enrolled.txt");
        assert!(is_key_enrolled(fixture));
    }

    #[test]
    fn absent_key_not_found() {
        let fixture = "[key 1]\nSubject:\n\tCN=Some Other Key\n";
        assert!(!is_key_enrolled(fixture));
    }

    #[test]
    fn genkey_argv_uses_fixed_paths() {
        let argv = openssl_genkey_argv();
        assert!(argv.contains(&MOK_KEY_PATH.to_string()));
        assert!(argv.contains(&MOK_CERT_PATH.to_string()));
    }
}
