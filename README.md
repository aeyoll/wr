# wr

[![GitHub Actions workflow status](https://github.com/aeyoll/wr/workflows/ci/badge.svg)](https://github.com/aeyoll/wr/actions)
[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.81.0+-lightgray.svg)](#rust-version-requirements)
[![Conventional commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-yellow.svg)](https://conventionalcommits.org)

A Rust tool to deploy websites with ease, using git-flow and Gitlab CI/CD.

Philosophy
---

Warning: personal opinion here.

While git-flow is not trendy _at all_, it still has advantages over GitHub flow for website deployment:

- clear distinction between production and development environments
- a strong usage of tags, providing an easy rollback mechanism
- a good convention for branch names

This tool helps create and deploy releases and hotfixes. git-flow features and bugfixes are not covered.

Installation
---

To install wr, use the install-script and add `$HOME/.wr/bin` to your `$PATH`.

```shell
curl -fsSL https://raw.githubusercontent.com/aeyoll/wr/main/install.sh | bash
```

Environment variables:

- `WR_INSTALL` - Override install directory (e.g., `/usr/local`)
- `WR_TARGET` - Override target platform (e.g., `x86_64-unknown-linux-musl`)

Configuration
----

Setup some environment variables:

```sh
export GITLAB_HOST=gitlab.com # default to gitlab.com, but it can be a private instance
export GITLAB_TOKEN=glpat-012345678012345678 # GitLab access token with "api" rights
```

Usage
---

### Release

```
USAGE:
    wr release [FLAGS] [OPTIONS]

FLAGS:
        --debug      Print additional debug information
    -d, --deploy     Launch a deploy job after the release
    -f, --force      Allow to make a release even if the remote is up to date
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -e, --environment <environment>    Define the deploy environment (default: "Production", available: "Production",
                                       "Staging")
    -s, --semver_type <semver_type>    Define how to increment the version number (default: "Patch", available: "Patch",
                                       "Minor", "Major")
```

### Hotfix

```
USAGE:
    wr hotfix [FLAGS] <COMMIT>...

FLAGS:
        --debug      Print additional debug information
    -d, --deploy     Launch a deploy job after the hotfix
    -f, --force      Kept for consistency with release
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <COMMIT>...    Commit hashes to cherry-pick onto the hotfix, in order
```

Creates a git-flow hotfix named after the next patch tag, cherry-picks the given commits, finishes, and pushes master, develop, and the tag. On failure before push, local master/develop/tag/hotfix branch are rolled back.

Examples:
---

Create a staging release and deploy it:

```sh
wr release --environment=staging --deploy
```

Create a production release, with logger level set at "debug", incrementing to the next minor version:

```sh
wr release --semver_type=minor --debug
wr release --environment=production --semver_type=minor --debug
# Those two lines are equivalent
```

Create a production hotfix from one or more commits and deploy it:

```sh
wr hotfix abc1234 def5678 --deploy
```
