#!/usr/bin/env bash
set -euo pipefail

version="130"
archive="binaryen-version_${version}-x86_64-linux.tar.gz"
url="https://github.com/WebAssembly/binaryen/releases/download/version_${version}/${archive}"
expected_sha256="0a18362361ad05465118cd8eeb72edaeec89de6894bc283576ef4e07aa3babcc"
install_root="${HOME}/.local/binaryen-version_${version}"

temporary_directory=$(mktemp -d)
trap 'rm -rf "$temporary_directory"' EXIT
curl --fail --location --silent --show-error "$url" --output "$temporary_directory/$archive"
printf '%s  %s\n' "$expected_sha256" "$temporary_directory/$archive" | sha256sum --check --status
mkdir -p "$install_root"
tar --extract --gzip --file "$temporary_directory/$archive" --strip-components=1 --directory "$install_root"
"$install_root/bin/wasm-opt" --version | grep --fixed-strings "wasm-opt version ${version}"
printf '%s\n' "$install_root/bin" >> "${GITHUB_PATH:?GITHUB_PATH must be set by GitHub Actions}"
