class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.4.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "73a18d319314ed64b09bcef45d47a67cbce3e2361cad72a29c1edf7f53f74b2f"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "0637ce8f7fe1c66887fead8bc99220638f83030984f9549085503bef0787dda4"
    elsif Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "a3a2bbccfeccb3a337be66569e3c337868428319407a78962f7b2cd273b501a9"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "Rust Graph Tracker", shell_output("#{bin}/rgt --help")
  end
end
