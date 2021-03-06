# Advanced Usage

## Some definitions

Let's assume the following `setup.py`:

```python
# setup.py
setup(
    name="demo",
    version="0.6.1",
    ...
    install_requires=[
      "path.py"
    ],
    extras_require={
        "dev": [
            "pytest",
        ]
    },
)
```

Since `pytest` is only used by the tests, and not the rest of the code,
`path.py` is a *regular dependency*, and `pytest` is a *development
dependency*.

Now let's assume you've run `dmenv lock` and that the following lock file was produced:

```text
# requirements.lock
path-py==11.4.0
importlib-metadata==0.5
pytest==3.9.0
py==1.6.0
```

`importlib-metadata` and `py` are dependencies of `path.py` and `pytest`
respectively, and their version got "frozen" in the lock file.

To differentiate those dependencies from the rest, we say that `pytest` and
`path.py` are *abstract*, and than `importlib-metadata` and `py` are
*concrete*.

Note that if you publish your code on pypi.org, consumers of your package will
only see the *abstract*, *regular* dependencies, so be careful with the
`install_requires` section of the `setup.py`!

## How the lock command works

The `requirements.lock` is obtained by parsing the output of `pip freeze`,
and thus is only a reflection of the *state* of the virtual environment from which `pip`
was run.

That means the result of the lock depends of something
"stateful" that can change independently of the contents of the `setup.py`.

For instance, if you run `dmenv lock` in a empty virtual environment, every concrete
dependency gets frozen to their latest compatible version.

On the other hand, if you run `dmenv lock` from a virtual environment that already
contains `foo`, the `foo` version won't change (unless something in the
`setup.py` causes it to change).

This may seem like an horrible bug but, as we'll see in the next section,
it makes it possible to use various interesting workflows when upgrading
dependencies.

Two features of `dmenv` make this work:

* One, both `setup.py` and `requirements.lock` can be edited by hand.
* Two, when the lock file already exists, `dmenv lock` "applies" the result of `pip freeze`
  to the existing lock file, and thus can preserve manual changes.

Let's see some examples.


## Upgrade all the things!

The simplest way is to just re-run `dmenv lock` after having cleaned the virtual environment:

```
$ dmenv clean
$ dmenv lock
```

That way, all existing dependencies from the `requirements.lock` will get
ignored, and you'll get the latest version of everything listed in the
`setup.py`.

Give it a go, it often works better than you might think :)

If something breaks (for instance when going from `path.py` 11.4 to `path.py`
11.5), you can edit the `setup.py` to specify that you are *not* compatible
with the latest of path.py:

```python
setup(
    ...
    install_requires=[
      "path.py < 11.5",
    ],
)
```

## Freeze dev dependencies

The above approach does not work really well if you use a linter like `pylint`
or `flake8`, of even a type checker like `mypy`

This is because new releases of those tools often cause new warnings or errors
to be produced, so you only want to update them when you're ready.

Thus, a good practice is to freeze the versions of those tools directly in the
`setup.py`:

```python
setup(

  extras_require={
        "dev": [
            "flake8==3.5.0",
        ]
  }

)
```

That way you can freely re-run `dmenv lock`, even in a completely fresh
environment.


## Upgrading just one development dependency

For instance if there's a bug in `py`, you can bump `py` version by editing the
lock file directly:

```patch
- py==1.6.0
+ py==1.7.0
```

## Upgrading just one regular dependency

If the bug is in one of the concrete dependencies, you should update the `setup.py` file instead

```patch
    install_requires=[
      "path.py"
+     "importlib-metadata >= 0.6"
  ]
```

That way consumers of your code *will* get the correct version.

Then run `dmenv lock` without cleaning the virtual environment so that
`importlib-metadata` gets upgraded and its new version frozen.


## Using dependencies from git URLs

Let's say you came across a bug that's only fixed on the `master` branch of
`pytest` on GitHub, specifically at the commit `deadbeef`.

One solution is to replace the line in `requirements.lock` to use a git URL
like this:

```text
# requirements
git+https://github.com/pytest-dev/pytest@deadbeef#egg=pytest
```

In that case, `pip freeze` will contain a line looking like `pytest==4.0b1`, where `4.0b1`
is the `pytest` version at this particular commit.

When it comes to re-generating the lock, `dmenv` will see that there is already a line
specifying the `pytest` version in a more precise manner, so it will keep the `git` line
in the lock and ignore the non-precise `4.0b1` version.


## Using different system platforms

Sometimes a concrete dependency will only be available on a specify platform.

So if you've generated the lock file on Linux, you may get different results on Windows.

One way to solve this is to run `dmenv lock` with the `--platfrom` argument.

Existing lines in the lock file will be kept, and any *new* dependency will be suffixed
with a [platform marker](https://www.python.org/dev/peps/pep-0508/), like this:

```text
# requirements.lock, generated on liux
foo==0.2

> dmenv lock --platform windows   # run on Windows

# requirements.lock
foo==0.2
pywin2==0.42 ; platform == "windows"
```

## Using different Python versions

If you want your code to be run across different Python versions, you may encounter similar issues.

Sometimes one of your concrete dependency will *only* be required for old interpreters.

For instance, the `pathlib2` package is only useful for 3.5 and below. After
that you simply use the standard library.

In than case, you can specify a python version requirement, like this:

```text
# requirements.lock , generated with Python 3.6
foo==0.2

$ dmenv lock --python '< "3.5"'  <- note the quotes

# requirements.lock
foo==0.2
bar==0.42 ; python_version < "3.5"
```

## Skipping development dependencies

Sometimes you will want to skip development dependencies.

In this case, prefix your `dmenv` command with a `--production` flag, like so:

```
$ dmenv --production lock
$ dmenv --production install
```
