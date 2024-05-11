{ pkgs ? import <nixpkgs> { }, ... }:
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    clang
    clippy
    gdb
    pipewire
    pkg-config
    rustc
    rustfmt
    rust-analyzer
    valgrind
    wayland
    wayland-protocols
  ];

  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
}

