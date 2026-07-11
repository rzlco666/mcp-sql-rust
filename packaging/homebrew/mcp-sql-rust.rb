class McpSqlRust < Formula
  desc "Token-efficient MCP server for MySQL, PostgreSQL, and SQLite"
  homepage "https://github.com/rzlco666/mcp-sql-rust"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-aarch64-apple-darwin.tar.gz"
      sha256 "3014ed4a731281551f70693dd1aa2bbedd6e7303023e8c87d18e3e093595b197"
    else
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-x86_64-apple-darwin.tar.gz"
      sha256 "3019f10be78d3389c1268aef33a1427e6bc1fddd99488dd3332c58b0884bbe95"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "946153d029bced3af6b6bea89eb4e70c34030468e67f71e9bf1e08345469664d"
    else
      url "https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "e3b29df773b660f29eed8dc24a806be0fb55dcef5a146a714df9cd796faa2f4d"
    end
  end

  def install
    bin.install "mcp-sql-rust"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/mcp-sql-rust --version")
  end
end
