# RZN DataSourcer - Packaging & Distribution

This directory contains all packaging and distribution configurations for RZN DataSourcer.

## Structure

```
packaging/
├── homebrew/           # Homebrew formula
├── docker/            # Docker configurations
├── scripts/           # Build and installation scripts
├── debian/            # Debian package configuration (future)
├── rpm/               # RPM package configuration (future)
└── snap/              # Snap package configuration (future)
```

## Build Scripts

### macOS Build (Current)
```bash
cargo build --release --package rzn_datasourcer_cli
```

Currently focused on macOS distribution. Cross-compilation scripts are available for future use:
- `./packaging/scripts/build-all.sh` - Docker-based cross-compilation (requires Docker)
- `./packaging/scripts/build-all-zigbuild.sh` - Zigbuild-based cross-compilation (containerless)

### One-line Installation (Future)
```bash
curl -fsSL https://raw.githubusercontent.com/srv1n/rzn_datasourcer/main/packaging/scripts/install.sh | bash
```

## Platform Support

### Tier 1 Platforms (Current Focus)
- **macOS**: 10.15+ (Intel), 11.0+ (Apple Silicon)

### Future Platforms (Infrastructure Ready)
- **Linux x86_64**: Ubuntu 20.04+, Debian 11+, Fedora 35+, Arch Linux
- **Windows**: Windows 10+, Windows Server 2019+
- **Linux ARM64**: Ubuntu 20.04+ on ARM64, Raspberry Pi OS 64-bit
- **Linux ARMv7**: Raspberry Pi OS 32-bit, Debian ARM

## Package Managers

### Homebrew (macOS)
```bash
# Add tap
brew tap srv1n/tap

# Install
brew install rzn-datasourcer
```

### Cargo (All Platforms)
```bash
cargo install rzn-datasourcer-cli
```

### Future Package Managers
- **AUR (Arch)**: `rzn-datasourcer-bin`
- **Snap (Linux)**: `rzn-datasourcer`
- **Scoop (Windows)**: `rzn-datasourcer`
- **Chocolatey (Windows)**: `rzn-datasourcer`

## Release Process

### 1. Prepare Release
```bash
# Update version in Cargo.toml files
# Update CHANGELOG.md
# Commit changes
git commit -m "chore: prepare release v0.1.0"
```

### 2. Create Tag
```bash
git tag v0.1.0
git push origin v0.1.0
```

### 3. GitHub Actions (Future)
The release workflow will automatically:
- Build for macOS platforms
- Create GitHub release
- Upload binaries
- Generate checksums

### 4. Update Package Managers
```bash
# Update Homebrew formula
./packaging/scripts/update-homebrew.sh v0.1.0

# Update AUR package
./packaging/scripts/update-aur.sh v0.1.0
```

## Manual Building

### Prerequisites
- Rust 1.70+
- Docker (for cross-compilation)
- Cross tool: `cargo install cross --git https://github.com/cross-rs/cross`

### Build for Current Platform
```bash
cargo build --release --package rzn_datasourcer_cli
```

### Cross-Compile for Linux (from any platform)
```bash
cross build --release --target x86_64-unknown-linux-gnu --package rzn_datasourcer_cli
```

### Cross-Compile for Windows (from any platform)
```bash
cross build --release --target x86_64-pc-windows-gnu --package rzn_datasourcer_cli
```

### Build for macOS (native only)
```bash
# Add targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin

# Build
cargo build --release --target x86_64-apple-darwin --package rzn_datasourcer_cli
cargo build --release --target aarch64-apple-darwin --package rzn_datasourcer_cli
```

## Distribution Channels

### Cargo/crates.io (Current)
- Primary distribution method
- Source distribution
- `cargo install rzn-datasourcer-cli`

### Homebrew (Future)
- macOS users
- Formula in `packaging/homebrew/`
- Manual update required after release

### GitHub Releases (Future)
- Pre-built binaries for macOS
- Manual or automatic via GitHub Actions

### Future Package Managers
- **AUR (Arch)**: `rzn-datasourcer-bin`
- **Snap (Linux)**: `rzn-datasourcer`
- **Scoop (Windows)**: `rzn-datasourcer`
- **Chocolatey (Windows)**: `rzn-datasourcer`

## Binary Verification

### Checksums
All releases include SHA256 checksums:
```bash
# Download checksums.txt from GitHub releases
wget https://github.com/srv1n/rzn_datasourcer/releases/latest/download/checksums.txt

# Verify binary
sha256sum -c checksums.txt
```

### GPG Signatures *(Future)*
Planned for future releases:
```bash
# Download signature
wget https://github.com/srv1n/rzn_datasourcer/releases/latest/download/rzn-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sig

# Verify
gpg --verify rzn-v0.1.0-x86_64-unknown-linux-gnu.tar.gz.sig
```

## Installation Paths

### System-wide Installation
```bash
# Linux/macOS
/usr/local/bin/rzn

# Windows
C:\Program Files\rzn\rzn.exe
```

### User Installation
```bash
# Linux/macOS
~/.local/bin/rzn

# Windows
%USERPROFILE%\bin\rzn.exe
```

## Troubleshooting

### Build Issues
```bash
# Clean build
cargo clean
rm -rf target/

# Update Cross
cargo install --force cross --git https://github.com/cross-rs/cross

# Docker issues
docker system prune
```

### Installation Issues
```bash
# Check binary
file /usr/local/bin/rzn

# Check permissions
ls -la /usr/local/bin/rzn

# Check PATH
echo $PATH | grep -o '/usr/local/bin'
```

### Platform-Specific Issues

#### Linux: Missing libraries
```bash
# Install required libraries
sudo apt install libc6-dev  # Ubuntu/Debian
sudo dnf install glibc-devel  # Fedora
```

#### macOS: Code signing
```bash
# Remove quarantine attribute
xattr -d com.apple.quarantine /usr/local/bin/rzn
```

#### Windows: Antivirus
- Add binary to antivirus exceptions
- Use signed releases (future)

## Contributing

### Adding New Platform
1. Add target to `Cross.toml`
2. Update `build-all.sh`
3. Add to GitHub Actions workflow
4. Test on target platform
5. Update documentation

### Package Manager Integration
1. Create package configuration
2. Add to build pipeline
3. Document installation process
4. Set up automated updates

For more information, see the main [README.md](../README.md) and [CONTRIBUTING.md](../CONTRIBUTING.md).