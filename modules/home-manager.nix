{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib) mkIf;
  cfg = config.services.wayland-pipewire-idle-inhibit;
  tomlFormat = pkgs.formats.toml { };
  configFile = tomlFormat.generate "wayland-pipewire-idle-inhibit.toml" cfg.settings;
in
{
  imports = [ ./common.nix ];

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
