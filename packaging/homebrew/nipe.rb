class Nipe < Formula
  desc "Advanced Tor Network Security Gateway"
  homepage "https://github.com/Arunkumarkethana/tor-war"
  url "https://github.com/Arunkumarkethana/tor-war/releases/download/v1.0.0/nipe-macos-universal.tar.gz"
  version "1.0.0"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"

  depends_on "tor"

  def install
    bin.install "nipe"
  end

  def caveats
    <<~EOS
      Nipe requires root privileges to configure the firewall.
      Always run with sudo:
        sudo nipe start
    EOS
  end

  test do
    system "#{bin}/nipe", "--version"
  end
end
