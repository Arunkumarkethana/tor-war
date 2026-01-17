#!/bin/bash
set -e

# Configuration
APP_NAME="nipe"
VERSION="1.0.0"
ARCH="amd64"
BUILD_DIR="target/debian"

echo "Building Debian package for $APP_NAME v$VERSION..."

# Ensure release build is done
cargo build --release

# Prepare directory structure
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/DEBIAN"
mkdir -p "$BUILD_DIR/usr/local/bin"
mkdir -p "$BUILD_DIR/var/lib/nipe"
mkdir -p "$BUILD_DIR/etc/nipe"

# Copy files
cp packaging/debian/control "$BUILD_DIR/DEBIAN/"
cp target/release/nipe "$BUILD_DIR/usr/local/bin/"

# Set permissions
chmod 755 "$BUILD_DIR/usr/local/bin/nipe"
chmod 700 "$BUILD_DIR/var/lib/nipe"

# Build package
dpkg-deb --build "$BUILD_DIR" "target/${APP_NAME}_${VERSION}_${ARCH}.deb"

echo "Package created at target/${APP_NAME}_${VERSION}_${ARCH}.deb"
