{
  description = "Inhibit Wayland idling when audio is played through PipeWire";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } (
      { self, withSystem, ... }:
      {
        imports = [
          inputs.flake-parts.flakeModules.easyOverlay
          inputs.treefmt-nix.flakeModule
        ];

        flake.nixosModules.default = self.nixosModules.wayland-pipewire-idle-inhibit;
        flake.nixosModules.wayland-pipewire-idle-inhibit =
          { lib, pkgs, ... }:
          {
            imports = [ ./nix/modules/nixos.nix ];
            services.wayland-pipewire-idle-inhibit.package = lib.mkDefault (
              withSystem pkgs.stdenv.hostPlatform.system (
                { self', ... }: self'.packages.wayland-pipewire-idle-inhibit
              )
            );
          };

        flake.homeModules.default = self.homeModules.wayland-pipewire-idle-inhibit;
        flake.homeModules.wayland-pipewire-idle-inhibit =
          { lib, pkgs, ... }:
          {
            imports = [ ./nix/modules/home-manager.nix ];
            services.wayland-pipewire-idle-inhibit.package = lib.mkDefault (
              withSystem pkgs.stdenv.hostPlatform.system (
                { self', ... }: self'.packages.wayland-pipewire-idle-inhibit
              )
            );
          };

        systems = import inputs.systems;
        perSystem =
          { self', pkgs, ... }:
          {
            packages = {
              default = self'.packages.wayland-pipewire-idle-inhibit;
              wayland-pipewire-idle-inhibit = pkgs.callPackage ./default.nix { };
            };

            overlayAttrs = {
              inherit (self'.packages) wayland-pipewire-idle-inhibit;
            };

            devShells.default = import ./shell.nix {
              inherit pkgs;
              inputsFrom = [ self'.packages.wayland-pipewire-idle-inhibit ];
            };

            treefmt.config = {
              projectRootFile = "flake.nix";
              programs = {
                nixfmt.enable = true;
                prettier.enable = true;
                rustfmt.enable = true;
                taplo.enable = true;
              };
            };
          };
      }
    );
}
