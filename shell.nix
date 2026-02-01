{
  pkgs ? import <nixpkgs> { },
  devTools ? true,
  inputsFrom ? [ (pkgs.callPackage ./default.nix { }) ],
  ...
}:
pkgs.mkShell {
  inherit inputsFrom;
  strictDeps = true;
  nativeBuildInputs =
    with pkgs;
    [
      cargo
      rustc
    ]
    ++ pkgs.lib.optionals devTools [
      clippy
      gdb
      rust-analyzer
      rustfmt
      valgrind
    ];
}
