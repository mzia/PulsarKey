#!/usr/bin/env bash
# ==============================================================================
# 🌌 PulsarKey macOS Application Bundle & DMG/PKG Packaging Script
# Builds PulsarKey.app, .dmg disk image, and .pkg installer for macOS
# ==============================================================================

set -euo pipefail

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d '"' -f2)}"
APP_NAME="PulsarKey.app"
BUILD_DIR="dist/macos"
APP_BUNDLE="${BUILD_DIR}/${APP_NAME}"
DMG_NAME="PulsarKey-${VERSION}.dmg"
PKG_NAME="PulsarKey-${VERSION}.pkg"

mkdir -p dist

echo "🍎 Building PulsarKey for macOS (v${VERSION})..."

# 1. Build release binaries
echo "🔨 Compiling release binaries..."
cargo build --release --bin pulsarkey --bin pulsarkey-settings

# 2. Assemble App Bundle Structure
echo "📦 Assembling ${APP_NAME} bundle..."
rm -rf "${BUILD_DIR}"
mkdir -p "${APP_BUNDLE}/Contents/MacOS"
mkdir -p "${APP_BUNDLE}/Contents/Resources"

# Copy executables
cp target/release/pulsarkey "${APP_BUNDLE}/Contents/MacOS/pulsarkey"
cp target/release/pulsarkey-settings "${APP_BUNDLE}/Contents/MacOS/pulsarkey-settings"
chmod +x "${APP_BUNDLE}/Contents/MacOS/pulsarkey"
chmod +x "${APP_BUNDLE}/Contents/MacOS/pulsarkey-settings"

# Copy Info.plist with dynamic version substitution
sed -E "s/<string>[0-9]+\.[0-9]+\.[0-9]+<\/string>/<string>${VERSION}<\/string>/g" packaging/macos/Info.plist > "${APP_BUNDLE}/Contents/Info.plist"

# Copy icon if available
if [ -f "packaging/cosmic-fido2.svg" ]; then
    cp packaging/cosmic-fido2.svg "${APP_BUNDLE}/Contents/Resources/AppIcon.svg"
fi

echo "✅ App bundle assembled at ${APP_BUNDLE}"

# 3. If running natively on macOS, generate DMG and PKG
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "💿 Creating macOS DMG disk image (${DMG_NAME})..."
    DMG_STAGING="${BUILD_DIR}/dmg_staging"
    mkdir -p "${DMG_STAGING}"
    cp -R "${APP_BUNDLE}" "${DMG_STAGING}/"
    ln -s /Applications "${DMG_STAGING}/Applications"

    hdiutil create -volname "PulsarKey ${VERSION}" \
                   -srcfolder "${DMG_STAGING}" \
                   -ov -format UDZO \
                   "dist/${DMG_NAME}"
    rm -rf "${DMG_STAGING}"
    echo "🎉 Created dist/${DMG_NAME}"

    if which pkgbuild >/dev/null 2>&1; then
        echo "📦 Creating macOS PKG installer (${PKG_NAME})..."
        pkgbuild --component "${APP_BUNDLE}" \
                 --install-location "/Applications" \
                 --identifier "io.github.mzia.PulsarKey" \
                 --version "${VERSION}" \
                 "dist/${PKG_NAME}"
        echo "🎉 Created dist/${PKG_NAME}"
    fi
else
    echo "ℹ️ Note: Creating a native .dmg and .pkg requires macOS tools ('hdiutil' and 'pkgbuild')."
    echo "Creating compressed distribution tarball for macOS..."
    tar -czf "dist/PulsarKey-macOS-${VERSION}.tar.gz" -C "${BUILD_DIR}" "${APP_NAME}"
    echo "🎉 Created dist/PulsarKey-macOS-${VERSION}.tar.gz"
fi
