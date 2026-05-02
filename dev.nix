{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = [
    pkgs.rustup
    pkgs.gcc
    pkgs.gnumake
    pkgs.pkg-config
    pkgs.openssl
    pkgs.gmp
    pkgs.yices
  ];
  shellHook = ''
    export PATH="$HOME/.cargo/bin:$PATH"
    export CC=gcc
  '';
}
