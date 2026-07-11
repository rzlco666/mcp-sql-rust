class McpSqlRust < Formula
  desc "Token-efficient MCP server for MySQL, PostgreSQL, and SQLite"
  homepage "https://github.com/rzlco666/mcp-sql-rust"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_ME_AARCH64_APPLE_DARWIN"
    else
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_ME_X86_64_APPLE_DARWIN"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_ME_AARCH64_LINUX"
    else
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_ME_X86_64_LINUX"
    end
  end

  def install
    bin.install "mcp-sql-rust"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/mcp-sql-rust --version")
  end
end
