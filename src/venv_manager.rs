use app_dirs::{AppDataType, AppInfo};
use colored::*;

#[cfg(unix)]
use crate::execv::execv;
#[cfg(windows)]
use crate::win_job;

use crate::cmd::*;
use crate::dependencies::FrozenDependency;
use crate::error::*;
use crate::lock::Lock;
use crate::python_info::PythonInfo;
use crate::settings::Settings;

pub const LOCK_FILE_NAME: &str = "requirements.lock";

struct LockMetadata {
    dmenv_version: String,
    python_platform: String,
    python_version: String,
}

#[derive(Default)]
pub struct LockOptions {
    pub python_version: Option<String>,
    pub sys_platform: Option<String>,
}

#[derive(Default)]
pub struct InstallOptions {
    pub develop: bool,
}

pub struct VenvManager {
    paths: Paths,
    python_info: PythonInfo,
    settings: Settings,
}

const APP_INFO: AppInfo = AppInfo {
    name: "dmenv",
    author: "Tanker",
};

impl VenvManager {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(
        project_path: std::path::PathBuf,
        python_info: PythonInfo,
        settings: Settings,
    ) -> Result<Self, Error> {
        let lock_path = project_path.join(LOCK_FILE_NAME);
        let setup_py_path = project_path.join("setup.py");
        let venv_path = Self::get_venv_path(
            &project_path,
            &python_info.version,
            settings.venv_outside_project,
        )?;
        let paths = Paths {
            project: project_path,
            venv: venv_path,
            lock: lock_path,
            setup_py: setup_py_path,
        };
        let venv_manager = VenvManager {
            paths,
            python_info,
            settings,
        };
        Ok(venv_manager)
    }

    fn get_venv_path(
        project_path: &std::path::PathBuf,
        python_version: &str,
        venv_outside_project: bool,
    ) -> Result<std::path::PathBuf, Error> {
        if let Ok(existing_venv) = std::env::var("VIRTUAL_ENV") {
            return Ok(std::path::PathBuf::from(existing_venv));
        }
        if venv_outside_project {
            return Self::get_venv_path_outside(project_path, python_version);
        }
        Self::get_venv_path_inside(project_path, python_version)
    }

    fn get_venv_path_inside(
        project_path: &std::path::PathBuf,
        python_version: &str,
    ) -> Result<std::path::PathBuf, Error> {
        Ok(project_path.join(".venv").join(python_version))
    }

    fn get_venv_path_outside(
        project_path: &std::path::PathBuf,
        python_version: &str,
    ) -> Result<std::path::PathBuf, Error> {
        let data_dir =
            app_dirs::app_dir(AppDataType::UserCache, &APP_INFO, "venv").map_err(|e| {
                Error::Other {
                    message: format!("Could not create dmenv cache path: {}", e.to_string()),
                }
            })?;
        let project_name = project_path.file_name().ok_or_else(|| Error::Other {
            message: format!("project path: {:?} has no file name", project_path),
        })?;
        Ok(data_dir.join(python_version).join(project_name))
    }

    pub fn clean(&self) -> Result<(), Error> {
        print_info_1(&format!("Cleaning {}", &self.paths.venv.to_string_lossy()));
        if !self.paths.venv.exists() {
            return Ok(());
        }
        std::fs::remove_dir_all(&self.paths.venv).map_err(|e| Error::Other {
            message: format!(
                "could not remove {}: {}",
                &self.paths.venv.to_string_lossy(),
                e
            ),
        })
    }

    pub fn develop(&self) -> Result<(), Error> {
        print_info_2("Running setup_py.py develop");
        if !self.paths.setup_py.exists() {
            return Err(Error::MissingSetupPy {});
        }

        self.run_cmd_in_venv("python", vec!["setup.py", "develop", "--no-deps"])
    }

