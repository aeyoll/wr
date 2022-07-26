#!/usr/bin/env bash

# shellcheck disable=SC2181

# Reset
Color_Off=''

# Regular Colors
Red=''
Green=''

# Bold
BGreen=''

Dim='' # White

if test -t 1; then
    # Reset
    Color_Off='\033[0m' # Text Reset

    # Regular Colors
    Red='\033[0;31m'   # Red
    Green='\033[0;32m' # Green

    Dim='\033[0;2m' # White

    # Bold
    BGreen='\033[1;32m' # Green
fi

case $(uname -sm) in
    "Darwin x86_64") target="x86_64-apple-darwin" ;;
    "Darwin arm64") target="aarch64-apple-darwin" ;;
    "Linux aarch64") target="aarch64-unknown-linux-gnu" ;;
    "Linux arm64") target="aarch64-unknown-linux-gnu" ;;
    "Linux x86_64") target="x86_64-unknown-linux-gnu" ;;
    *) target="x86_64-unknown-linux-gnu" ;;
esac

if [ "$target" = "x86_64-apple-darwin" ]; then
    # Is it rosetta?
    sysctl sysctl.proc_translated >/dev/null 2>&1
    if [ $? -eq 0 ]; then
        target="aarch64-apple-darwin"
        echo -e "$Dim Your shell is running in Rosetta 2. Downloading wr for $target instead. $Color_Off"
    fi
fi

github_repo="https://github.com/aeyoll/wr"

if [ $# -eq 0 ]; then
    wr_uri="$github_repo/releases/latest/download/wr-${target}.tar.gz"
else
    wr_uri="$github_repo/releases/download/${1}/wr-${target}.tar.gz"
fi

wr_install="${WR_INSTALL:-$HOME/.wr}"
bin_dir="$wr_install/bin"
exe="$bin_dir/wr"

if [ ! -d "$bin_dir" ]; then
    mkdir -p "$bin_dir"

    if (($?)); then
        echo -e "${Red}error${Color_Off}: Failed to create install directory $bin_dir" 1>&2
        exit 1
    fi
fi

curl --fail --location --progress-bar --output "$exe.tar.gz" "$wr_uri"

if (($?)); then
    echo -e "${Red}error${Color_Off}: Failed to download wr from $wr_uri" 1>&2
    exit 1
fi

tar xvzf "$exe.tar.gz" -C "$bin_dir"

if (($?)); then
    echo -e "${Red}error${Color_Off}: Failed to extract wr" 1>&2
    exit 1
fi

chmod +x "$exe"

if (($?)); then
    echo -e "${Red}error${Color_Off}: Failed to set permissions on wr executable." 1>&2
    exit 1
fi

rm "$exe.tar.gz"

echo -e "${Green}wr was installed successfully to ${BGreen}$exe$Color_Off"
