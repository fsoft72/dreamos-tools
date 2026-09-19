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

use crate::disk::Plan;

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub description: String,
    pub argv: Vec<String>,
}

pub fn steps_for_plan(plan: &Plan) -> Vec<Step> {
    match plan {
        Plan::Create { device, partition_number, start_bytes, end_bytes } => vec![
            Step {
                description: "Create persistence partition".into(),
                argv: vec![
                    "parted".into(), "-s".into(), device.clone(), "unit".into(), "B".into(),
                    "mkpart".into(), "primary".into(), "ext4".into(),
                    format!("{start_bytes}B"), format!("{end_bytes}B"),
                ],
            },
            Step {
                description: "Re-read partition table".into(),
                argv: vec!["partprobe".into(), device.clone()],
            },
            Step {
                description: "Wait for device node".into(),
                argv: vec!["udevadm".into(), "settle".into()],
            },
            Step {
                description: "Format as ext4, label persistence".into(),
                argv: vec![
                    "mkfs.ext4".into(), "-F".into(), "-L".into(), "persistence".into(),
                    format!("{device}{partition_number}"),
                ],
            },
        ],
        Plan::Grow { device, partition_number, new_end_bytes } => vec![
            Step {
                description: "Grow partition".into(),
                argv: vec![
                    "parted".into(), "-s".into(), device.clone(), "unit".into(), "B".into(),
                    "resizepart".into(), partition_number.to_string(), format!("{new_end_bytes}B"),
                ],
            },
            Step {
                description: "Re-read partition table".into(),
                argv: vec!["partprobe".into(), device.clone()],
            },
            Step {
                description: "Wait for device node".into(),
                argv: vec!["udevadm".into(), "settle".into()],
            },
            Step {
                description: "Grow filesystem".into(),
                argv: vec!["resize2fs".into(), format!("{device}{partition_number}")],
            },
        ],
        Plan::NoAction { .. } => vec![],
    }
}

pub fn execute_steps<F>(steps: &[Step], mut on_step: F) -> Result<(), DiskOpError>
where
    F: FnMut(&Step, Result<&CmdOutput, &DiskOpError>),
{
    for step in steps {
        let argv_refs: Vec<&str> = step.argv.iter().map(String::as_str).collect();
        match run_cmd(&argv_refs) {
            Ok(out) => on_step(step, Ok(&out)),
            Err(e) => {
                on_step(step, Err(&e));
                return Err(e);
            }
        }
    }
    Ok(())
}

pub fn write_persistence_conf(mount_point: &str) -> Result<(), DiskOpError> {
    let path = format!("{mount_point}/persistence.conf");
    std::fs::write(&path, "/ union\n").map_err(|e| DiskOpError::Parse {
        what: "writing persistence.conf".into(),
        detail: e.to_string(),
    })
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

#[cfg(test)]
mod step_tests {
    use super::*;
    use crate::disk::Plan;

    #[test]
    fn create_plan_produces_mkpart_then_mkfs() {
        let plan = Plan::Create {
            device: "/dev/sdb".into(),
            partition_number: 3,
            start_bytes: 100,
            end_bytes: 200,
        };
        let steps = steps_for_plan(&plan);
        assert_eq!(steps.len(), 4);
        assert!(steps[0].argv.contains(&"mkpart".to_string()));
        assert_eq!(steps[3].argv.last().unwrap(), "/dev/sdb3");
        assert!(steps[3].argv.contains(&"persistence".to_string()));
    }

    #[test]
    fn grow_plan_produces_resizepart_then_resize2fs() {
        let plan = Plan::Grow {
            device: "/dev/sdb".into(),
            partition_number: 2,
            new_end_bytes: 500,
        };
        let steps = steps_for_plan(&plan);
        assert_eq!(steps.len(), 4);
        assert_eq!(steps[0].argv[0], "parted");
        assert_eq!(steps[3].argv, vec!["resize2fs".to_string(), "/dev/sdb2".to_string()]);
    }

    #[test]
    fn no_action_plan_produces_no_steps() {
        let plan = Plan::NoAction { reason: "x".into() };
        assert!(steps_for_plan(&plan).is_empty());
    }

    #[test]
    fn execute_steps_calls_callback_per_step_and_stops_on_failure() {
        let steps = vec![
            Step { description: "ok".into(), argv: vec!["true".into()] },
            Step { description: "fail".into(), argv: vec!["false".into()] },
            Step { description: "never runs".into(), argv: vec!["true".into()] },
        ];
        let mut seen = Vec::new();
        let result = execute_steps(&steps, |step, res| {
            seen.push((step.description.clone(), res.is_ok()));
        });
        assert!(result.is_err());
        assert_eq!(seen, vec![
            ("ok".to_string(), true),
            ("fail".to_string(), false),
        ]);
    }

    #[test]
    fn write_persistence_conf_writes_expected_content() {
        let dir = std::env::temp_dir().join(format!("filesys-extender-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        write_persistence_conf(dir.to_str().unwrap()).unwrap();
        let content = std::fs::read_to_string(dir.join("persistence.conf")).unwrap();
        assert_eq!(content, "/ union\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
