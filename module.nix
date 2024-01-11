{ config, lib, pkgs, ... }:
with lib;
let
  cfg = config.services.wayland-pipewire-idle-inhibit;
  tomlFormat = pkgs.formats.toml { };
  configFile = tomlFormat.generate "wayland-pipewire-idle-inhibit.toml" cfg.settings;
in
{
  options.services.wayland-pipewire-idle-inhibit = {
    enable = mkEnableOption "wayland-pipewire-idle-inhibit";

    package = mkOption {
      type = types.package;
      default = pkgs.callPackage ./default.nix { };
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

  config = mkIf cfg.enable {
    systemd.user.services.wayland-pipewire-idle-inhibit = {
      Unit = {
        Description = "Inhibit Wayland idling when media is played through pipewire";
        Documentation = "https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit";
      };

      Install.WantedBy = [ cfg.systemdTarget ];

      Service = {
        ExecStart = "${cfg.package}/bin/wayland-pipewire-idle-inhibit --config ${configFile}";
        Restart = "always";
        RestartSec = 10;
      };
    };
  };
}

