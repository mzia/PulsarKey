// ==============================================================================
// 🌌 PulsarKeyBar — Native macOS Menu Bar Status Companion
// Provides an unobtrusive NSStatusItem for the Apple Menu Bar, featuring:
// - Hardware FIDO2 & WebAuthn / Touch ID live presence monitoring
// - Presence Sentinel Auto-Lock on key removal
// - Quick lock screen action
// - One-click access to Ratatui TUI Dashboard, Biometrics, and Setup
// ==============================================================================

import AppKit
import Foundation

class PulsarKeyBarDelegate: NSObject, NSApplicationDelegate {
    private var statusItem: NSStatusItem!
    private var timer: Timer?

    // Menu Item References
    private var headerItem: NSMenuItem!
    private var updateItem: NSMenuItem!
    private var deviceItem: NSMenuItem!
    private var touchIdItem: NSMenuItem!
    private var sentinelItem: NSMenuItem!

    // State
    private var wasConnected: Bool = false
    private var autolockEnabled: Bool = false
    private var lastDeviceName: String = "Scanning..."
    private var touchIdActive: Bool = false
    private var lastUpdateCheck: Date = Date.distantPast

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Run as menu bar accessory (no Dock icon or Cmd-Tab switcher)
        NSApp.setActivationPolicy(.accessory)

