# This repo isn't named `homebrew-*`, so `brew tap` needs the explicit URL:
#   brew tap matheusbuniotto/caption-captain https://github.com/matheusbuniotto/caption-captain
#   brew install capcap
class Capcap < Formula
  desc "Zero-cloud, local-first video captioning, translation & container muxing"
  homepage "https://github.com/matheusbuniotto/caption-captain"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.1.0/capcap-macos-arm64.tar.gz"
      sha256 "5cfb181c6e16e46c44ddd7c2259b3860095e86b2e529c468e278eeb996632284"
    else
      url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.1.0/capcap-macos-x86_64.tar.gz"
      sha256 "7633b5b5326eddf020f72ec23fffff2fdf45a0b0eb4a9e02df81de4b137ac3e7"
    end
  end

  on_linux do
    url "https://github.com/matheusbuniotto/caption-captain/releases/download/v0.1.0/capcap-linux-x86_64.tar.gz"
    sha256 "c7429ac3108e9db5e1ec96919e9555a8acbdb6193833805a5c17f369b754f240"
  end

  def install
    bin.install "capcap"
    bin.install "ffmpeg" if File.exist?("ffmpeg")
  end

  test do
    system "#{bin}/capcap", "--help"
  end
end
