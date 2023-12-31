{ pkgs ? import <nixpkgs> { }, ... }:
pkgs.mkShell {
  packages = with pkgs; [
    cargo
    clang
    gdb
    pipewire
    pkg-config
    rustc
    rustfmt
    valgrind
    wayland
    wayland-protocols
  ];

  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
}

