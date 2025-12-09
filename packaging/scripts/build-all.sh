#!/bin/bash
# Build script for cross-compiling RZN DataSourcer to multiple targets

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
PACKAGE_NAME="rzn-datasourcer"
VERSION=${VERSION:-$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')}
BUILD_DIR="target/releases"
WORKSPACE_ROOT=$(pwd)

# Supported targets
declare -a TARGETS=(
    # Linux x86_64
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    
    # Linux ARM64  
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    
    # Windows
    "x86_64-pc-windows-gnu"
    
    # ARM (Raspberry Pi, etc.)
    "armv7-unknown-linux-gnueabihf"
)

# macOS targets (requires special handling)
declare -a MACOS_TARGETS=(
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
)

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

# Check prerequisites
check_prerequisites() {
    print_header "Checking Prerequisites"
    
    if ! command -v cross &> /dev/null; then
        print_error "cross is not installed. Install with: cargo install cross --git https://github.com/cross-rs/cross"
        exit 1
    fi
    
    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not running"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        print_error "Docker daemon is not running"
        exit 1
    fi
    
    if ! command -v jq &> /dev/null; then
        print_warning "jq not found. Install for automatic version detection"
    fi
    
    print_success "Prerequisites check passed"
}

# Create build directory
prepare_build_dir() {
    print_header "Preparing Build Directory"
    
    rm -rf "${BUILD_DIR}"
    mkdir -p "${BUILD_DIR}"
    
    print_success "Build directory created: ${BUILD_DIR}"
}

# Build for a specific target
build_target() {
    local target=$1
    local binary_name="rzn"
    
    if [[ "$target" == *"windows"* ]]; then
        binary_name="rzn.exe"
    fi
    
    print_header "Building for ${target}"
    
    # Build with cross
    if cross build --release --target "${target}" --package rzn_datasourcer_cli; then
        print_success "Built successfully for ${target}"
        
        # Copy binary to releases directory
        local target_dir="${BUILD_DIR}/${target}"
        mkdir -p "${target_dir}"
        
        cp "target/${target}/release/${binary_name}" "${target_dir}/"
        
        # Create archive
        create_archive "${target}" "${target_dir}"
        
    else
        print_error "Failed to build for ${target}"
        return 1
    fi
}

# Build for macOS (native only)
build_macos() {
    print_header "Building for macOS (native)"
    
    # Check if we're on macOS
    if [[ "$OSTYPE" != "darwin"* ]]; then
        print_warning "Skipping macOS builds (not on macOS)"
        return 0
    fi
    
    # Add macOS targets
    for target in "${MACOS_TARGETS[@]}"; do
        print_header "Building for ${target}"
        
        # Add target if not already added
        rustup target add "${target}" 2>/dev/null || true
        
        if cargo build --release --target "${target}" --package rzn_datasourcer_cli; then
            print_success "Built successfully for ${target}"
            
            # Copy binary to releases directory
            local target_dir="${BUILD_DIR}/${target}"
            mkdir -p "${target_dir}"
            
            cp "target/${target}/release/rzn" "${target_dir}/"
            
            # Create archive
            create_archive "${target}" "${target_dir}"
            
        else
            print_error "Failed to build for ${target}"
        fi
    done
}

# Create compressed archive
create_archive() {
    local target=$1
    local target_dir=$2
    local archive_name="${PACKAGE_NAME}-${VERSION}-${target}"
    
    print_header "Creating archive for ${target}"
    
    cd "${target_dir}"
    
    if [[ "$target" == *"windows"* ]]; then
        # Create ZIP for Windows
        zip -q "${archive_name}.zip" rzn.exe
        mv "${archive_name}.zip" "../"
        print_success "Created ${archive_name}.zip"
    else
        # Create tar.gz for Unix-like systems
        tar -czf "${archive_name}.tar.gz" rzn
        mv "${archive_name}.tar.gz" "../"
        print_success "Created ${archive_name}.tar.gz"
    fi
    
    cd "${WORKSPACE_ROOT}"
}

# Generate checksums
generate_checksums() {
    print_header "Generating Checksums"
    
    cd "${BUILD_DIR}"
    
    # Generate SHA256 checksums
    if command -v sha256sum &> /dev/null; then
        sha256sum *.{tar.gz,zip} 2>/dev/null > checksums.txt || true
    elif command -v shasum &> /dev/null; then
        shasum -a 256 *.{tar.gz,zip} 2>/dev/null > checksums.txt || true
    else
        print_warning "No SHA256 utility found"
    fi
    
    cd "${WORKSPACE_ROOT}"
    
    if [[ -f "${BUILD_DIR}/checksums.txt" ]]; then
        print_success "Generated checksums.txt"
    fi
}

# List build artifacts
list_artifacts() {
    print_header "Build Artifacts"
    
    echo "Location: ${BUILD_DIR}"
    echo ""
    
    if [[ -d "${BUILD_DIR}" ]]; then
        ls -lh "${BUILD_DIR}"/*.{tar.gz,zip} 2>/dev/null || echo "No archives found"
        
        if [[ -f "${BUILD_DIR}/checksums.txt" ]]; then
            echo ""
            echo "Checksums:"
            cat "${BUILD_DIR}/checksums.txt"
        fi
    fi
}

# Main execution
main() {
    print_header "Cross-Compiling RZN DataSourcer v${VERSION}"
    
    check_prerequisites
    prepare_build_dir
    
    # Build for Linux and Windows targets
    for target in "${TARGETS[@]}"; do
        build_target "${target}" || print_warning "Skipping ${target} due to build failure"
    done
    
    # Build for macOS (if on macOS)
    build_macos
    
    generate_checksums
    list_artifacts
    
    print_success "Cross-compilation completed!"
    print_header "Next Steps"
    echo "1. Test binaries on target platforms"
    echo "2. Upload artifacts to GitHub Releases"
    echo "3. Update Homebrew formula with new checksums"
    echo "4. Update package manager configurations"
}

# Script execution
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi