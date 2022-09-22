# wr

[![LICENSE](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Minimum rustc version](https://img.shields.io/badge/rustc-1.48.0+-lightgray.svg)](#rust-version-requirements)

A Rust tool to deploy websites with ease, using git-flow and Gitlab CI/CD.

Philosophy
---

Warning: personal opinion here.

While git-flow is not trendy _at all_, it still has advantages over GitHub flow for website deployment:

- clear distinction between production and development environments
- a strong usage of tags, providing an easy rollback mechanism
- a good convention for branch names

This tool is only intended to help to create and deploy new releases. Handling gitflow's features, hotfixes and bugfixes are (and will) not be covered.

Installation
---

To install wr, use the install-script and add `$HOME/.wr/bin` to your `$PATH`.

```shell
curl -fsSL https://raw.githubusercontent.com/aeyoll/wr/main/install.sh | bash
```

Configuration
----

Setup some environment variables:

```sh
export WR_GITLAB_HOST=gitlab.com # default to gitlab.com, but it can be a private instance
export WR_GITLAB_TOKEN=glpat-012345678012345678 # GitLab access token with "api" rights
```

Usage
---

```
USAGE:
    wr [OPTIONS]

OPTIONS:
    -d, --debug                        Print additional debug information
        --deploy                       Launch a deploy job after the release
    -e, --environment <ENVIRONMENT>    Define the deploy environment [default: production] [possible
                                       values: production, staging]
    -f, --force                        Allow to make a release even if the remote is up to date
    -h, --help                         Print help information
    -s, --semver-type <SEMVER_TYPE>    Define how to increment the version number [default: patch]
                                       [possible values: major, minor, patch]
    -V, --version                      Print version information
```

Examples:
---

Create a staging release and deploy it:

```sh
wr --environment=Staging --deploy
```

Create a production release, with logger level set at "debug", incrementing to the next minor version:

```sh
wr --semver_type=Minor --debug
wr --environment=Production --semver_type=Minor --debug
# Those two lines are equivalent
```
