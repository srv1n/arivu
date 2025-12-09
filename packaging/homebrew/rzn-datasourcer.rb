class Arivu < Formula
  desc "Unified CLI for accessing 20+ data sources via MCP protocol"
  homepage "https://github.com/srv1n/arivu"
  version "0.1.0"
  license "MIT"

  if OS.mac?
    if Hardware::CPU.arm?
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_ARM64"
    else
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_X86_64"
    end
  elsif OS.linux?
    if Hardware::CPU.arm?
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX_ARM64"
    else
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_FOR_LINUX"
    end
  end

  def install
    bin.install "arivu"

    # Install shell completions (when implemented)
    # bash_completion.install "completions/arivu.bash" => "arivu"
    # zsh_completion.install "completions/_arivu"
    # fish_completion.install "completions/arivu.fish"

    # Install man page (when implemented)
    # man1.install "docs/arivu.1"
  end

  test do
    # Test that the binary works
    assert_match "Arivu", shell_output("#{bin}/arivu --help")
    assert_match version.to_s, shell_output("#{bin}/arivu --version")

    # Test basic functionality
    system "#{bin}/arivu", "list"
  end

  def caveats
    <<~EOS
      Arivu has been installed!

      First-time setup:
        arivu setup                             # Interactive configuration wizard

      Quick start:
        arivu list                              # List available connectors
        arivu search youtube "rust programming" # Search YouTube videos
        arivu get youtube dQw4w9WgXcQ           # Get video with transcript
        arivu tools slack                       # Show Slack connector tools

      Configure authenticated connectors:
        arivu setup slack                       # Interactive Slack setup
        arivu config set github --value "token" # Set GitHub token
        arivu config test slack                 # Test authentication

      Documentation:
        https://github.com/srv1n/arivu/blob/main/CLI_USAGE.md

      Report issues:
        https://github.com/srv1n/arivu/issues
    EOS
  end
end
