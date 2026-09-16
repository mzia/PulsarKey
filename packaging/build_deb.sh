#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-$(grep -m1 '^version' Cargo.toml | cut -d '"' -f2)}"
ARCH="amd64"
PKG_DIR="packaging/deb/pulsarkey_${VERSION}_${ARCH}"

echo "🔨 Building PulsarKey .deb package (v${VERSION}-${ARCH})..."

# Ensure release binaries are built
cargo build --release

# Clean & create staging tree
rm -rf "${PKG_DIR}"
mkdir -p "${PKG_DIR}/DEBIAN" \
         "${PKG_DIR}/usr/bin" \
         "${PKG_DIR}/usr/share/applications" \
         "${PKG_DIR}/usr/share/icons/hicolor/scalable/apps" \
         "${PKG_DIR}/usr/share/metainfo" \
         "${PKG_DIR}/usr/lib/systemd/user" \
         "${PKG_DIR}/etc/xdg/autostart"

# Binaries
cp target/release/pulsarkey "${PKG_DIR}/usr/bin/pulsarkey"
chmod 755 "${PKG_DIR}/usr/bin/pulsarkey"
ln -sf pulsarkey "${PKG_DIR}/usr/bin/cosmic-fido2"
cp target/release/pulsarkey-settings "${PKG_DIR}/usr/bin/pulsarkey-settings"
chmod 755 "${PKG_DIR}/usr/bin/pulsarkey-settings"

# Icons
cp packaging/cosmic-fido2.svg "${PKG_DIR}/usr/share/icons/hicolor/scalable/apps/io.github.mzia.PulsarKey.svg"
ln -sf io.github.mzia.PulsarKey.svg "${PKG_DIR}/usr/share/icons/hicolor/scalable/apps/com.system76.cosmic-fido2.svg"

# Desktop entries
cat << 'EOF' > "${PKG_DIR}/usr/share/applications/io.github.mzia.PulsarKey.desktop"
[Desktop Entry]
Name=PulsarKey Status
GenericName=FIDO2 Security Key Status
Comment=Hardware-backed FIDO2 & Biometric Authentication Manager for Pop!_OS COSMIC
Exec=cosmic-term -e bash -c "pulsarkey status; echo ''; read -p 'Press Enter to exit...'"
Icon=io.github.mzia.PulsarKey
Terminal=false
Type=Application
Categories=Settings;System;Security;Utility;
Keywords=pulsar;yubikey;fido2;u2f;security;biometric;cosmic;pam;greeter;
StartupNotify=true
EOF

cat << 'EOF' > "${PKG_DIR}/usr/share/applications/io.github.mzia.PulsarKey.Settings.desktop"
[Desktop Entry]
Name=PulsarKey Settings
GenericName=Security & Hardware Control Panel
Comment=COSMIC Native Settings Control Panel for YubiKey FIDO2 & Biometrics
Exec=/usr/bin/pulsarkey-settings
Icon=io.github.mzia.PulsarKey
Terminal=false
Type=Application
Categories=Settings;System;Security;Utility;COSMIC;
Keywords=pulsar;yubikey;fido2;u2f;security;biometric;cosmic;settings;profiles;rescue;
StartupNotify=true
EOF

cat << 'EOF' > "${PKG_DIR}/usr/share/applications/io.github.mzia.PulsarKey.Applet.desktop"
[Desktop Entry]
Name=PulsarKey Security Applet
Comment=COSMIC Panel Status Applet for YubiKey FIDO2
Exec=/usr/bin/pulsarkey applet
Icon=io.github.mzia.PulsarKey
Terminal=false
Type=Application
Categories=COSMIC;Utility;Security;
X-CosmicApplet=true
X-GNOME-Autostart-enabled=true
NoDisplay=true
EOF

cp "${PKG_DIR}/usr/share/applications/io.github.mzia.PulsarKey.Applet.desktop" \
   "${PKG_DIR}/etc/xdg/autostart/io.github.mzia.PulsarKey.Applet.desktop"

