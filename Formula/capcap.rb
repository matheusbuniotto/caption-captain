class Capcap < Formula
  desc "Zero-cloud, local-first video captioning, translation & container muxing"
  homepage "https://github.com/matheusbuniotto/caption-captain"
  version "0.0.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.0.0/capcap-macos-arm64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000"
    else
      url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.0.0/capcap-macos-x86_64.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.0.0/capcap-linux-x86_64.tar.gz"
    sha256 "0000000000000000000000000000000000000000000000000000000000000"
  end

  def install
    bin.install "capcap"
  end

  test do
    system "#{bin}/capcap", "--help"
  end
end
