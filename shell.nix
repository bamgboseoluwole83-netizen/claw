{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    pkgs.rustc
    pkgs.yices
    pkgs.gmp
    pkgs.openssl
    pkgs.pkg-config
    pkgs.gcc
    pkgs.binutils
    pkgs.glibc
  ];
  shellHook = ''
    export CC=gcc
  '';
}
