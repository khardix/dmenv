use crate::dependencies::{FrozenDependency, LockedDependency, SimpleDependency};
use crate::error::Error;

// Common trait used by any struct able to bump a dependency
trait Bumper {
    /// Modify the dep passed as argument.
    /// Returns true if the dependency actually changed
    fn bump(&self, dep: &mut LockedDependency) -> bool;
}

struct SimpleBumper {
    version: String,
}

/// Changes the `version` field for the `Simple`
/// variant of the `LockedDependency` enum
impl SimpleBumper {
    fn new(version: &str) -> Self {
        SimpleBumper {
            version: version.to_string(),
        }
    }
}

impl Bumper for SimpleBumper {
    fn bump(&self, dep: &mut LockedDependency) -> bool {
        if let LockedDependency::Simple(s) = dep {
            s.bump(&self.version)
        } else {
            false
        }
    }
}

/// Changes the `git_ref` field for the `Git`
/// variant of the `LockedDependency` enum
struct GitBumper {
    git_ref: String,
}

impl GitBumper {
    fn new(git_ref: &str) -> Self {
        GitBumper {
            git_ref: git_ref.to_string(),
        }
    }
}

impl Bumper for GitBumper {
    fn bump(&self, dep: &mut LockedDependency) -> bool {
        if let LockedDependency::Git(g) = dep {
            g.bump(&self.git_ref)
        } else {
            false
        }
    }
}

/// Implements various operations on the lock file
/// Usage:
/// ```text
/// let lock_contents = read_from("foo.lock");
/// let mut lock = Lock::from_string(lock_contents)
/// // Mutate the lock, for instance with `bump()` or `freeze()
/// let lock_contents = lock.to_string();
/// write_to("foo.lock", lock_contents);
/// ```
#[derive(Debug)]
pub struct Lock {
    dependencies: Vec<LockedDependency>,
    python_version: Option<String>,
    sys_platform: Option<String>,
}

