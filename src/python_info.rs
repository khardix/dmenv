use crate::error::Error;
use std::path::PathBuf;

/// Represent output of the info.py script
/// This allows dmenv to know details about
/// the Python intrepreter it is using.
pub struct PythonInfo {
    pub binary: PathBuf,
    pub version: String,
    pub platform: String,
}

impl PythonInfo {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(python: &Option<String>) -> Result<Self, Error> {
        let binary = get_python_binary(&python)?;
        let info_script = include_str!("info.py");

        let command = std::process::Command::new(&binary)
            .args(&["-c", info_script])
            .output();
        let command = command.map_err(|e| Error::ProcessOutError { io_error: e })?;
        if !command.status.success() {
            return Err(Error::Other {
                message: format!(
                    "Failed to run info script: {}",
                    String::from_utf8_lossy(&command.stderr)
                ),
            });
        }
        let info_out = String::from_utf8_lossy(&command.stdout);
        let lines: Vec<_> = info_out.split('\n').collect();
        let expected_lines = 3; // Keep this in sync with src/info.py
        if lines.len() != 3 {
            return Err(Error::Other {
                message: format!(
                    "Expected {} lines in info_out, got: {}",
                    expected_lines,
                    lines.len()
                ),
            });
        }
        let version = lines[0].trim().to_string();
        let platform = lines[1].trim().to_string();
        Ok(PythonInfo {
            binary,
            version,
            platform,
        })
    }
}

/// Look for a suitable Python binary in PATH
// Note: doses not get called if `dmenv` was invoked with an explicit `--python`
// option.
fn get_python_binary(requested_python: &Option<String>) -> Result<PathBuf, Error> {
    if let Some(python) = requested_python {
        return Ok(PathBuf::from(python));
    }

    if let Ok(python3) = which::which("python3") {
        return Ok(python3);
    }

    which::which("python").map_err(|_| Error::Other {
        message: "Neither `python3` nor `python` found in PATH".to_string(),
    })
}
