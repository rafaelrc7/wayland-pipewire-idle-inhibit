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

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      description = "Configuration for wayland-pipewire-idle-inhibit";
      example = literalExpression ''
        {
          verbosity = "WARN";
          media_minimum_duration = 5;
          node_blacklist = [
              { name = "spotify"; }
          ];
        }
      '';
    };

    systemdTarget = mkOption {
      type = lib.types.str;
      default = "graphical-session.target";
      example = "sway-session.target";
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
        ExecStart = "${pkgs.wayland-pipewire-idle-inhibit}/bin/wayland-pipewire-idle-inhibit --config ${configFile}";
        Restart = "always";
        RestartSec = 10;
      };
    };
  };
}