impl Lock {
    pub fn from_string(string: &str) -> Result<Self, Error> {
        let mut dependencies = vec![];
        for (i, line) in string.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let dep = LockedDependency::from_line(&line).map_err(|e| Error::MalformedLock {
                line: i + 1,
                details: e.details,
            })?;
            dependencies.push(dep);
        }
        Ok(Lock {
            dependencies,
            python_version: None,
            sys_platform: None,
        })
    }

    /// Serialize the lock to a string
    pub fn to_string(&self) -> String {
        // Dependencies are sorted according to their *lowercase* name.
        // This is consistent with how `pip freeze` is implemented.
        // See bottom of pip/_internal/operations/freeze.py:freeze()
        let mut lines: Vec<_> = self.dependencies.iter().map(|x| x.line()).collect();
        lines.sort_by(|x, y| x.to_lowercase().cmp(&y.to_lowercase()));
        lines.join("\n") + "\n"
    }

    /// Set the python version
    // Note: This cause the behavior of `freeze()` to change.
    // See `add_missing_deps` for details
    pub fn python_version(&mut self, python_version: &str) {
        self.python_version = Some(python_version.to_string())
    }

    /// Set the python platform
    // Note: This cause the behavior of `freeze()` to change.
    // See `add_missing_deps` for details
    pub fn sys_platform(&mut self, sys_platform: &str) {
        self.sys_platform = Some(sys_platform.to_string())
    }

    /// Bump the dependency `name` to new `version`.
    /// Returns a tuple (locked_changed: bool, new_contents: String)
    // Note: the locked_changed boolean is used to improve precision of
    // messages printed by the VenvManager struct.
    pub fn bump(&mut self, name: &str, version: &str) -> Result<bool, Error> {
        let simple_bumper = SimpleBumper::new(version);
        self.bump_impl(&simple_bumper, name)
    }

    /// Bump the git dependency `name` to new `git_ref`.
    /// Returns a tuple (locked_changed: bool, new_contents: String)
    // Note: the locked_changed boolean is used to improve precision of
    // messages printed by the VenvManager struct.
    pub fn git_bump(&mut self, name: &str, git_ref: &str) -> Result<bool, Error> {
        let git_bumper = GitBumper::new(git_ref);
        self.bump_impl(&git_bumper, name)
    }

    // Implement common behavior for any Bumper (regular or git)
    fn bump_impl<T>(&mut self, bumper: &T, name: &str) -> Result<bool, Error>
    where
        T: Bumper,
    {
        let mut changed = true;
        let mut num_matches = 0;
        for dep in &mut self.dependencies {
            if dep.name() == name {
                num_matches += 1;
                changed = bumper.bump(dep);
            }
        }
        if num_matches == 0 {
            return Err(Error::NothingToBump {
                name: name.to_string(),
            });
        }
        if num_matches > 1 {
            return Err(Error::MultipleBumps {
                name: name.to_string(),
            });
        }
        Ok(changed)
    }

    /// Applies a set of new FrozenDependency to the lock
    // Basically, "merge" `self.dependencies` with some new frozen deps and
    // make sure no existing information in the lock is lost
    // This in not an actual merge because we only modify existing lines
    // or add new ones (no deletion ocurrs).
    pub fn freeze(&mut self, deps: &[FrozenDependency]) {
        self.patch_existing_deps(deps);
        self.add_missing_deps(deps);
    }

    /// Add dependencies from `frozen_deps` that were missing in the lock
    fn add_missing_deps(&mut self, frozen_deps: &[FrozenDependency]) {
        let known_names: &Vec<_> = &mut self.dependencies.iter().map(|d| d.name()).collect();
        let new_deps: Vec<_> = frozen_deps
            .iter()
            .filter(|x| !known_names.contains(&&x.name))
            .collect();
        for dep in new_deps {
            // If self.python_version or self.sys_platform is not None,
            // make sure to append that data.
            // For instance, if we generated the lock on Linux and we see a
            // new dependency `foo==42` while running `lock --platform=win32`,
            // we know `foo` *must* be Windows-specify.
            // Thus we want to write `foo==42; sys_platform = "win32"` in the lock
            // so that `foo` is *not* installed when running `pip install` on Linux.
            let mut locked_dep = SimpleDependency::from_frozen(dep);
            if let Some(python_version) = &self.python_version {
                locked_dep.python_version(python_version);
            }
            if let Some(sys_platform) = &self.sys_platform {
                locked_dep.sys_platform(sys_platform);
            }
            println!("+ {}", locked_dep.line);
            self.dependencies.push(LockedDependency::Simple(locked_dep));
        }
    }

    /// Modify dependencies that were in the lock to match those passed in `frozen_deps`
    fn patch_existing_deps(&mut self, frozen_deps: &[FrozenDependency]) {
        for dep in &mut self.dependencies {
            match dep {
                // frozen deps *never* contain git information (because `pip freeze`
                // only returns names and versions), so always keep those in the lock.
                LockedDependency::Git(_) => (),
                LockedDependency::Simple(s) => {
                    Self::patch_existing_dep(s, frozen_deps);
                }
            }
        }
    }

    /// Modify an existing dependency to match the frozen version
    fn patch_existing_dep(dep: &mut SimpleDependency, frozen_deps: &[FrozenDependency]) {
        let frozen_match = frozen_deps.iter().find(|x| x.name == dep.name);
        let frozen_version = match frozen_match {
            None => return,
            Some(frozen) => &frozen.version,
        };
        if &dep.version.value == frozen_version {
            return;
        }

        println!("{}: {} -> {}", dep.name, dep.version.value, &frozen_version);
        dep.freeze(&frozen_version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl FrozenDependency {
        pub fn new(name: &str, version: &str) -> Self {
            FrozenDependency {
                name: name.to_string(),
                version: version.to_string(),
            }
        }
    }

    #[test]
    fn malformed_lock() {
        let lock_contents = "bar==42\ngit://foo/bar.git@master#egggg=bar";
        let actual = Lock::from_string(&lock_contents);
        let actual = actual.unwrap_err();
        match actual {
            Error::MalformedLock { line, .. } => assert_eq!(line, 2),
            _ => panic!("Expecting MalformedLock, got: {}", actual),
        }
    }

    #[test]
    fn simple_bump() {
        let lock_contents = "bar==0.3\nfoo==0.42\n";
        let mut lock = Lock::from_string(lock_contents).unwrap();
        let changed = lock.bump("foo", "0.43").unwrap();
        assert!(changed);
        let expected = lock_contents.replace("0.42", "0.43");
        let actual = lock.to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn dep_not_found() {
        let lock_contents = "bar==0.3\nfoo==0.42\n";
        let mut lock = Lock::from_string(lock_contents).unwrap();
        let actual = lock.bump("no-such", "0.43");
        match actual {
            Err(Error::NothingToBump { name }) => assert_eq!(name, "no-such"),
            _ => panic!("Expecting NothingToBump, got: {:?}", actual),
        }
    }

    #[test]
    fn idem_potent_change() {
        let lock_contents = "bar==0.3\nfoo==0.42\n";
        let mut lock = Lock::from_string(lock_contents).unwrap();
        let changed = lock.bump("bar", "0.3").unwrap();
        let actual = lock.to_string();
        assert!(!changed);
        assert_eq!(actual, lock_contents.to_string());
    }

    #[test]
    fn git_bump() {
        let old_sha1 = "dae42f";
        let lock_contents = format!("git@example.com/bar.git@{}#egg=bar\n", old_sha1);
        let mut lock = Lock::from_string(&lock_contents).unwrap();
        let new_sha1 = "cda431";
        let changed = lock.git_bump("bar", new_sha1).unwrap();
        assert!(changed);
        let expected = lock_contents.replace(old_sha1, new_sha1);
        let actual = lock.to_string();
        assert_eq!(actual, expected);
    }

    fn assert_freeze(contents: &str, frozen: &[FrozenDependency], expected: &str) {
        let mut lock = Lock::from_string(contents).unwrap();
        lock.freeze(frozen);
        let actual = lock.to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn freeze_simple_bump() {
        assert_freeze(
            "foo==0.42\n",
            &[FrozenDependency::new("foo", "0.43")],
            "foo==0.43\n",
        );
    }

    #[test]
    fn freeze_keep_old_deps() {
        assert_freeze(
            "bar==1.3\nfoo==0.42\n",
            &[FrozenDependency::new("foo", "0.43")],
            "bar==1.3\nfoo==0.43\n",
        );
    }

    #[test]
    fn freeze_keep_git_deps() {
        assert_freeze(
            "git@example.com:bar/foo.git@master#egg=foo\n",
            &[FrozenDependency::new("foo", "0.42")],
            "git@example.com:bar/foo.git@master#egg=foo\n",
        );
    }

    #[test]
    fn freeze_keep_specifications() {
        assert_freeze(
            "foo == 1.3 ; python_version >= '3.6'\n",
            &[FrozenDependency::new("foo", "1.4")],
            "foo == 1.4 ; python_version >= '3.6'\n",
        );
    }

    #[test]
    fn freeze_add_new_deps() {
        assert_freeze("", &[FrozenDependency::new("foo", "0.42")], "foo==0.42\n");
    }

    #[test]
    fn freeze_different_version() {
        let mut lock = Lock::from_string("foo==0.42\n").unwrap();
        lock.python_version("< '3.6'");
        lock.freeze(&[
            FrozenDependency::new("foo", "0.42"),
            FrozenDependency::new("bar", "1.3"),
        ]);
        let actual = lock.to_string();
        assert_eq!(actual, "bar==1.3 ; python_version < '3.6'\nfoo==0.42\n");
    }

    #[test]
    fn freeze_different_platform() {
        let mut lock = Lock::from_string("foo==0.42\n").unwrap();
        lock.sys_platform("win32");
        lock.freeze(&[
            FrozenDependency::new("foo", "0.42"),
            FrozenDependency::new("winapi", "1.3"),
        ]);
        let actual = lock.to_string();
        assert_eq!(actual, "foo==0.42\nwinapi==1.3 ; sys_platform == 'win32'\n");
    }

}
