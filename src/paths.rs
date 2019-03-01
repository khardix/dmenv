use app_dirs::{AppDataType, AppInfo};
use std::path::PathBuf;

const APP_INFO: AppInfo = AppInfo {
    name: "dmenv",
    author: "Tanker",
};

use crate::error::*;

pub struct Paths {
    pub project: PathBuf,
    pub venv: PathBuf,
    pub lock: PathBuf,
    pub setup_py: PathBuf,
}

pub struct PathsResolver {
    pub venv_outside_project: bool,
    pub python_version: String,

    pub project_path: PathBuf,
}

impl PathsResolver {
    pub fn new(project_path: PathBuf, python_version: &str) -> Self {
        PathsResolver {
            venv_outside_project: false,
            project_path,
            python_version: python_version.into(),
        }
    }

    pub fn paths(&self) -> Result<Paths, Error> {
        let venv_path = self.get_venv_path()?;
        Ok(Paths {
            project: self.project_path.clone(),
            venv: venv_path,
            lock: self.project_path.join("requirements.lock"),
            setup_py: self.project_path.join("setup.py"),
        })
    }

    fn get_venv_path(&self) -> Result<PathBuf, Error> {
        if let Ok(existing_venv) = std::env::var("VIRTUAL_ENV") {
            return Ok(PathBuf::from(existing_venv));
        }
        if self.venv_outside_project {
            self.get_venv_path_outside()
        } else {
            self.get_venv_path_inside()
        }
    }

    fn get_venv_path_inside(&self) -> Result<PathBuf, Error> {
        Ok(self.project_path.join(".venv").join(&self.python_version))
    }

    fn get_venv_path_outside(&self) -> Result<PathBuf, Error> {
        let data_dir =
            app_dirs::app_dir(AppDataType::UserCache, &APP_INFO, "venv").map_err(|e| {
                Error::Other {
                    message: format!("Could not create dmenv cache path: {}", e.to_string()),
                }
            })?;
        let project_name = self.project_path.file_name().ok_or_else(|| Error::Other {
            message: format!("project path: {:?} has no file name", self.project_path),
        })?;
        Ok(data_dir.join(&self.python_version).join(project_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolving_paths() {
        let project_path = Path::new("/tmp/foo");
        let python_version = "3.7.1";
        let mut paths_resolver = PathsResolver::new(project_path.to_path_buf(), python_version);
        paths_resolver.venv_outside_project = true;
        let paths = paths_resolver.paths().unwrap();

        assert_eq!(paths.project, project_path);
        assert!(paths.venv.to_string_lossy().contains(python_version));
    }
}
