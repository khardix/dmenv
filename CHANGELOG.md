# 0.12.0

## Allow access to system site packages

* Use `dmenv --system-site-packages install` and/or `dmenv --system-site-packages lock` to create a virtual environment that has access to the system's site packages. In the latter case, dependencies outside the virtual environment are *not* included in the lock file.

## Allow skipping dev dependencies

This is done with the `--production` flag. For instance, `dmenv --production install`.
`dmenv --production lock` will create a `production.lock` that contains no development dependencies.

## Breaking changes

Virtualenv location has changed to allow both production and full virtual environments to coexist:

* When using `DMENV_VENV_OUTSIDE_PROJECT`

| version | location |
|-|----------|
| <= 0.11 | DATA_DIR/dmenv/venv/3.7.1/foo/
| >= 0.12, default | DATA_DIR/dmenv/venv/dev/3.7.1/foo/
| >= 0.12, with --production |  DATA_DIR/dmenv/venv/prod/3.7.1/foo/


* Otherwise:

| version | location |
|-|----------|
| <= 0.11 | .venv/3.7.1/foo/ |
| >= 0.12, default | .venv/dev/3.7.1/foo/ |
| >= 0.12, with --production | .venv/prod/3.7.1/foo/ |

## Migrating from 0.11

* Run `dmenv clean` with `dmenv 0.11` to clean up the deprecated location
* Upgrade to `dmenv 0.12`
* Run `dmenv install`  to create the new virtual environment

# 0.11.1

* Fix metadata on Cargo to include new tagline.

# 0.11.0

* Add `dmenv show:bin_path` to show the path of the virtual environment binaries.

## Breaking changes

* Fix [#31](https://github.com/TankerHQ/dmenv/issues/31): make sure the wheel
  package gets frozen when running `dmenv lock`. Note: this also causes other packages
  like `setuptools` and `pip` itself to get frozen. As a consequence `dmenv
  install` no longer upgrades pip automatically, and so the `--no-upgrade-pip` option
  is gone.

# 0.10.0

* Allow using `dmenv` outside the current project, by setting an environment variable named `DMENV_VENV_OUTSIDE_PROJECT`.

# 0.9.0

* Fix [#54](https://github.com/TankerHQ/dmenv/issues/54): rename `--cwd` option to `--project`.

* Avoid blindly overwriting the `requirements.lock` file when running.
  `dmenv lock`. See [#11](https://github.com/TankerHQ/dmenv/issues/11) and [#7](https://github.com/TankerHQ/dmenv/issues/7) for background.

# 0.8.4

* Fix [#49](https://github.com/TankerHQ/dmenv/issues/49): return code was always 0 when using `dmenv run` on Windows. (regression introduced in `0.8.1`).

# 0.8.3

* Add documentation link to `Cargo.toml`.

# 0.8.2

* Fix [#45](https://github.com/TankerHQ/dmenv/issues/45): `dmenv env` can be used with non-ASCII chars on Windows.

# 0.8.1

* `dmenv run` now uses `execv` from `libc`. This means the child process is killed when killing `dmenv`.
   The previous behavior (starting a new subprocess) can be activated with the `--no-exec` option.

# 0.8.0

* Allow using `python3 -m virtualenv` instead of `python3 -m venv` to create the virtual
  environments by setting an environment variable named `DMENV_NO_VENV_STDLIB`. This can be used to work around
  some bugs in Debian-based distributions.

# 0.7.0

* Add `bump-in-lock` command. Use to bump version or git references in the `requirements.lock`
  file.

# 0.6.0

* Run `setup.py develop` with `--no-deps`.
* Rename `show` to `show:venv_path`, add `show:deps` to display the list of dependencies.

# 0.5.0

* `dmenv init`: since name is required, it is now an argument, no longer an option.
  So instead of `dmenv init --name foo --version 0.42`, use `dmenv init foo --version 0.42`
* Add a command named `dmenv develop` that just runs `python setup.py develop` and nothing else.
* `dmenv install`: add `--no-upgrade-pip` and `--no-develop` options.

# 0.4.3

* Add a `--author` option to `dmenv init`, used when generating the `setup.py` file.
* Fix [#12](https://github.com/TankerHQ/dmenv/issues/12): `dmenv lock` now exits immediately if the lock file is missing.
* Workaround Debian bug in pip (See [#15](https://github.com/TankerHQ/dmenv/issues/15) for details).

# 0.4.2

* Write some metadata inside the `requirements.lock` file.

* Improve `dmenv run`:
  * Suggest running `lock` or `install`
  * Do not crash if used without arguments


# 0.4.1

* Fix CI on Windows.

# 0.4.0

* `dmenv` no longer needs a configuration file.
* Find the Python interpreter to use by looking in the PATH environment variable.

# 0.3.4

* Fix [#9](https://github.com/TankerHQ/dmenv/issues/9): If `dmenv` is run *inside an existing virtual environment*, just use it.

# 0.3.3

* Also upgrade pip when running `dmenv install`.
* Fix incorrect message when running `dmenv lock`.

# 0.3.2

* Fix regression introduced in 0.3.1: create config path parent subdirectory
  before trying to write inside it.

# 0.3.1

* Add a `dmenv` subdirectory to the configuration file path.

# 0.3.0

* Replace command `freeze` by `lock`.

# 0.2.3

* Add command `dmenv init` to generate a working `setup.py` file.

# 0.2.2

* Fix running dmenv on Windows.
* The configuration file is now read from $HOME (`~/.config` on Linux and macOS, `%HOME%\AppData\Local` on Windows).

# 0.2.1

* The `.dmenv.toml` file is now required.

# 0.2.0

* Can be used with multiple python versions, using the `.dmenv.toml` config file.

# 0.1.0

* Initial release.