    pub fn install(&self, install_options: &InstallOptions) -> Result<(), Error> {
        print_info_1("Preparing project for developement");
        if !self.paths.lock.exists() {
            return Err(Error::MissingLock {});
        }

        self.ensure_venv()?;
        self.install_from_lock()?;

        if install_options.develop {
            self.develop()?;
        }
        Ok(())
    }

    /// Run a program from the virtualenv, making sure it dies
    /// when we get killed and that the exit code is forwarded
    pub fn run(&self, args: &[String]) -> Result<(), Error> {
        #[cfg(windows)]
        {
            unsafe {
                win_job::setup();
            }
            self.run_no_exec(args)
        }

        #[cfg(unix)]
        {
            let bin_path = &self.get_path_in_venv(&args[0])?;
            let bin_path_str = bin_path.to_str().ok_or(Error::Other {
                message: "Could not convert binary path to String".to_string(),
            })?;
            let mut fixed_args: Vec<String> = args.to_vec();
            fixed_args[0] = bin_path_str.to_string();
            execv(bin_path_str, fixed_args)
        }
    }

    /// On Windows:
    ///   - same as run
    /// On Linux:
    ///   - same as run, but create a new process instead of using execv()
    pub fn run_no_exec(&self, args: &[String]) -> Result<(), Error> {
        self.expect_venv()?;
        let cmd = args[0].clone();
        let args: Vec<&str> = args.iter().skip(1).map(|x| x.as_str()).collect();
        self.run_cmd_in_venv(&cmd, args)
    }

    pub fn lock(&self, lock_options: &LockOptions) -> Result<(), Error> {
        print_info_1("Locking dependencies");
        if !self.paths.setup_py.exists() {
            return Err(Error::MissingSetupPy {});
        }

        self.ensure_venv()?;
        self.upgrade_pip()?;

        self.install_editable()?;

        self.write_lock(&lock_options)?;
        Ok(())
    }

    pub fn show_deps(&self) -> Result<(), Error> {
        self.run_cmd_in_venv("pip", vec!["list"])
    }

    pub fn show_venv_path(&self) -> Result<(), Error> {
        println!("{}", self.paths.venv.to_string_lossy());
        Ok(())
    }

    pub fn show_venv_bin_path(&self) -> Result<(), Error> {
        let bin_path = &self.get_venv_bin_path();
        println!("{}", bin_path.to_string_lossy());
        Ok(())
    }

    pub fn init(&self, name: &str, version: &str, author: &Option<String>) -> Result<(), Error> {
        let path = &self.paths.setup_py;
        if path.exists() {
            return Err(Error::FileExists {
                path: path.to_path_buf(),
            });
        }
        let template = include_str!("setup.in.py");
        let with_name = template.replace("<NAME>", name);
        let with_version = with_name.replace("<VERSION>", version);
        let to_write = if let Some(author) = author {
            with_version.replace("<AUTHOR>", author)
        } else {
            with_version
        };
        std::fs::write(&path, to_write).map_err(|e| Error::WriteError {
            path: path.to_path_buf(),
            io_error: e,
        })?;
        print_info_1("Generated a new setup.py");
        Ok(())
    }

    pub fn bump_in_lock(&self, name: &str, version: &str, git: bool) -> Result<(), Error> {
        print_info_1(&format!("Bumping {} to {} ...", name, version));
        let path = &self.paths.lock;
        let lock_contents = std::fs::read_to_string(&path).map_err(|e| Error::ReadError {
            path: path.to_path_buf(),
            io_error: e,
        })?;
        let mut lock = Lock::from_string(&lock_contents)?;
        let changed = if git {
            lock.git_bump(name, version)
        } else {
            lock.bump(name, version)
        }?;
        if !changed {
            print_warning(&format!("Dependency {} already up-to-date", name.bold()));
            return Ok(());
        }
        let new_contents = lock.to_string();
        std::fs::write(&path, &new_contents).map_err(|e| Error::WriteError {
            path: path.to_path_buf(),
            io_error: e,
        })?;
        println!("{}", "ok!".green());
        Ok(())
    }

