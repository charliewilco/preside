class Preside < Formula
  desc "Manage named local development services"
  homepage "https://github.com/charliewilco/preside"
  license "MIT"
  head "https://github.com/charliewilco/preside.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    assert_match "Manage named local development services", shell_output("#{bin}/pp --help")
  end
end
