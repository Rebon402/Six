let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      rustc
      cargo
      zig
      gcc
    ];

    shellHook = ''
      echo "--- .six Hardcore Dev Shell (Legacy) ---"
    '';
  }
