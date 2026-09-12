cask "capcap" do
  version "0.1.0"

  on_arm do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v#{version}/capcap-gui-macos-arm64.dmg"
    sha256 "ba2e08ad019c6ee0a3c36da22218f7db5d049821b1c6da1785a5ff1c36124310"
  end
  on_intel do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v#{version}/capcap-gui-macos-x86_64.dmg"
    sha256 "9372c0e9d55a2909695585c48abbfa832cb913e3b54cee920bdcdd08479b7b67"
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
