class Nanoswarm < Formula
  desc "100% on-device autonomous AI agent swarm in native Rust for Apple Silicon"
  homepage "https://github.com/bhubbard/nanoswarm-rs"
  url "https://github.com/bhubbard/nanoswarm-rs/archive/refs/tags/v0.1.0.tar.gz"
  license "MIT"
  head "https://github.com/bhubbard/nanoswarm-rs.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "NanoSwarm", shell_output("#{bin}/nanoswarm --version")
    assert_match "CodeAuditSwarm", shell_output("#{bin}/nanoswarm list-swarms")
  end
end
