class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.6.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "6a8915d22b9dd7b0d661d27e517f3c76fbf4e4be8ea5d4bfc767243b16d16fde"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "9d98b1c21b6e90504ebbfcc0a7159fca824055854ceff75901a64739b6e2669b"
    elsif Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "f4f3794319bd78ee2ce1d3680a6d588e80800d97ca0aca1b881c0e18ee54813e"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "Rust Graph Tracker", shell_output("#{bin}/rgt --help")
  end
end
