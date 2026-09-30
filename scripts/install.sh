#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Download and install the Oracle ERP MCP release binary.

Usage: install.sh [--owner OWNER] [--repo REPOSITORY] [--version VERSION]
                  [--install-dir DIRECTORY] [--asset NAME]

Defaults:
  --owner        GITHUB_OWNER, or OWNER (replace this placeholder)
  --repo         GITHUB_REPO, or REPO (replace this placeholder)
  --version      VERSION, or latest
  --install-dir  INSTALL_DIR, or $HOME/.local/bin
  --asset        ASSET_NAME, or REPOSITORY-OS-ARCH

The release asset is expected to be a directly downloadable executable.
EOF
}

owner=${GITHUB_OWNER:-OWNER}
repo=${GITHUB_REPO:-REPO}
version=${VERSION:-latest}
install_dir=${INSTALL_DIR:-"${HOME:-}/.local/bin"}
asset=${ASSET_NAME:-}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --owner) [ "$#" -ge 2 ] || { echo "error: --owner requires a value" >&2; exit 2; }; owner=$2; shift 2 ;;
        --repo) [ "$#" -ge 2 ] || { echo "error: --repo requires a value" >&2; exit 2; }; repo=$2; shift 2 ;;
        --version) [ "$#" -ge 2 ] || { echo "error: --version requires a value" >&2; exit 2; }; version=$2; shift 2 ;;
        --install-dir) [ "$#" -ge 2 ] || { echo "error: --install-dir requires a value" >&2; exit 2; }; install_dir=$2; shift 2 ;;
        --asset) [ "$#" -ge 2 ] || { echo "error: --asset requires a value" >&2; exit 2; }; asset=$2; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

[ "$owner" != "OWNER" ] || { echo "error: configure GITHUB_OWNER or pass --owner" >&2; exit 1; }
[ "$repo" != "REPO" ] || { echo "error: configure GITHUB_REPO or pass --repo" >&2; exit 1; }
command -v curl >/dev/null 2>&1 || { echo "error: curl is required" >&2; exit 1; }
command -v uname >/dev/null 2>&1 || { echo "error: uname is required" >&2; exit 1; }

os=$(uname -s)
arch=$(uname -m)
case "$os" in
    Darwin) os=macos ;;
    Linux) os=linux ;;
    *) echo "error: unsupported operating system: $os" >&2; exit 1 ;;
esac
case "$arch" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) echo "error: unsupported architecture: $arch" >&2; exit 1 ;;
esac

[ -n "$asset" ] || asset="${repo}-${os}-${arch}"
case "$version" in
    latest) url="https://github.com/${owner}/${repo}/releases/latest/download/${asset}" ;;
    v*) url="https://github.com/${owner}/${repo}/releases/download/${version}/${asset}" ;;
    *) url="https://github.com/${owner}/${repo}/releases/download/v${version}/${asset}" ;;
esac

mkdir -p "$install_dir"
tmp="${install_dir}/.${repo}.$$"
trap 'rm -f "$tmp"' EXIT HUP INT TERM
echo "Downloading ${url}" >&2
if ! curl --fail --location --silent --show-error "$url" --output "$tmp"; then
    echo "error: could not download release asset ${asset}" >&2
    exit 1
fi
chmod 755 "$tmp"
mv "$tmp" "${install_dir}/oracle-fusion-erp-catalog-mcp"
trap - EXIT HUP INT TERM
echo "Installed ${install_dir}/oracle-fusion-erp-catalog-mcp"
