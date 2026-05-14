{ pkgs, ... }:

{
  languages.rust = {
    enable = true;
    channel = "stable";
    version = "1.85.0";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
    ];
  };

  packages = [
    pkgs.git
  ];

  scripts = {
    check.exec = "cargo check";
    tests.exec = "cargo test";
    fmt.exec = "cargo fmt --check";
    clippy.exec = "cargo clippy --all-targets -- -D warnings";
    ci.exec = ''
      cargo fmt --check
      cargo clippy --all-targets -- -D warnings
      cargo test
    '';
  };
}
