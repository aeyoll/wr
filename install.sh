#!/usr/bin/env bash

# shellcheck disable=SC2181

# Environment variables:
# WR_TARGET - Override target platform (e.g., x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc)
# WR_INSTALL - Override install directory (e.g., /usr/local/bin)

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

if [ -n "$WR_TARGET" ]; then
    target="$WR_TARGET"
else
    # Detect OS and architecture
    os_type=$(uname -s)
    arch_type=$(uname -m)

    case "$os_type" in
        Darwin)
            case "$arch_type" in
                x86_64) target="x86_64-apple-darwin" ;;
                arm64) target="aarch64-apple-darwin" ;;
                *) target="x86_64-apple-darwin" ;;
            esac
            ;;
        Linux)
            case "$arch_type" in
                aarch64|arm64) target="aarch64-unknown-linux-gnu" ;;
                x86_64) target="x86_64-unknown-linux-gnu" ;;
                *) target="x86_64-unknown-linux-gnu" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            # Windows environment (Git Bash, MSYS2, Cygwin)
            case "$arch_type" in
                x86_64|AMD64) target="x86_64-pc-windows-msvc" ;;
                aarch64|arm64) target="aarch64-pc-windows-msvc" ;;
                *) target="x86_64-pc-windows-msvc" ;;
            esac
            ;;
        *)
            # Default to Linux x86_64
            target="x86_64-unknown-linux-gnu"
            ;;
    esac
fi

# Check for Rosetta on macOS
if [ -z "$WR_TARGET" ] && [ "$target" = "x86_64-apple-darwin" ]; then
    sysctl sysctl.proc_translated >/dev/null 2>&1
    if [ $? -eq 0 ]; then
        target="aarch64-apple-darwin"
        echo -e "$Dim Your shell is running in Rosetta 2. Downloading wr for $target instead. $Color_Off"
    fi
fi

github_repo="https://github.com/aeyoll/wr"

# Determine file extension based on target
case "$target" in
    *-windows-*)
        archive_ext="zip"
        exe_name="wr.exe"
        ;;
    *)
        archive_ext="tar.gz"
        exe_name="wr"
        ;;
esac

if [ $# -eq 0 ]; then
    wr_uri="$github_repo/releases/latest/download/wr-${target}.${archive_ext}"
else
    wr_uri="$github_repo/releases/download/${1}/wr-${target}.${archive_ext}"
fi

wr_install="${WR_INSTALL:-$HOME/.wr}"
bin_dir="$wr_install/bin"
exe="$bin_dir/$exe_name"

if [ ! -d "$bin_dir" ]; then
    mkdir -p "$bin_dir"

    if (($?)); then
        echo -e "${Red}error${Color_Off}: Failed to create install directory $bin_dir" 1>&2
        exit 1
    fi
fi

archive_file="$exe.$archive_ext"

curl --fail --location --progress-bar --output "$archive_file" "$wr_uri"

if (($?)); then
    echo -e "${Red}error${Color_Off}: Failed to download wr from $wr_uri" 1>&2
    exit 1
fi

# Extract based on archive type
if [ "$archive_ext" = "zip" ]; then
    # Check if unzip is available
    if command -v unzip >/dev/null 2>&1; then
        unzip -o "$archive_file" -d "$bin_dir"
    elif command -v 7z >/dev/null 2>&1; then
        7z x "$archive_file" -o"$bin_dir" -y
    else
        echo -e "${Red}error${Color_Off}: Neither unzip nor 7z found. Please install one to extract the archive." 1>&2
        exit 1
    fi
else
    tar xzf "$archive_file" -C "$bin_dir"
fi

if (($?)); then
    echo -e "${Red}error${Color_Off}: Failed to extract wr" 1>&2
    exit 1
fi

chmod +x "$exe" 2>/dev/null

if (($?)); then
    # On Windows, chmod might fail or not be needed
    if [[ ! "$target" =~ windows ]]; then
        echo -e "${Red}error${Color_Off}: Failed to set permissions on wr executable." 1>&2
        exit 1
    fi
fi

rm "$archive_file"

echo -e "${Green}wr was installed successfully to ${BGreen}$exe$Color_Off"

# Show PATH instructions for Windows
if [[ "$target" =~ windows ]]; then
    echo ""
    echo -e "${Dim}To use wr, add it to your PATH or run it directly from:$Color_Off"
    echo -e "${Dim}  $exe$Color_Off"
fi