        statusItem = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)

        if let button = statusItem.button {
            if #available(macOS 11.0, *) {
                let config = NSImage.SymbolConfiguration(pointSize: 14, weight: .medium)
                if let image = NSImage(systemSymbolName: "key.fill", accessibilityDescription: "PulsarKey")?.withSymbolConfiguration(config) {
                    image.isTemplate = true
                    button.image = image
                } else {
                    button.title = "🌌"
                }
            } else {
                button.title = "🌌"
            }
            button.toolTip = "PulsarKey — Hardware Security Suite"
        }

        buildMenu()
        loadInitialState()
        refreshStatus()

        // Background monitor loop every 2.0 seconds
        timer = Timer.scheduledTimer(withTimeInterval: 2.0, repeats: true) { [weak self] _ in
            self?.refreshStatus()
        }
    }

    private func buildMenu() {
        let menu = NSMenu()
        menu.autoenablesItems = false

        headerItem = NSMenuItem(title: "🌌 PulsarKey Security Suite", action: nil, keyEquivalent: "")
        headerItem.isEnabled = false
        headerItem.attributedTitle = NSAttributedString(
            string: "🌌 PulsarKey Security Suite",
            attributes: [.font: NSFont.boldSystemFont(ofSize: 13)]
        )
        menu.addItem(headerItem)

        updateItem = NSMenuItem(
            title: "🚀 Update Available (Click to Upgrade)",
            action: #selector(openUpdate),
            keyEquivalent: ""
        )
        updateItem.target = self
        updateItem.isHidden = true
        menu.addItem(updateItem)

        deviceItem = NSMenuItem(title: "🔑 Token: Scanning...", action: nil, keyEquivalent: "")
        deviceItem.isEnabled = false
        menu.addItem(deviceItem)

        touchIdItem = NSMenuItem(title: "🍏 Touch ID (WebAuthn): Checking...", action: nil, keyEquivalent: "")
        touchIdItem.isEnabled = false
        menu.addItem(touchIdItem)

        menu.addItem(NSMenuItem.separator())

        sentinelItem = NSMenuItem(
            title: "🛡️ Presence Sentinel Auto-Lock",
            action: #selector(toggleSentinel),
            keyEquivalent: ""
        )
        sentinelItem.target = self
        sentinelItem.state = autolockEnabled ? .on : .off
        menu.addItem(sentinelItem)

        let lockItem = NSMenuItem(title: "🔒 Lock Screen Now", action: #selector(lockScreen), keyEquivalent: "l")
        lockItem.keyEquivalentModifierMask = [.command, .control]
        lockItem.target = self
        menu.addItem(lockItem)

        menu.addItem(NSMenuItem.separator())

        let tuiItem = NSMenuItem(title: "📊 Open Security Dashboard (TUI)...", action: #selector(openTui), keyEquivalent: "")
        tuiItem.target = self
        menu.addItem(tuiItem)

        let bioItem = NSMenuItem(title: "🧬 Biometrics & PIN Manager...", action: #selector(openBio), keyEquivalent: "")
        bioItem.target = self
        menu.addItem(bioItem)

        let setupItem = NSMenuItem(title: "🚀 Run Setup Wizard...", action: #selector(openSetup), keyEquivalent: "")
        setupItem.target = self
        menu.addItem(setupItem)

        let auditItem = NSMenuItem(title: "📜 View Audit Journal...", action: #selector(openAudit), keyEquivalent: "")
        auditItem.target = self
        menu.addItem(auditItem)

        menu.addItem(NSMenuItem.separator())

        let quitItem = NSMenuItem(title: "🚪 Quit PulsarKey Menu", action: #selector(quitApp), keyEquivalent: "q")
        quitItem.target = self
        menu.addItem(quitItem)

        statusItem.menu = menu
    }

    private func loadInitialState() {
        // Read ~/.config/pulsarkey/config.json if available
        let configPath = ("~/.config/pulsarkey/config.json" as NSString).expandingTildeInPath
        if let data = try? Data(contentsOf: URL(fileURLWithPath: configPath)),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            if let al = json["autolock"] as? Bool {
                autolockEnabled = al
                sentinelItem?.state = autolockEnabled ? .on : .off
            }
        }
    }

    private func checkCachedUpdate() -> String? {
        let updatePath = ("~/.config/pulsarkey/update_info.json" as NSString).expandingTildeInPath
        if let data = try? Data(contentsOf: URL(fileURLWithPath: updatePath)),
           let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            if let isAvailable = json["is_update_available"] as? Bool, isAvailable,
               let latestVer = json["latest_version"] as? String {
                return latestVer
            }
        }
        return nil
    }

    private func refreshStatus() {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self = self else { return }

            // Periodically check GitHub for updates (every 30 mins)
            if Date().timeIntervalSince(self.lastUpdateCheck) > 1800 {
                self.lastUpdateCheck = Date()
                self.runPulsarkeyCommand(args: ["update", "--check"])
            }

            // 1. Detect USB Security Key via ioreg
            let (connected, devName) = self.detectUsbKey()

            // 2. Detect Touch ID via bioutil & sudo_local
            let touchIdStatus = self.detectTouchId()

            // 3. Detect cached software update
            let updateVersion = self.checkCachedUpdate()

            DispatchQueue.main.async {
                self.updateUI(isConnected: connected, deviceName: devName, touchId: touchIdStatus, updateVersion: updateVersion)
            }
        }
    }

    private func detectUsbKey() -> (Bool, String) {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/sbin/ioreg")
        process.arguments = ["-p", "IOUSB", "-l"]

        let pipe = Pipe()
        process.standardOutput = pipe

        do {
            try process.run()
            process.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let output = String(data: data, encoding: .utf8)?.lowercased() {
                if output.contains("1050") || output.contains("yubikey") {
                    return (true, "Yubico YubiKey")
                } else if output.contains("20a0") || output.contains("nitrokey") {
                    return (true, "Nitrokey FIDO2")
                } else if output.contains("1209") || output.contains("solokeys") {
                    return (true, "SoloKeys Solo 2")
                } else if output.contains("096e") || output.contains("feitian") {
                    return (true, "Feitian BioPass")
                } else if output.contains("18d1") || output.contains("titan") {
                    return (true, "Google Titan Key")
                } else if output.contains("fido") || output.contains("u2f") {
                    return (true, "FIDO2 Security Key")
                }
            }
        } catch {}

        return (false, "No Security Key Connected")
    }

    private func detectTouchId() -> String {
        // Check bioutil
        let bioutilProcess = Process()
        bioutilProcess.executableURL = URL(fileURLWithPath: "/usr/bin/bioutil")
        bioutilProcess.arguments = ["-r"]
        let pipe = Pipe()
        bioutilProcess.standardOutput = pipe

        var hasHardware = false
        if (try? bioutilProcess.run()) != nil {
            bioutilProcess.waitUntilExit()
            let data = pipe.fileHandleForReading.readDataToEndOfFile()
            if let text = String(data: data, encoding: .utf8) {
                if text.contains("Biometrics for unlock: 1") || text.contains("User Touch ID configuration:") {
                    hasHardware = true
                }
            }
        }

        // Check /etc/pam.d/sudo_local for pam_tid.so
        var sudoLocalActive = false
        if let content = try? String(contentsOfFile: "/etc/pam.d/sudo_local", encoding: .utf8) {
            for line in content.components(separatedBy: .newlines) {
                let trimmed = line.trimmingCharacters(in: .whitespaces)
                if !trimmed.hasPrefix("#") && trimmed.contains("pam_tid.so") {
                    sudoLocalActive = true
                    break
                }
            }
        }

        if hasHardware && sudoLocalActive {
            return "Active (WebAuthn + sudo)"
        } else if hasHardware {
            return "Available (Built-in)"
        } else {
            return "Not Supported"
        }
    }

    private func updateUI(isConnected: Bool, deviceName: String, touchId: String, updateVersion: String?) {
        // Presence Sentinel check: key was pulled!
        if wasConnected && !isConnected && autolockEnabled {
            postNotification(title: "🛡️ PulsarKey Sentinel", body: "Hardware token removed. Locking Mac screen...")
            lockScreen()
        }

        wasConnected = isConnected
        lastDeviceName = deviceName

        if let updateVer = updateVersion {
            updateItem.title = "🚀 Update Available: v\(updateVer) (Click to Upgrade)"
            updateItem.isHidden = false
        } else {
            updateItem.isHidden = true
        }

        if isConnected {
            deviceItem.title = "🔑 Key: \(deviceName) (Online)"
            if let button = statusItem.button {
                button.toolTip = "PulsarKey: \(deviceName) connected"
            }
        } else {
            deviceItem.title = "⚪ Key: Disconnected (Docked/Lid Mode)"
            if let button = statusItem.button {
                button.toolTip = "PulsarKey: No hardware token connected"
            }
        }

        touchIdItem.title = "🍏 Touch ID: \(touchId)"
    }

    @objc private func openUpdate() {
        launchInTerminal(command: "pulsarkey update")
    }

    @objc private func toggleSentinel() {
        autolockEnabled = !autolockEnabled
        sentinelItem.state = autolockEnabled ? .on : .off

        // Invoke pulsarkey autolock toggle
        runPulsarkeyCommand(args: ["autolock", autolockEnabled ? "enable" : "disable"])

        postNotification(
            title: "PulsarKey Sentinel",
            body: "Auto-Lock on key removal: \(autolockEnabled ? "Enabled" : "Disabled")"
        )
    }

    @objc private func lockScreen() {
        // 1. Send Control+Command+Q to System Events
        let script = "tell application \"System Events\" to keystroke \"q\" using {control down, command down}"
        let appleScript = NSAppleScript(source: script)
        appleScript?.executeAndReturnError(nil)

        // 2. Fallback pmset displaysleepnow
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/bin/pmset")
        p.arguments = ["displaysleepnow"]
        try? p.run()
    }

    @objc private func openTui() {
        launchInTerminal(command: "pulsarkey")
    }

    @objc private func openBio() {
        launchInTerminal(command: "pulsarkey bio")
    }

    @objc private func openSetup() {
        launchInTerminal(command: "sudo pulsarkey setup")
    }

    @objc private func openAudit() {
        launchInTerminal(command: "pulsarkey audit; echo ''; read -p 'Press Enter to close...'")
    }

    private func launchInTerminal(command: String) {
        let escaped = command.replacingOccurrences(of: "\"", with: "\\\"")
        let script = "tell application \"Terminal\" to do script \"\(escaped)\""
        if let appleScript = NSAppleScript(source: script) {
            appleScript.executeAndReturnError(nil)
        }
        let activateScript = "tell application \"Terminal\" to activate"
        if let appleScript = NSAppleScript(source: activateScript) {
            appleScript.executeAndReturnError(nil)
        }
    }

    private func runPulsarkeyCommand(args: [String]) {
        let p = Process()
        p.executableURL = URL(fileURLWithPath: "/usr/local/bin/pulsarkey")
        p.arguments = args
        try? p.run()
    }

    private func postNotification(title: String, body: String) {
        let escapedTitle = title.replacingOccurrences(of: "\"", with: "\\\"")
        let escapedBody = body.replacingOccurrences(of: "\"", with: "\\\"")
        let script = "display notification \"\(escapedBody)\" with title \"\(escapedTitle)\""
        let appleScript = NSAppleScript(source: script)
        appleScript?.executeAndReturnError(nil)
    }

    @objc private func quitApp() {
        NSApp.terminate(nil)
    }
}

let app = NSApplication.shared
let delegate = PulsarKeyBarDelegate()
app.delegate = delegate
app.run()
