# Build and install to ~/.cargo/bin

default:
    @just --list

# Build in debug mode
build:
    cargo build
    cd tap && swift build

# Build in release mode
release:
    cargo build --release
    cd tap && swift build -c release

# Run in debug mode
run *ARGS:
    cargo run -- {{ARGS}}

# Run in release mode
run-release *ARGS:
    cargo run --release -- {{ARGS}}

# Run clippy lints
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Check formatting without modifying
fmt-check:
    cargo fmt --check

# Run tests
test:
    cargo test

# Build and install both binaries to ~/.cargo/bin
install:
    cargo install --path .
    cd tap && swift build -c release
    cp tap/.build/release/specterm-tap ~/.cargo/bin/

# Build an ad-hoc-signed DMG for local release-pipeline validation
package version:
    ./scripts/package-release.sh {{version}}

# Uninstall
uninstall:
    cargo uninstall specterm
    rm -f ~/.cargo/bin/specterm-tap

# Clean build artifacts
clean:
    cargo clean
    cd tap && swift package clean
