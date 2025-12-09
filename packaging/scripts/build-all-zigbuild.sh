#!/bin/bash
# Build script for cross-compiling RZN DataSourcer using zigbuild

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

# Supported targets (zigbuild compatible)
declare -a TARGETS=(
    # Linux x86_64
    "x86_64-unknown-linux-gnu"
    "x86_64-unknown-linux-musl"
    
    # Linux ARM64  
    "aarch64-unknown-linux-gnu"
    "aarch64-unknown-linux-musl"
    
    # Windows
    "x86_64-pc-windows-gnu"
    
    # macOS (if on macOS)
    "x86_64-apple-darwin"
    "aarch64-apple-darwin"
    
    # ARM (Raspberry Pi, etc.)
    "armv7-unknown-linux-gnueabihf"
    "armv7-unknown-linux-musleabihf"
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
    
    if ! command -v cargo-zigbuild &> /dev/null; then
        print_error "cargo-zigbuild is not installed. Install with: cargo install cargo-zigbuild"
        exit 1
    fi
    
    if ! command -v zig &> /dev/null; then
        print_warning "zig not found. cargo-zigbuild will download it automatically"
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
    
    # Add the target if not already added
    rustup target add "${target}" 2>/dev/null || true
    
    # Build with zigbuild
    if cargo zigbuild --release --target "${target}" --package rzn_datasourcer_cli; then
        print_success "Built successfully for ${target}"
        
        # Copy binary to releases directory
        local target_dir="${BUILD_DIR}/${target}"
        mkdir -p "${target_dir}"
        
        cp "target/${target}/release/${binary_name}" "${target_dir}/"
        
        # Strip debug symbols for smaller binaries (except Windows)
        if [[ "$target" != *"windows"* ]] && command -v strip &> /dev/null; then
            strip "${target_dir}/${binary_name}" 2>/dev/null || true
        fi
        
        # Create archive
        create_archive "${target}" "${target_dir}"
        
    else
        print_error "Failed to build for ${target}"
        return 1
    fi
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
        if command -v zip &> /dev/null; then
            zip -q "${archive_name}.zip" rzn.exe
            mv "${archive_name}.zip" "../"
            print_success "Created ${archive_name}.zip"
        else
            print_warning "zip not found, creating tar.gz instead"
            tar -czf "${archive_name}.tar.gz" rzn.exe
            mv "${archive_name}.tar.gz" "../"
        fi
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

# Test that the binary works (for native platform)
test_binary() {
    local native_target
    
    case "$OSTYPE" in
        linux*)   
            case "$(uname -m)" in
                x86_64) native_target="x86_64-unknown-linux-gnu" ;;
                aarch64) native_target="aarch64-unknown-linux-gnu" ;;
                *) return 0 ;;
            esac
            ;;
        darwin*)  
            case "$(uname -m)" in
                x86_64) native_target="x86_64-apple-darwin" ;;
                arm64) native_target="aarch64-apple-darwin" ;;
                *) return 0 ;;
            esac
            ;;
        *) return 0 ;;
    esac
    
    local test_binary="${BUILD_DIR}/${native_target}/rzn"
    
    if [[ -f "$test_binary" ]]; then
        print_header "Testing native binary"
        
        if "$test_binary" --version &> /dev/null; then
            print_success "Binary test passed: $("$test_binary" --version)"
        else
            print_warning "Binary test failed"
        fi
    fi
}

# Main execution
main() {
    print_header "Cross-Compiling RZN DataSourcer v${VERSION} with zigbuild"
    
    check_prerequisites
    prepare_build_dir
    
    # Build for all targets
    for target in "${TARGETS[@]}"; do
        # Skip macOS targets if not on macOS
        if [[ "$target" == *"darwin"* ]] && [[ "$OSTYPE" != "darwin"* ]]; then
            print_warning "Skipping ${target} (requires macOS host)"
            continue
        fi
        
        build_target "${target}" || print_warning "Skipping ${target} due to build failure"
    done
    
    generate_checksums
    test_binary
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