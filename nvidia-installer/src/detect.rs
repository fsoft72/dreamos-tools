use crate::InstallError;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentDriver {
    Nouveau,
    Nvidia,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuDevice {
    pub pci_slot: String,
    pub model: String,
    pub driver: CurrentDriver,
}

/// Parses `lspci -nnk` output into the NVIDIA (vendor id `10de`) VGA/3D
/// controller entries, skipping companion devices (e.g. the HDMI audio
/// function on the same card) since those never carry a display driver.
pub fn parse_lspci(output: &str) -> Vec<GpuDevice> {
    let mut devices = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(line) = lines.next() {
        let is_display_class = line.contains("VGA compatible controller")
            || line.contains("3D controller");
        if !is_display_class || !line.contains("[10de:") {
            continue;
        }
        let Some(slot) = line.split_whitespace().next() else { continue };
        let model = line
            .split("NVIDIA Corporation ")
            .nth(1)
            .and_then(|rest| rest.split(" [10de:").next())
            .unwrap_or("unknown NVIDIA GPU")
            .to_string();

        let mut driver = CurrentDriver::None;
        while let Some(next_line) = lines.peek() {
            if next_line.starts_with('\t') {
                let next_line = lines.next().unwrap();
                if let Some(name) = next_line.trim().strip_prefix("Kernel driver in use: ") {
                    driver = match name {
                        "nouveau" => CurrentDriver::Nouveau,
                        "nvidia" => CurrentDriver::Nvidia,
                        _ => CurrentDriver::None,
                    };
                }
            } else {
                break;
            }
        }

        devices.push(GpuDevice { pci_slot: slot.to_string(), model, driver });
    }

    devices
}

pub fn detect_gpus() -> Result<Vec<GpuDevice>, InstallError> {
    let output = Command::new("lspci")
        .args(["-nnk"])
        .output()
        .map_err(|source| InstallError::Spawn {
            cmd: "lspci -nnk".into(),
            detail: source.to_string(),
        })?;
    if !output.status.success() {
        return Err(InstallError::CommandFailed {
            cmd: "lspci -nnk".into(),
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    Ok(parse_lspci(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nvidia_with_nouveau_bound() {
        let fixture = include_str!("../tests/fixtures/lspci_nvidia_nouveau.txt");
        let devices = parse_lspci(fixture);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].pci_slot, "01:00.0");
        assert_eq!(devices[0].model, "GA106 [GeForce RTX 3060]");
        assert_eq!(devices[0].driver, CurrentDriver::Nouveau);
    }

    #[test]
    fn parses_nvidia_with_proprietary_bound() {
        let fixture = include_str!("../tests/fixtures/lspci_nvidia_proprietary.txt");
        let devices = parse_lspci(fixture);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].driver, CurrentDriver::Nvidia);
    }

    #[test]
    fn no_nvidia_returns_empty() {
        let fixture = include_str!("../tests/fixtures/lspci_no_nvidia.txt");
        let devices = parse_lspci(fixture);
        assert!(devices.is_empty());
    }
}
