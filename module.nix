{ self, ... }:
let
  commonModule =
    { lib, pkgs, ... }:
    let
      tomlFormat = pkgs.formats.toml { };
      inherit (lib)
        literalExpression
        mkEnableOption
        mkOption
        types
        ;
    in
    {
      options.services.wayland-pipewire-idle-inhibit = {
        enable = mkEnableOption "wayland-pipewire-idle-inhibit";

        package = mkOption {
          type = types.package;
          default = self.packages.${pkgs.stdenv.hostPlatform.system}.wayland-pipewire-idle-inhibit;
          description = "The wayland-pipewire-idle-inhibit package to use.";
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
          description = "Configuration for wayland-pipewire-idle-inhibit.";
        };

        systemdTarget = mkOption {
          type = types.str;
          default = "graphical-session.target";
          example = "sway-session.target";
          description = "systemd target to bind to.";
        };
      };
    };
in
{
  flake.nixosModules.default = self.nixosModules.wayland-pipewire-idle-inhibit;
  flake.nixosModules.wayland-pipewire-idle-inhibit =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.services.wayland-pipewire-idle-inhibit;
      tomlFormat = pkgs.formats.toml { };
      configFile = tomlFormat.generate "wayland-pipewire-idle-inhibit.toml" cfg.settings;
      inherit (lib) mkIf;
    in
    {
      imports = [ commonModule ];

      config = mkIf cfg.enable {
        systemd.user.services.wayland-pipewire-idle-inhibit = {
          unitConfig = {
            Description = "Inhibit Wayland idling when media is played through pipewire";
            Documentation = "https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit";
            After = [
              "pipewire.service"
              cfg.systemdTarget
            ];
            Wants = [ "pipewire.service" ];
          };

          wantedBy = [ cfg.systemdTarget ];

          serviceConfig = {
            ExecStart = "${lib.getExe cfg.package} --config ${configFile}";
            Restart = "always";
            RestartSec = 10;
          };
        };
      };
    };

  flake.homeModules.default = self.homeModules.wayland-pipewire-idle-inhibit;
  flake.homeModules.wayland-pipewire-idle-inhibit =
    {
      config,
      lib,
      pkgs,
      ...
    }:
    let
      cfg = config.services.wayland-pipewire-idle-inhibit;
      tomlFormat = pkgs.formats.toml { };
      configFile = tomlFormat.generate "wayland-pipewire-idle-inhibit.toml" cfg.settings;
      inherit (lib) mkIf;
    in
    {
      imports = [ commonModule ];

      config = mkIf cfg.enable {
        systemd.user.services.wayland-pipewire-idle-inhibit = {
          Unit = {
            Description = "Inhibit Wayland idling when media is played through pipewire";
            Documentation = "https://github.com/rafaelrc7/wayland-pipewire-idle-inhibit";
            After = [
              "pipewire.service"
              cfg.systemdTarget
            ];
            Wants = [ "pipewire.service" ];
          };

          Install.WantedBy = [ cfg.systemdTarget ];

          Service = {
            ExecStart = "${lib.getExe cfg.package} --config ${configFile}";
            Restart = "always";
            RestartSec = 10;
          };
        };
      };
    };
}
