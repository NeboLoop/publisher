class Neboai < Formula
  desc "Build, validate, and publish to the NeboLoop marketplace"
  homepage "https://github.com/NeboLoop/publisher"
  version "0.1.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/NeboLoop/publisher/releases/download/v#{version}/neboai-darwin-arm64"
      sha256 "" # Updated by release workflow

      def install
        bin.install "neboai-darwin-arm64" => "neboai"
      end
    end

    on_intel do
      url "https://github.com/NeboLoop/publisher/releases/download/v#{version}/neboai-darwin-amd64"
      sha256 "" # Updated by release workflow

      def install
        bin.install "neboai-darwin-amd64" => "neboai"
      end
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/NeboLoop/publisher/releases/download/v#{version}/neboai-linux-arm64"
      sha256 "" # Updated by release workflow

      def install
        bin.install "neboai-linux-arm64" => "neboai"
      end
    end

    on_intel do
      url "https://github.com/NeboLoop/publisher/releases/download/v#{version}/neboai-linux-amd64"
      sha256 "" # Updated by release workflow

      def install
        bin.install "neboai-linux-amd64" => "neboai"
      end
    end
  end

  test do
    assert_match "neboai", shell_output("#{bin}/neboai --version")
  end
end
