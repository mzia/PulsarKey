# 🍎 PulsarKey for macOS

PulsarKey brings unified hardware-backed FIDO2 and biometric security (YubiKey Bio, YubiKey 5 Series, Security Key NFC) to macOS (macOS 12 Monterey or later).

It provides a single unified experience across both **Pop!_OS Linux** and **macOS Darwin**, featuring:
- **Unified Security CLI Suite:** Instant dashboard, on-key biometric & PIN management, security strictness profiles, offline paper keys, and SSH setup with zero GUI overhead.
- **System PAM Integration:** Protects `sudo`, lockscreen (`/etc/pam.d/screensaver`), and elevation prompts (`/etc/pam.d/authorization`).
- **Presence Sentinel Daemon:** Monitors USB insertion and removal via `IOUSB`/`ioreg`, instantly locking the Mac screen when your security token is pulled.
- **Emergency Paper Key Suite:** Generate offline recovery codes and emergency unlock documentation.
- **Hardware SSH & Git Signing:** Automate `ed25519-sk` and `ecdsa-sk` key generation and commit signing.

---

## 📦 Prerequisites

Install the required FIDO2 PAM module and Yubico CLI via Homebrew:

```bash
brew install pam-u2f ykman
```

---

## 🔨 Building for macOS

To compile binaries and assemble the `PulsarKey.app` application bundle:

```bash
# Clone the repository
git clone https://github.com/mzia/PulsarKey.git
cd PulsarKey

# Run the macOS packaging script
./packaging/macos/build_mac.sh
```

### Outputs generated:
1. **`dist/macos/PulsarKey.app`**: Complete macOS Application Bundle.
2. **`dist/PulsarKey-1.4.1.dmg`**: Drag-and-drop disk image installer (when built on macOS).
3. **`dist/PulsarKey-1.4.1.pkg`**: Standard macOS package installer (via `pkgbuild`).

---

## 🍺 Installation via Homebrew Cask

Once packaged or downloaded:

```bash
brew install --cask packaging/macos/homebrew/pulsarkey.rb
```

This installs `PulsarKey.app` to `/Applications` and creates symlinks for CLI commands:
```bash
pulsarkey status
pulsarkey setup
pulsarkey applet
```

---

## 🛡️ macOS PAM Configuration Details

macOS PAM files are located in `/etc/pam.d/`:

| Service | PAM Configuration File | Description |
| :--- | :--- | :--- |
| **Sudo CLI** | `/etc/pam.d/sudo` | Terminal privilege elevation |
| **Lockscreen** | `/etc/pam.d/screensaver` | macOS screensaver and display lock authentication |
| **GUI Elevation** | `/etc/pam.d/authorization` | SecurityAgent system prompts (system settings, keychain access) |

PulsarKey handles atomic file updates with automatic `.pulsarkey.bak` backup creation before any modification.

---

## 🔒 Presence Sentinel on macOS

On macOS, PulsarKey detects token presence via native IOKit USB registries (`ioreg`). When the token is unplugged:
1. A native macOS notification is sent via AppleScript.
2. The session is immediately locked via native macOS security framework (`SACLockScreenImmediate`) or `pmset displaysleepnow`.
3. An audit record is logged to `~/.config/pulsarkey/audit.log`.

To run the Sentinel monitor in the background on macOS:
```bash
pulsarkey applet &
```
