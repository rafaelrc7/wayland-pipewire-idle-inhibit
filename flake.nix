{
  description = "Inhibit Wayland idling when audio is played through PipeWire";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.flake-parts.flakeModules.easyOverlay
      ];
      flake = {
        homeModules = rec {
          wayland-pipewire-idle-inhibit = import ./module.nix;
          default = wayland-pipewire-idle-inhibit;
        };
      };
      systems = [ "x86_64-linux" ];
      perSystem = { config, pkgs, ... }: {
        devShells.default = import ./shell.nix { inherit pkgs; };

        packages.wayland-pipewire-idle-inhibit = pkgs.callPackage ./default.nix { };
        packages.default = config.packages.wayland-pipewire-idle-inhibit;

        overlayAttrs = {
          inherit (config.packages) wayland-pipewire-idle-inhibit;
        };

        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            rustfmt.enable = true;
            nixpkgs-fmt.enable = true;
            taplo.enable = true;
            prettier.enable = true;
          };
        };
      };
    };
}