    fn ensure_venv(&self) -> Result<(), Error> {
        if self.paths.venv.exists() {
            print_info_2(&format!(
                "Using existing virtualenv: {}",
                self.paths.venv.to_string_lossy()
            ));
        } else {
            self.create_venv()?;
        }
        Ok(())
    }

    fn expect_venv(&self) -> Result<(), Error> {
        if !self.paths.venv.exists() {
            return Err(Error::MissingVenv {
                path: self.paths.venv.clone(),
            });
        }
        Ok(())
    }

    fn create_venv(&self) -> Result<(), Error> {
        let parent_venv_path = &self.paths.venv.parent().ok_or(Error::Other {
            message: "venv_path has no parent".to_string(),
        })?;
        print_info_2(&format!(
            "Creating virtualenv in: {}",
            self.paths.venv.to_string_lossy()
        ));
        std::fs::create_dir_all(&parent_venv_path).map_err(|e| Error::Other {
            message: format!(
                "Could not create {}: {}",
                parent_venv_path.to_string_lossy(),
                e
            ),
        })?;
        let venv_path = &self.paths.venv.to_string_lossy();
        let mut args = vec!["-m"];
        if self.settings.venv_from_stdlib {
            args.push("venv")
        } else {
            args.push("virtualenv")
        };
        args.push(venv_path);
        let python_binary = &self.python_info.binary;
        Self::print_cmd(&python_binary.to_string_lossy(), &args);
        let status = std::process::Command::new(&python_binary)
            .current_dir(&self.paths.project)
            .args(&args)
            .status();
        let status = status.map_err(|e| Error::ProcessWaitError { io_error: e })?;
        if !status.success() {
            return Err(Error::Other {
                message: "failed to create virtualenv".to_string(),
            });
        }
        Ok(())
    }

    fn write_lock(&self, lock_options: &LockOptions) -> Result<(), Error> {
        let metadata = &self.get_metadata()?;

        let lock_path = &self.paths.lock;
        let lock_contents = if lock_path.exists() {
            std::fs::read_to_string(&lock_path).map_err(|e| Error::ReadError {
                path: lock_path.to_path_buf(),
                io_error: e,
            })?
        } else {
            String::new()
        };

        let mut lock = Lock::from_string(&lock_contents)?;
        if let Some(python_version) = &lock_options.python_version {
            lock.python_version(&python_version);
        }
        if let Some(sys_platform) = &lock_options.sys_platform {
            lock.sys_platform(&sys_platform);
        }
        let frozen_deps = self.get_frozen_deps()?;
        lock.freeze(&frozen_deps);
        let new_contents = lock.to_string();

        let LockMetadata {
            dmenv_version,
            python_version,
            python_platform,
        } = metadata;
        let top_comment = format!(
            "# Generated with dmenv {}, python {}, on {}\n",
            dmenv_version, &python_version, &python_platform
        );

        let to_write = top_comment + &new_contents;
        std::fs::write(&lock_path, &to_write).map_err(|e| Error::WriteError {
            path: lock_path.to_path_buf(),
            io_error: e,
        })
    }

    fn get_frozen_deps(&self) -> Result<Vec<FrozenDependency>, Error> {
        let freeze_output = self.run_pip_freeze()?;
        let mut res = vec![];
        for line in freeze_output.lines() {
            let frozen_dep = FrozenDependency::from_string(&line)?;
            // Filter out pkg-resources. This works around
            // a Debian bug in pip: https://bugs.debian.org/cgi-bin/bugreport.cgi?bug=871790
            if frozen_dep.name != "pkg-resources" {
                res.push(frozen_dep);
            }
        }

        Ok(res)
    }