# Systemd user service
cat << 'EOF' > "${PKG_DIR}/usr/lib/systemd/user/pulsarkey-applet.service"
[Unit]
Description=PulsarKey FIDO2 & Biometric Status Applet
PartOf=graphical-session.target
After=graphical-session.target

[Service]
ExecStart=/usr/bin/pulsarkey applet
Restart=on-failure
RestartSec=3

[Install]
WantedBy=graphical-session.target
EOF

# AppStream Metainfo
cat << 'EOF' > "${PKG_DIR}/usr/share/metainfo/io.github.mzia.PulsarKey.metainfo.xml"
<?xml version="1.0" encoding="UTF-8"?>
<component type="desktop-application">
  <id>io.github.mzia.PulsarKey</id>
  <metadata_license>CC0-1.0</metadata_license>
  <project_license>MIT</project_license>
  <name>PulsarKey</name>
  <summary>Hardware-backed FIDO2 &amp; Biometric Authentication Manager for COSMIC</summary>
  <developer id="io.github.mzia">
    <name>M. Zia</name>
  </developer>
  <description>
    <p>
      PulsarKey is a native Rust security suite designed specifically for the Pop!_OS COSMIC desktop environment.
      Named after celestial neutron stars that pulse across the cosmos, PulsarKey connects the rhythmic pulsing LED of your FIDO2 key (YubiKey C Bio) with zero-lag desktop lockscreen unlocking, biometric sudo authentication, and real-time top panel monitoring.
    </p>
    <p>Key Capabilities:</p>
    <ul>
      <li>Native COSMIC Panel Applet: Real-time hardware monitoring via D-Bus StatusNotifierItem</li>
      <li>Zero-Lag Lockscreen: Seamless biometric unlock with Space+Enter activation to eliminate greeter stalls</li>
      <li>Passwordless Sudo: Instant biometric or touch authorization for administrative commands</li>
      <li>Sensor Health Check: One-click biometric sensor diagnostics and desktop notifications</li>
      <li>Automated Safety: Guided enrollment with instant rollback to password-only authentication</li>
    </ul>
  </description>
  <launchable type="desktop-id">io.github.mzia.PulsarKey.desktop</launchable>
  <icon type="stock">io.github.mzia.PulsarKey</icon>
  <url type="homepage">https://github.com/mzia/PulsarKey</url>
  <url type="bugtracker">https://github.com/mzia/PulsarKey/issues</url>
  <categories>
    <category>Utility</category>
    <category>System</category>
    <category>Security</category>
  </categories>
  <keywords>
    <keyword>pulsarkey</keyword>
    <keyword>yubikey</keyword>
    <keyword>fido2</keyword>
    <keyword>u2f</keyword>
    <keyword>biometric</keyword>
    <keyword>cosmic</keyword>
    <keyword>pam</keyword>
  </keywords>
  <provides>
    <binary>pulsarkey</binary>
    <binary>cosmic-fido2</binary>
    <binary>pulsarkey-settings</binary>
  </provides>
  <releases>
    <release version="1.4.0" date="2026-09-15">
      <description>
        <p>Introduces COSMIC Native Settings App ('pulsarkey-settings' / 'pulsarkey gui') with full graphical control panel and Emergency Paper Recovery Key &amp; Offline Rescue Runbook Suite ('pulsarkey rescue').</p>
      </description>
    </release>
    <release version="1.3.0" date="2026-09-15">
      <description>
        <p>Introduces Security Strictness Profiles (Convenience, Fortress 2FA, Lockdown), Authentication Audit Journal ('Recent Pulses'), and Backup Key Pairing &amp; Recovery Assistant.</p>
      </description>
    </release>
    <release version="1.2.0" date="2026-09-14">
      <description>
        <p>Introduces Native On-Key Biometric &amp; Fingerprint Manager (pulsarkey bio &amp; pulsarkey pin), instant sysfs hardware polling with zero-contention applet engine, and COSMIC panel biometric launcher.</p>
      </description>
    </release>
    <release version="1.1.0" date="2026-09-14">
      <description>
        <p>Adds Hardware-Backed SSH &amp; Git Signing Wizard (pulsarkey ssh-setup), Presence Sentinel auto-lock on key removal, and Polkit GUI authentication.</p>
      </description>
    </release>
    <release version="1.0.0" date="2026-09-14">
      <description>
        <p>Initial stable release of PulsarKey featuring full COSMIC Greeter biometric integration, PAM sudo authentication, and real-time StatusNotifierItem panel applet.</p>
      </description>
    </release>
  </releases>
  <content_rating type="oars-1.1" />
