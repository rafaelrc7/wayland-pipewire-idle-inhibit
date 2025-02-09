{
  description = "Inhibit Wayland idling when audio is played through PipeWire";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.flake-parts.flakeModules.easyOverlay
      ];
      flake = {
        nixosModules = rec {
          wayland-pipewire-idle-inhibit = import ./modules/nixos.nix;
          default = wayland-pipewire-idle-inhibit;
        };
        homeModules = rec {
          wayland-pipewire-idle-inhibit = import ./modules/home-manager.nix;
          default = wayland-pipewire-idle-inhibit;
        };
      };
      perSystem =
        { config, pkgs, ... }:
        {
          packages = rec {
            default = wayland-pipewire-idle-inhibit;
            wayland-pipewire-idle-inhibit = pkgs.callPackage ./default.nix { };
          };

          overlayAttrs = {
            inherit (config.packages) wayland-pipewire-idle-inhibit;
          };

          devShells.default = import ./shell.nix {
            inherit pkgs;
            inputsFrom = [ config.packages.wayland-pipewire-idle-inhibit ];
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
    };
}
