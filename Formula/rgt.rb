class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.8.1"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "1de9357a55720a2e46a542975ae6d444c8b633b251552f549fbcb1dcb89706cf"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "a121afce2d72979fda4008f0b4518497eda14ac2f1177d692258e96cad2d6595"
    elsif Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "26c6ee5b3349ea1b3fe6a50da09db70b26fee53222a756d12108d3f9ad67fcc5"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "Rust Graph Tracker", shell_output("#{bin}/rgt --help")
  end
end
