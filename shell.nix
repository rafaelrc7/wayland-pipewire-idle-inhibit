{ pkgs ? import <nixpkgs> { }, ... }:
pkgs.mkShell {
  packages = with pkgs; [
    cargo rustc clang pkg-config rustfmt
    pipewire
    gdb valgrind
  ];

  LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
}

