{ lib, pkgs, ... }:
with lib;
let
  tomlFormat = pkgs.formats.toml { };
in
{
  options.services.wayland-pipewire-idle-inhibit = {
    enable = mkEnableOption "wayland-pipewire-idle-inhibit";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ../default.nix { };
      description = ''
        The wayland-pipewire-idle-inhibit package to use.
      '';
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          verbosity = "WARN";
          media_minimum_duration = 5;
          node_blacklist = [
              { name = "spotify"; }
          ];
        }
      '';
      description = ''
        Configuration for wayland-pipewire-idle-inhibit.
      '';
    };

    systemdTarget = mkOption {
      type = types.str;
      default = "graphical-session.target";
      example = "sway-session.target";
      description = ''
        systemd target to bind to.
      '';
    };
  };
}

