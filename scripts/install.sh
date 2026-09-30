#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Download and install the Oracle ERP MCP release binary.

Usage: install.sh [--owner OWNER] [--repo REPOSITORY] [--version VERSION]
                  [--install-dir DIRECTORY] [--asset NAME] [--database PATH]
                  [--agent AGENT]

Defaults:
  --owner        thegreatyamori
  --repo         oracle-fusion-erp-catalog-mcp
  --version      VERSION, or latest
  --install-dir  INSTALL_DIR, or $HOME/.local/bin
  --asset        ASSET_NAME, or REPOSITORY-OS-ARCH
  --database     DATABASE, or $PWD/oracle-fusion-erp-catalog-mcp.sqlite
  --agent        cursor, claude-code, codex, opencode, all, or none

The release asset is expected to be a directly downloadable executable.
Without --agent, the installer asks which agent should receive the MCP
configuration when a terminal is available.
EOF
}

owner=${GITHUB_OWNER:-thegreatyamori}
repo=${GITHUB_REPO:-oracle-fusion-erp-catalog-mcp}
version=${VERSION:-latest}
install_dir=${INSTALL_DIR:-"${HOME:-}/.local/bin"}
asset=${ASSET_NAME:-}
database=${DATABASE:-"${PWD}/oracle-fusion-erp-catalog-mcp.sqlite"}
agent=${AGENT:-}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --owner) [ "$#" -ge 2 ] || { echo "error: --owner requires a value" >&2; exit 2; }; owner=$2; shift 2 ;;
        --repo) [ "$#" -ge 2 ] || { echo "error: --repo requires a value" >&2; exit 2; }; repo=$2; shift 2 ;;
        --version) [ "$#" -ge 2 ] || { echo "error: --version requires a value" >&2; exit 2; }; version=$2; shift 2 ;;
        --install-dir) [ "$#" -ge 2 ] || { echo "error: --install-dir requires a value" >&2; exit 2; }; install_dir=$2; shift 2 ;;
        --asset) [ "$#" -ge 2 ] || { echo "error: --asset requires a value" >&2; exit 2; }; asset=$2; shift 2 ;;
        --database) [ "$#" -ge 2 ] || { echo "error: --database requires a value" >&2; exit 2; }; database=$2; shift 2 ;;
        --agent) [ "$#" -ge 2 ] || { echo "error: --agent requires a value" >&2; exit 2; }; agent=$2; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "error: unknown option: $1" >&2; usage >&2; exit 2 ;;
    esac
done

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

if [ "$os" = "macos" ] && [ "$arch" = "x86_64" ]; then
    echo "error: Intel macOS is not supported; use an Apple Silicon build or Linux release" >&2
    exit 1
fi

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
binary="${install_dir}/oracle-fusion-erp-catalog-mcp"
mv "$tmp" "$binary"
trap - EXIT HUP INT TERM
echo "Installed ${binary}"

if [ -z "$agent" ] && [ -r /dev/tty ] && [ -w /dev/tty ]; then
    printf '%s\n' \
        "Where should the MCP be installed?" \
        "  1) Cursor" \
        "  2) Claude Code" \
        "  3) Codex CLI" \
        "  4) OpenCode" \
        "  5) All" \
        "  6) None (binary only)" >&2
    printf 'Choose [1-6] (default: 1): ' >&2
    read -r choice </dev/tty || choice=1
    case "$choice" in
        1|"") agent=cursor ;;
        2) agent=claude-code ;;
        3) agent=codex ;;
        4) agent=opencode ;;
        5) agent=all ;;
        6) agent=none ;;
        *) echo "error: choose a number from 1 to 6" >&2; exit 2 ;;
    esac
fi

agent=${agent:-none}
case "$agent" in
    cursor|claude-code|codex|opencode|all)
        "$binary" install "$agent" --binary "$binary" --database "$database"
        ;;
    none)
        echo "Skipped agent configuration."
        ;;
    *)
        echo "error: unsupported agent '$agent'; use cursor, claude-code, codex, opencode, all, or none" >&2
        exit 2
        ;;
esac
