use std::path::PathBuf;

/// Every variant matches a type of error we
/// want the end-use to see.
// Note: errors from external crates should be wrapped
/// here so that we have full control over the error
/// messages printed to the user.
#[derive(Debug)]
pub enum Error {
    ReadError {
        path: PathBuf,
        io_error: std::io::Error,
    },
    WriteError {
        path: PathBuf,
        io_error: std::io::Error,
    },

    NulByteFound {
        arg: String,
    },
    ProcessStartError {
        message: String,
    },
    ProcessWaitError {
        io_error: std::io::Error,
    },
    ProcessOutError {
        io_error: std::io::Error,
    },

    PipUpgradeFailed {},
    BrokenPipFreezeLine {
        line: String,
    },

    MissingSetupPy {},
    MissingLock {
        expected_path: PathBuf,
    },
    MissingVenv {
        path: PathBuf,
    },

    FileExists {
        path: PathBuf,
    },

    Other {
        message: String,
    },

    MalformedLock {
        line: usize,
        details: String,
    },

    NothingToBump {
        name: String,
    },

    MultipleBumps {
        name: String,
    },
}

/// Implement Display for our Error type
// Note: this is a not-so-bad way to make sure every error message is consistent
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let message = match self {
            Error::Other { message } => message.to_string(),

            Error::NulByteFound { arg } => format!("nul byte found in arg: {:?}", arg),

            Error::ReadError { path, io_error } => {
                format!("could not read {}: {}", path.to_string_lossy(), io_error)
            }
            Error::WriteError { path, io_error } => {
                format!("could not write {}: {}", path.to_string_lossy(), io_error)
            }

            Error::ProcessStartError { message } => format!("could not start process: {}", message),
            Error::ProcessWaitError { io_error } => {
                format!("could not wait for process: {}", io_error)
            }
            Error::ProcessOutError { io_error } => {
                format!("could not get process output: {}", io_error)
            }

            Error::MissingSetupPy {} => {
                "setup.py not found.\n You may want to run `dmenv init` now".to_string()
            }
            Error::MissingLock { expected_path } => format!(
                "{} not found.\n You may want to run `dmenv lock` now",
                expected_path.display()
            ),
            Error::MissingVenv { path } => {
                let mut message = format!("virtualenv in '{}' does not exist\n", path.display());
                message.push_str("Please run `dmenv lock` or `dmenv install` to create it");
                message
            }

            Error::BrokenPipFreezeLine { line } => {
                format!("could not parse `pip freeze` output at line: '{}'", line)
            }
            Error::PipUpgradeFailed {} => {
                "could not upgrade pip. Try using `dmenv clean`".to_string()
            }

            Error::FileExists { path } => format!("{} already exist", path.display()),

            Error::MalformedLock { line, details } => {
                format!("Malformed lock at line {}\n:{}", line, details)
            }
            Error::NothingToBump { name } => format!("'{}' not found in lock", name),
            Error::MultipleBumps { name } => {
                format!("multiple matches found for '{}' in lock", name)
            }
        };
        write!(f, "{}", message)
    }
}
