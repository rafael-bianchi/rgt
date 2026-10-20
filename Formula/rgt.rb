class Rgt < Formula
  desc "Rust Graph Tracker: Numeric and date provenance tracking for LLM coding agents"
  homepage "https://github.com/rafael-bianchi/rgt"
  version "0.8.0"
  license "Apache-2.0"

  on_macos do
    if Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "44c191fb2ce47ce0554d6844b6ed229d814e274a6de84b404b26421c1f78f2f8"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "3ec9b7a09982ed891681a21aa60b8eaabd9ad6931e61466d80cd8e18b72ec49a"
    elsif Hardware::CPU.arm?
      url "#{homepage}/releases/download/v#{version}/rgt-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "9ec3ba61189fcba563be5272385e445a99490d3af9ec1c9302b3937d006dc41f"
    end
  end

  def install
    bin.install "rgt"
  end

  test do
    assert_match "Rust Graph Tracker", shell_output("#{bin}/rgt --help")
  end
end
