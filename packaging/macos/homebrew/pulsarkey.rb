cask "pulsarkey" do
  version "1.4.0"
  sha256 :no_check

  url "https://github.com/mzia/PulsarKey/releases/download/v#{version}/PulsarKey-#{version}.dmg"
  name "PulsarKey"
  desc "Hardware-backed FIDO2 & Biometric Security Suite for macOS & Linux"
  homepage "https://github.com/mzia/PulsarKey"

  depends_on macos: ">= :monterey"
  depends_on formula: "pam-u2f"
  depends_on formula: "ykman"

  app "PulsarKey.app"
  binary "#{appdir}/PulsarKey.app/Contents/MacOS/pulsarkey"
  binary "#{appdir}/PulsarKey.app/Contents/MacOS/pulsarkey-settings"

  postflight do
    set_permissions "#{appdir}/PulsarKey.app/Contents/MacOS/pulsarkey", "0755"
    set_permissions "#{appdir}/PulsarKey.app/Contents/MacOS/pulsarkey-settings", "0755"
  end

  zap trash: [
    "~/.config/pulsarkey",
    "/etc/yubico/u2f_keys",
  ]
end
