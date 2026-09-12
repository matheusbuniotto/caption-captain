cask "capcap" do
  version "0.0.0"

  on_arm do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v#{version}/capcap-gui-macos-arm64.dmg"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end
  on_intel do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v#{version}/capcap-gui-macos-x86_64.dmg"
    sha256 "0000000000000000000000000000000000000000000000000000000000000000"
  end

  name "Capcap"
  desc "Zero-cloud, local-first video captioning, translation & container muxing"
  homepage "https://github.com/matheusbuniotto/caption-captain"

  app "Capcap.app"

  zap trash: [
    "~/Library/Application Support/com.capcap.gui",
    "~/Library/Caches/com.capcap.gui",
    "~/Library/Saved Application State/com.capcap.gui.savedState",
  ]
end