    fn run_pip_freeze(&self) -> Result<String, Error> {
        print_info_2(&format!("Generating {}", LOCK_FILE_NAME));
        let pip = self.get_path_in_venv("pip")?;
        let pip_str = pip.to_string_lossy().to_string();
        let args = vec!["freeze", "--exclude-editable", "--all"];
        Self::print_cmd(&pip_str, &args);
        let command = std::process::Command::new(pip)
            .current_dir(&self.paths.project)
            .args(args)
            .output();
        let command = command.map_err(|e| Error::ProcessOutError { io_error: e })?;
        if !command.status.success() {
            return Err(Error::Other {
                message: format!(
                    "pip freeze failed: {}",
                    String::from_utf8_lossy(&command.stderr)
                ),
            });
        }
        Ok(String::from_utf8_lossy(&command.stdout).to_string())
    }

    fn get_metadata(&self) -> Result<LockMetadata, Error> {
        let dmenv_version = env!("CARGO_PKG_VERSION");
        let python_platform = &self.python_info.platform;
        let python_version = &self.python_info.version;
        Ok(LockMetadata {
            dmenv_version: dmenv_version.to_string(),
            python_platform: python_platform.to_string(),
            python_version: python_version.to_string(),
        })
    }

    fn install_from_lock(&self) -> Result<(), Error> {
        print_info_2(&format!("Installing dependencies from {}", LOCK_FILE_NAME));
        let as_str = &self.paths.lock.to_string_lossy();
        let args = vec!["-m", "pip", "install", "--requirement", as_str];
        self.run_cmd_in_venv("python", args)
    }

    pub fn upgrade_pip(&self) -> Result<(), Error> {
        print_info_2("Upgrading pip");
        let args = vec!["-m", "pip", "install", "pip", "--upgrade"];
        self.run_cmd_in_venv("python", args)
            .map_err(|_| Error::PipUpgradeFailed {})
    }

    fn install_editable(&self) -> Result<(), Error> {
        print_info_2("Installing deps from setup.py");

        // tells pip to run `setup.py develop` (that's --editable), and
        // install the dev requirements too
        let args = vec!["-m", "pip", "install", "--editable", ".[dev]"];
        self.run_cmd_in_venv("python", args)
    }

    fn run_cmd_in_venv(&self, name: &str, args: Vec<&str>) -> Result<(), Error> {
        let bin_path = &self.get_path_in_venv(name)?;
        Self::print_cmd(&bin_path.to_string_lossy(), &args);
        let command = std::process::Command::new(bin_path)
            .args(args)
            .current_dir(&self.paths.project)
            .status();
        let command = command.map_err(|e| Error::ProcessWaitError { io_error: e })?;
        if !command.success() {
            return Err(Error::Other {
                message: "command failed".to_string(),
            });
        }

        Ok(())
    }

    fn get_venv_bin_path(&self) -> std::path::PathBuf {
        #[cfg(not(windows))]
        let binaries_subdirs = "bin";

        #[cfg(windows)]
        let binaries_subdirs = "Scripts";

        self.paths.venv.join(binaries_subdirs)
    }

    fn get_path_in_venv(&self, name: &str) -> Result<std::path::PathBuf, Error> {
        if !self.paths.venv.exists() {
            return Err(Error::Other {
                message: format!(
                    "virtualenv in {} does not exist",
                    &self.paths.venv.to_string_lossy()
                ),
            });
        }

        #[cfg(windows)]
        let suffix = ".exe";
        #[cfg(not(windows))]
        let suffix = "";

        let name = format!("{}{}", name, suffix);
        let bin_path = &self.get_venv_bin_path();
        let path = self.paths.venv.join(bin_path).join(name);
        if !path.exists() {
            return Err(Error::Other {
                message: format!("Cannot run: '{}' does not exist", &path.to_string_lossy()),
            });
        }
        Ok(path)
    }

    fn print_cmd(bin_path: &str, args: &[&str]) {
        println!("{} {} {}", "$".blue(), bin_path, args.join(" "));
    }
}

struct Paths {
    project: std::path::PathBuf,
    venv: std::path::PathBuf,
    lock: std::path::PathBuf,
    setup_py: std::path::PathBuf,
}
