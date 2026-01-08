{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

let
  cfg = config.programs.dwrs;
in
{
  options.programs.dwrs = {
    enable = mkEnableOption "dwrs downloader";

    package = mkOption {
      type = types.package;
      default = pkgs.dwrs;
      description = "dwrs package to use";
    };
    settings = {
      template = mkOption {
        type = types.str;
        default = "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}";
      };

      bar_chars = mkOption {
        type = types.str;
        default = "█▌░";
      };

      workers = mkOption {
        type = types.int;
        default = 1;
      };
    };
  };
  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];
    xdg.configFile."dwrs/config.toml".text = ''
      template = "${cfg.settings.template}"
      bar_chars = "${cfg.settings.bar_chars}"
      workers = ${toString cfg.settings.workers}
    '';
  };
}
