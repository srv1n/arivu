class Arivu < Formula
  desc "Unified CLI for accessing 20+ data sources - YouTube, Reddit, arXiv, and more"
  homepage "https://github.com/srv1n/arivu"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "d390c4e2f570d70359e47ea696dad15b1105d3dfcac66a12fd80ab1cb402c869"
    end
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "c24e17de6eeb5c9ba3c194bc424c276a8c99509b26810cce06fe69fda23c536e"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "5e0de9db182cb7d2378dc14b87f6d0ecd6b638fbe10467658b968566b044c748"
    end
  end

  def install
    bin.install "arivu"
  end

  test do
    assert_match "arivu", shell_output("#{bin}/arivu --version")
  end
end
