class Arivu < Formula
  desc "Unified CLI for accessing 20+ data sources - YouTube, Reddit, arXiv, and more"
  homepage "https://github.com/srv1n/arivu"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "13c36271d851db7278c82be74e350ee6277bc3223ac06ffc9dc3d129919d5dde"
    end
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "31b6e9b246a1233487235204cca8e74b41870419d986fb9ea4ac55d0b9674b4a"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/srv1n/arivu/releases/download/v#{version}/arivu-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "60722e85066ea5d88315947d417deea3667739f8785d8314cde00f62d98f71b1"
    end
  end

  def install
    bin.install "arivu"
  end

  test do
    assert_match "arivu", shell_output("#{bin}/arivu --version")
  end
end
