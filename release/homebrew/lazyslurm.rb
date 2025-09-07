class Lazyslurm < Formula
  desc "A terminal UI for monitoring and managing SLURM jobs"
  homepage "https://github.com/hill/lazyslurm"
  version "0.1.0"

  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/hill/lazyslurm/releases/download/v0.1.0/lazyslurm-aarch64-apple-darwin.tar.xz"
    sha256 "TBD" # Will be updated with actual hash
  elsif OS.mac? && Hardware::CPU.intel?
    url "https://github.com/hill/lazyslurm/releases/download/v0.1.0/lazyslurm-x86_64-apple-darwin.tar.xz"
    sha256 "TBD" # Will be updated with actual hash
  elsif OS.linux?
    url "https://github.com/hill/lazyslurm/releases/download/v0.1.0/lazyslurm-x86_64-unknown-linux-gnu.tar.xz"
    sha256 "TBD" # Will be updated with actual hash
  end

  license "MIT"

  def install
    bin.install "lazyslurm"
  end

  test do
    system "#{bin}/lazyslurm", "--version"
  end
end