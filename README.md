# rgs

## Description

`rgs` (recheck-git-status) is a simple tool for us forgetful people. Namely, what the tool does is checks a folder with git repositories for ones that have been modified and/or that have commits to be pushed/pulled.

Usage revolves around a simple env variable `CODE`. That variable should be the path of the folder which contains repositories e.g. `$HOME/projects`.

The tool recursively checks all child folders for valid git repositories and checks their status. Tool can be also called with an option to periodically fetch from a remote or send out notifications when there are commits to be pulled.

Tested and working on Linux and Windows.

## Usage

```
USAGE:
    cgs [FLAGS] [OPTIONS] --code <code> [REPOS]...

FLAGS:
    -a, --all          show both clean and dirty repositories
    -d, --dir          show all repository directories (turns off -t and -m flags)
    -e, --exit         exit on first non-zero repository ahead-behind diff
    -f, --fetch        also fetch from origin
    -h, --help         Prints help information
    -m, --mod          show modifications or ahead/behind status
    -i, --no-ignore    don't read .codeignore file
    -n, --notify       send an OS notification on every non-zero diff
    -t, --time         show execution time
    -V, --version      Prints version information
    -v, --verbose      print additional information
    -w, --watch        watch for changes

OPTIONS:
    -c, --code <code>          override CODE variable [env: CODE=/home/nik/.local/src]
    -D, --depth <depth>        project search recursive depth [default: 2]
    -p, --profile <profile>    load profile configuration from 'coderc'
    -s, --sort <sort>          sort by: directory (d), modifications (m), time (t), ahead-behind (a)
    -j, --jobs <threads>       number of threads, default: number of logical cpus
    -T, --timeout <timeout>    timeout in seconds between git fetches [default: 60]

ARGS:
    <REPOS>...    list of repositories to watch
```
By default `rgs` assumes that repositories are categorized in one of the following way in a root folder as show by the tree below.

Also, by default `rgs` uses 2 as the recursion depth which can be changed with `-D` option.

```
CODE
├── group1
│   ├── repository1
│   ├── repository2
│   ├── repository3
│   ├── repository4
│   └── repository5
├── group2
│   ├── repository1
│   ├── repository2
│   ├── repository3
│   ├── repository4
│   ├── repository5
│   └── repository6
├── repository1
└── repository2
```

Root folder is being determined by the `CODE` environmental variable, and it is recommended to have it set up in your `.bashrc` or any `.profile` file as follows that is being sources at shell startup:

```
export CODE=/path/to/your/repos/folder
```

Output should look like this when called with `-m` flag that shows modifications and ahead-behind status.

```
rs               rgs                      ±3
var              OpenRGB                  ±5    ↑  0 ↓ 51
var              i3-gaps                  ±6    ↑  0 ↓  3
```

`-f` - performs `git fetch` for each detected repository

`-a` - outputs all detected repositories regardless of their modified status.

`-t` - times the execution of the whole program and parsing of each repository

```
~ $ cgs -t
android          duncan                                   7ms
rs               rgs                                      1ms
81ms
```

`-d` - outputs the absolute path to the repository. Useful for scripting.

```
~ $ cgs -d
/home/nik/.local/src/android/duncan
/home/nik/.local/src/rs/rgs
```

`-s` - sorts output based on parsed information (modification - m, ahead-behind - a, time - t, directory - d).

`-w` - takes multiple paths to repositories to fetch and watch for commits e.g. `cgs -w uni rs/rgs /home/nik/projs/awesome_proj`. Relative paths are resolved relative to `CODE`.

`-n` - when used with `-w` displays native OS notification with first 10 commits and abbreviated messages.

`-e` - when used with `-w` exists after first non-zero behind commit count. Exit code is number of behind commits.

Few options are available that pretty print the stats:

`-v` - shows all categories and number of repositories in them. Below that are listed all the uncommitted repositories.

```
CODE     1           /home/nik/.local/src/uni
android  1           /home/nik/.local/src/android
c        7           /home/nik/.local/src/c
go       5           /home/nik/.local/src/go
ino      3           /home/nik/.local/src/ino
java     1           /home/nik/.local/src/java
js       14          /home/nik/.local/src/js
py       2           /home/nik/.local/src/py
rs       4           /home/nik/.local/src/rs
sh       5           /home/nik/.local/src/sh
work     11          /home/nik/.local/src/work
android          duncan
rs               rgs
```

`-vv` - shows all categories, and their repositories in a tree like form (same as `tree` linux program) with uncommitted ones being highlighted in red:

```
.
├── category1 (5)
│   ├── repository1
│   ├── repository2
│   ├── repository3
│   ├── repository4
│   └── repository5
├── category2 (6)
│   ├── repository1
│   ├── repository2
│   ├── repository3
│   ├── repository4
│   ├── repository5
│   └── repository6
├── repository1 (1)
└── repository2 (1)
```

### Ignoring

If `.codeignore` file is supplied in the root of `CODE` directory its read for folders to be ignored in the search.

Example:

```
# .codeignore
# comment

target
out
build
cmake-build-debug
cmake-build-release
dist
venv
__pycache__
node_modules

/tmp
/var
# un-ignores neovim repository
!/var/neovim
```

Codeignore file can be disabled using `-i` flag.

### Profiles

Coderc file can be used to predefine some parameters that are often used. Or specify override `CODE` folder.

Profiles checked for `coderc` files are in order:

`$HOME/.config/coderc`
`$HOME/.coderc`

Files are based on TOML structure.

```
[work] # profile name
code = "/home/nik/.local/src/work"
mod = true
sort = "m"
```

Profile is loaded by using `-p` flag e.g. `cgs -p work`.