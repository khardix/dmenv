pub struct Settings {
    pub venv_from_stdlib: bool,
    pub venv_outside_project: bool,
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            venv_from_stdlib: true,
            venv_outside_project: false,
        }
    }
}
