#!/usr/bin/env bash
set -e

REPO="vizier-lab/vizier"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

info() { echo "→ $1"; }
error() { echo "✗ $1" >&2; exit 1; }

usage() {
    cat <<EOF
Usage: install.sh [options]

Options:
  -v, --version <version>   Install a specific version (e.g. 0.13.0 or v0.13.0-rc.1).
                            Defaults to the latest stable release.
  -d, --install-dir <dir>   Install directory (default: \$HOME/.local/bin).
  -h, --help                Show this help.

Environment variables VERSION and INSTALL_DIR are also honoured; flags take precedence.

Example:
  curl -fsSL https://raw.githubusercontent.com/$REPO/master/install.sh | bash -s -- --version 0.13.0
EOF
}

# Parse arguments
while [ $# -gt 0 ]; do
    case "$1" in
        -v|--version)
            [ -n "$2" ] || error "$1 requires a value"
            VERSION="$2"; shift 2;;
        --version=*) VERSION="${1#*=}"; shift;;
        -d|--install-dir)
            [ -n "$2" ] || error "$1 requires a value"
            INSTALL_DIR="$2"; shift 2;;
        --install-dir=*) INSTALL_DIR="${1#*=}"; shift;;
        -h|--help) usage; exit 0;;
        *) usage >&2; error "Unknown option: $1";;
    esac
done

# Detect platform
case "$(uname -s)" in
    Linux*) OS="linux";;
    Darwin*) OS="macos";;
    *) error "Unsupported OS: $(uname -s)";;
esac

case "$(uname -m)" in
    x86_64|amd64) ARCH="x86_64";;
    arm64|aarch64) ARCH="aarch64";;
    *) error "Unsupported architecture: $(uname -m)";;
esac

case "$OS" in
    linux) TARGET="${ARCH}-unknown-linux-gnu";;
    macos) TARGET="${ARCH}-apple-darwin";;
esac

# Get version (stable release only, no prereleases)
get_latest_version() {
    local releases_url="https://api.github.com/repos/$REPO/releases"
    curl -s "$releases_url" | awk -F'"' '/"tag_name":/ && !/-/ {gsub(/^v/, "", $4); print $4; exit}'
}
VERSION="${VERSION:-$(get_latest_version)}"
[ -z "$VERSION" ] && error "Failed to get latest version"
VERSION="${VERSION#v}"

info "Installing Vizier v$VERSION for $TARGET"

# Create install dir
mkdir -p "$INSTALL_DIR"

# Download
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT
cd "$TMP_DIR"
curl -fsSL "https://github.com/$REPO/releases/download/v$VERSION/vizier-v$VERSION-$TARGET.tar.gz" -o vizier.tar.gz || error "Download failed. Version v$VERSION for $TARGET may not exist."
tar -xzf vizier.tar.gz
mv "vizier-v$VERSION-$TARGET/vizier" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/vizier"

# Add to PATH
add_to_path() {
    local shell_rc=""
    case "$SHELL" in
        */zsh) shell_rc="$HOME/.zshrc";;
        */bash) shell_rc="$HOME/.bashrc";;
        */fish) shell_rc="$HOME/.config/fish/config.fish";;
    esac

    if [ -n "$shell_rc" ] && ! grep -q "export PATH=.*$INSTALL_DIR" "$shell_rc" 2>/dev/null; then
        echo "export PATH=\"$INSTALL_DIR:\$PATH\"" >> "$shell_rc"
        info "Added $INSTALL_DIR to PATH in $shell_rc"
        case "$SHELL" in
            */fish) info "Run: exec fish or restart terminal";;
            *) info "Run: source $shell_rc";;
        esac
    fi
}

# Check if in PATH
case ":$PATH:" in
    *":$INSTALL_DIR:"*) ;;
    *) add_to_path;;
esac

info "Vizier installed to $INSTALL_DIR/vizier"
"$INSTALL_DIR/vizier" --version 2>/dev/null || true
