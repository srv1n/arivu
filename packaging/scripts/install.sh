#!/bin/bash
# Universal installer script for Arivu

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
REPO="srv1n/arivu"
BINARY_NAME="arivu"
# Default to ~/.local/bin (no sudo required), fallback to /usr/local/bin
DEFAULT_INSTALL_DIR="${HOME}/.local/bin"
INSTALL_DIR="${INSTALL_DIR:-$DEFAULT_INSTALL_DIR}"
TEMP_DIR="/tmp/arivu-install"

print_header() {
    echo -e "${BLUE}=== $1 ===${NC}"
}

print_success() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

print_error() {
    echo -e "${RED}✗ $1${NC}"
}

# Detect system architecture and OS
detect_platform() {
    local os arch target

    # Detect OS
    case "$OSTYPE" in
        linux*)   os="unknown-linux-gnu" ;;
        darwin*)  os="apple-darwin" ;;
        msys*|cygwin*|win32*) os="pc-windows-msvc" ;;
        *)        print_error "Unsupported OS: $OSTYPE"; exit 1 ;;
    esac

    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64) arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        armv7l) arch="armv7"; os="unknown-linux-gnueabihf" ;;
        *) print_error "Unsupported architecture: $(uname -m)"; exit 1 ;;
    esac

    target="${arch}-${os}"
    echo "$target"
}

# Get latest release version
get_latest_version() {
    local version
    if command -v curl >/dev/null 2>&1; then
        version=$(curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    elif command -v wget >/dev/null 2>&1; then
        version=$(wget -qO- "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name"' | cut -d'"' -f4)
    else
        print_error "Neither curl nor wget found. Please install one of them."
        exit 1
    fi

    if [[ -z "$version" ]]; then
        print_error "Failed to get latest version"
        exit 1
    fi

    echo "$version"
}

# Download and extract binary
download_and_install() {
    local target=$1
    local version=$2
    local download_url archive_name

    # Determine archive format and URL
    if [[ "$target" == *"windows"* ]]; then
        archive_name="arivu-${version}-${target}.zip"
    else
        archive_name="arivu-${version}-${target}.tar.gz"
    fi

    download_url="https://github.com/${REPO}/releases/download/${version}/${archive_name}"

    print_header "Downloading Arivu ${version} for ${target}"

    # Create temp directory
    rm -rf "$TEMP_DIR"
    mkdir -p "$TEMP_DIR"
    cd "$TEMP_DIR"

    # Download
    if command -v curl >/dev/null 2>&1; then
        if ! curl -fsSL -o "$archive_name" "$download_url"; then
            print_error "Failed to download from $download_url"
            exit 1
        fi
    elif command -v wget >/dev/null 2>&1; then
        if ! wget -q -O "$archive_name" "$download_url"; then
            print_error "Failed to download from $download_url"
            exit 1
        fi
    fi

    print_success "Downloaded $archive_name"

    # Extract
    if [[ "$target" == *"windows"* ]]; then
        if command -v unzip >/dev/null 2>&1; then
            unzip -q "$archive_name"
        else
            print_error "unzip not found. Please install unzip."
            exit 1
        fi
    else
        tar -xzf "$archive_name"
    fi

    print_success "Extracted archive"

    # Install binary
    local binary_name="$BINARY_NAME"
    if [[ "$target" == *"windows"* ]]; then
        binary_name="${BINARY_NAME}.exe"
    fi

    if [[ ! -f "$binary_name" ]]; then
        print_error "Binary $binary_name not found in archive"
        exit 1
    fi

    # Create install directory if it doesn't exist
    if [[ ! -d "$INSTALL_DIR" ]]; then
        mkdir -p "$INSTALL_DIR"
    fi

    # Check if we need sudo
    if [[ -w "$INSTALL_DIR" ]]; then
        cp "$binary_name" "$INSTALL_DIR/"
        chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    else
        print_header "Installing to $INSTALL_DIR (requires sudo)"
        sudo cp "$binary_name" "$INSTALL_DIR/"
        sudo chmod +x "${INSTALL_DIR}/${BINARY_NAME}"
    fi

    print_success "Installed to ${INSTALL_DIR}/${BINARY_NAME}"
}

# Verify installation
verify_installation() {
    print_header "Verifying Installation"

    if command -v "$BINARY_NAME" >/dev/null 2>&1; then
        local version
        version=$("$BINARY_NAME" --version 2>/dev/null || echo "unknown")
        print_success "Arivu installed successfully: $version"
    else
        print_warning "Binary installed but not in PATH. Add $INSTALL_DIR to your PATH."
        echo "Add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
        echo "export PATH=\"$INSTALL_DIR:\$PATH\""
    fi
}

# Show post-install information
show_post_install() {
    print_header "Getting Started"

    echo "First-time setup:"
    echo "  $BINARY_NAME setup                             # Interactive configuration wizard"
    echo ""
    echo "Quick start commands:"
    echo "  $BINARY_NAME list                              # List available connectors"
    echo "  $BINARY_NAME search youtube 'rust tutorial'    # Search YouTube videos"
    echo "  $BINARY_NAME get youtube dQw4w9WgXcQ           # Get video with transcript"
    echo "  $BINARY_NAME tools slack                       # Show Slack connector tools"
    echo ""
    echo "Configure authenticated connectors:"
    echo "  $BINARY_NAME setup slack                       # Interactive Slack setup"
    echo "  $BINARY_NAME config set github --value <token> # Set credentials"
    echo ""
    echo "Documentation:"
    echo "  https://github.com/${REPO}/blob/main/INSTALLATION.md"
    echo ""
    echo "Get help:"
    echo "  $BINARY_NAME --help"
}

# Cleanup
cleanup() {
    rm -rf "$TEMP_DIR"
}

# Main installation function
main() {
    print_header "Arivu Installer"

    # Check prerequisites
    if [[ $EUID -eq 0 ]] && [[ "$INSTALL_DIR" == "/usr/local/bin" ]]; then
        print_warning "Running as root. Consider using a user-specific install directory."
    fi

    # Detect platform
    local target
    target=$(detect_platform)
    print_success "Detected platform: $target"

    # Get latest version
    local version
    version=$(get_latest_version)
    print_success "Latest version: $version"

    # Download and install
    download_and_install "$target" "$version"

    # Verify
    verify_installation

    # Show getting started info
    show_post_install

    # Cleanup
    cleanup

    print_success "Installation completed!"
}

# Handle command line arguments
case "${1:-}" in
    --help|-h)
        echo "Arivu Installer"
        echo ""
        echo "Usage: $0 [options]"
        echo ""
        echo "Options:"
        echo "  --help, -h     Show this help message"
        echo "  --version, -v  Show version information"
        echo ""
        echo "Environment variables:"
        echo "  INSTALL_DIR    Installation directory (default: ~/.local/bin)"
        echo ""
        echo "Examples:"
        echo "  $0                                # Install to ~/.local/bin"
        echo "  INSTALL_DIR=/usr/local/bin $0    # Install system-wide (requires sudo)"
        exit 0
        ;;
    --version|-v)
        echo "Arivu Installer v1.0.0"
        exit 0
        ;;
esac

# Set custom install directory if provided
if [[ -n "${INSTALL_DIR:-}" ]]; then
    INSTALL_DIR="$INSTALL_DIR"
fi

# Run main function
trap cleanup EXIT
main "$@"
