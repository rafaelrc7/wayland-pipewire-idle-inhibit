{ clang
, lib
, libclang
, pipewire
, pkg-config
, rustPlatform
, wayland
, wayland-protocols
}:
let cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
in rustPlatform.buildRustPackage rec {
  inherit (cargoToml.package) version;
  pname = cargoToml.package.name;
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;

  nativeBuildInputs = [
    pkg-config clang
  ];

  buildInputs = [
    pipewire
    wayland wayland-protocols
  ];

  LIBCLANG_PATH = "${libclang.lib}/lib";
}

