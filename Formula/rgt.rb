class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.1.0"
  license "MIT OR Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_AARCH64_DARWIN"
    else
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_X86_64_DARWIN"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_SHA256_X86_64_LINUX"
    elsif Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_AARCH64_LINUX"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "rgt", shell_output("#{bin}/rgt --version")
  end
end