</component>
EOF

# DEBIAN Control File
cat << EOF > "${PKG_DIR}/DEBIAN/control"
Package: pulsarkey
Version: ${VERSION}
Section: admin
Priority: optional
Architecture: ${ARCH}
Depends: libc6 (>= 2.34), libpam-u2f, pamu2fcfg, yubikey-manager
Recommends: libnotify-bin, cosmic-term
Provides: cosmic-fido2 (= ${VERSION})
Conflicts: cosmic-fido2 (<< ${VERSION})
Replaces: cosmic-fido2 (<< ${VERSION})
Maintainer: M. Zia <mzia@pop-os.local>
Installed-Size: 7400
Homepage: https://github.com/mzia/PulsarKey
Description: Hardware-backed FIDO2 & Biometric Authentication Manager for COSMIC
 PulsarKey is a native Rust security utility providing seamless FIDO2
 and biometric (YubiKey C Bio) integration for Pop!_OS COSMIC.
 .
 Features:
  * Zero-delay lockscreen unlocking with COSMIC greeter
  * Passwordless / single-touch sudo authentication
  * Hardware-backed SSH key generation & Git commit signing
  * Presence Sentinel auto-lock on key removal
  * Polkit graphical elevation integration
  * Native COSMIC desktop panel StatusNotifierItem applet
  * Real-time USB hardware monitoring and biometric sensor testing
EOF

# DEBIAN postinst script
cat << 'EOF' > "${PKG_DIR}/DEBIAN/postinst"
#!/bin/sh
set -e

case "$1" in
    configure)
        if which update-desktop-database >/dev/null 2>&1; then
            update-desktop-database -q /usr/share/applications || true
        fi
        if which gtk-update-icon-cache >/dev/null 2>&1; then
            gtk-update-icon-cache -q -t -f /usr/share/icons/hicolor || true
        fi
        if which udevadm >/dev/null 2>&1; then
            udevadm control --reload-rules 2>/dev/null || true
            udevadm trigger 2>/dev/null || true
        fi
        echo "================================================================"
        echo " 🌌 PulsarKey installed successfully!"
        echo ""
        echo " Next steps:"
        echo "  - To configure your YubiKey for COSMIC greeter & sudo, run:"
        echo "      sudo pulsarkey setup"
        echo "  - To configure hardware SSH and Git commit signing, run:"
        echo "      pulsarkey ssh-setup"
        echo "  - The COSMIC Panel Applet will launch on your next login,"
        echo "    or start it immediately with:"
        echo "      pulsarkey applet &"
        echo "================================================================"
    ;;
esac
exit 0
EOF
chmod 755 "${PKG_DIR}/DEBIAN/postinst"

# DEBIAN prerm script
cat << 'EOF' > "${PKG_DIR}/DEBIAN/prerm"
#!/bin/sh
set -e
case "$1" in
    remove|purge)
        if [ -x /usr/bin/pulsarkey ]; then
            /usr/bin/pulsarkey uninstall || true
        fi
    ;;
esac
exit 0
EOF
chmod 755 "${PKG_DIR}/DEBIAN/prerm"

# Build package
mkdir -p dist packaging
dpkg-deb --build "${PKG_DIR}" "dist/pulsarkey_${VERSION}_${ARCH}.deb"
cp "dist/pulsarkey_${VERSION}_${ARCH}.deb" "packaging/pulsarkey_${VERSION}_${ARCH}.deb"
echo "✅ Successfully generated dist/pulsarkey_${VERSION}_${ARCH}.deb"
