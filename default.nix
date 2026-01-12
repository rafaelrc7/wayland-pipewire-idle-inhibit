{
  lib,
  pipewire,
  pkg-config,
  rustPlatform,
}:
let
  cargoToml = fromTOML (builtins.readFile ./Cargo.toml);
in
rustPlatform.buildRustPackage {
  inherit (cargoToml.package) version;
  pname = cargoToml.package.name;
  cargoLock.lockFile = ./Cargo.lock;
  src = lib.cleanSource ./.;

  strictDeps = true;

  nativeBuildInputs = [
    pkg-config
    rustPlatform.bindgenHook
  ];

  buildInputs = [
    pipewire
  ];

  meta = {
    inherit (cargoToml.package) description;
    homepage = cargoToml.package.repository;
    mainProgram = cargoToml.package.name;
    license = lib.licenses.gpl3Only;
    platforms = lib.platforms.linux;
  };
}
