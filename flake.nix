{
  description = "The Six Language - Hardcore Systems Programming Environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            zig
            pkg-config
            openssl
            # Windows cross-compilation helpers if needed
            # pkgs.pkgsCross.mingwW64.buildPackages.gcc 
          ];

          shellHook = ''
            echo "--- .six Hardcore Dev Environment ---"
            echo "Rust: $(rustc --version)"
            echo "Zig: $(zig version)"
            echo "Nix: reproducible environment active."
          '';
        };
      }
    );
}
