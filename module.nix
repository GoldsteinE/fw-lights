fw-lights: { config, pkgs, ... }:
let
  cfg = config.fw-lights;
  description = "Framework LED Matrix daemon";
  socket_path = "/run/fw-lights/fw-lights.sock";

  configToml = (pkgs.formats.toml {}).generate "fw-lights.toml" (cfg // {
    inherit socket_path;
  });

  nc = "${pkgs.netcat}/bin/nc";
  sendChargerEvent = pkgs.writeShellScript "fw-lights-send-charger-event" ''
    set -e
    echo charger | nc -U ${socket_path}
  '';
in
{
  options.fw-lights = with pkgs.lib; {
    enable = mkEnableOption description;

    displays = mkOption {
      description = "Paths to serial devices corresponding to displays";
      type = types.attrsOf types.str;
      example = {
        left = "/dev/ttyACM1";
        right = "/dev/ttyACM0";
      };
    };

    builtin = mkOption {
      description = "Configuration for builtin watchers";
      type = types.submodule {
        options = {
          charger = mkOption {
            description = "Display an animation when a charger was plugged in";
            default = null;
            type = types.nullOr (types.submodule {
              options = {
                animation_left = mkOption {
                  description = "Animation to play when a charger was plugged at the left side";
                  type = types.str;
                };
                animation_right = mkOption {
                  description = "Animation to play when a charger was plugged at the right side";
                  type = types.str;
                };
                offset = mkOption {
                  description = "Place to put the animation as an offset from the port position";
                  type = types.ints.s8;
                  default = 0;
                };
                left_display = mkOption {
                  description = "Display to play the animation when the charger was plugged at the left side";
                  type = types.str;
                  default = "left";
                };
                right_display = mkOption {
                  description = "Display to play the animation when the charger was plugged at the right side";
                  type = types.str;
                  default = "right";
                };
              };
            });
          };
        };
      };
      default = {};
    };

    animations = mkOption {
      description = "Named animations that can be played";
      # requires something like tagged submodules to do
      type = types.attrs;
      default = {};
    };
  };

  config = {
    users.groups.fw-lights = {};
    systemd.services.fw-lights = {
      inherit description;
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        # sadly, we need to be root to access EC
        User = "root";
        Group = "fw-lights";
        UMask = "0007";
        ExecStart = "${fw-lights}/bin/fw-lights ${configToml}";
        # TODO: do some hardening? is there even a point?
        RuntimeDirectory = "fw-lights";
      };
    };
    services.udev.extraRules = if cfg.builtin.charger != null then ''
      SUBSYSTEM=="power_supply",ENV{POWER_SUPPLY_ONLINE}=="1",RUN+="${sendChargerEvent}"
    '' else "";
  };
}
