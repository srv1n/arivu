class Arivu < Formula
  desc "Unified CLI for accessing 20+ data sources - YouTube, Reddit, arXiv, and more"
  homepage "https://github.com/srv1n/arivu"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "e1e4727541b3efd13537181834e6bf11c3a36301a51ed45bcf7923faa8374658"
    end
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "50118ee09bac56f2888f8d8edb7f116ef065a3dbfbc018939ca33c1edb7ed469"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "344ec244ad372b7e94b6ef07225ef62fdacfe597bd688fe428da5a60a2540065"
    end
  end

  def install
    bin.install "arivu"
  end

  test do
    assert_match "arivu", shell_output("#{bin}/arivu --version")
  end
end
