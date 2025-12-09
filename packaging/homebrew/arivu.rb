class Arivu < Formula
  desc "Unified CLI for accessing 20+ data sources - YouTube, Reddit, arXiv, and more"
  homepage "https://github.com/srv1n/arivu"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "079a18ff55a2675b45c4015f8eec6af9f0285a5ebfff0f6db776c0dc1a401456"
    end
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "50118ee09bac56f2888f8d8edb7f116ef065a3dbfbc018939ca33c1edb7ed469"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "9686a178ebb69b8906895a08b98bf0498db9dcde7e055a41e5efec031148358f"
    end
  end

  def install
    bin.install "arivu"
  end

  test do
    assert_match "arivu", shell_output("#{bin}/arivu --version")
  end
end
