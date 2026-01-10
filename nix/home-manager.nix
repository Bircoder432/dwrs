{
  config,
  lib,
  pkgs,
  ...
}:

with lib;

{
  options.programs.dwrs = {
    enable = mkEnableOption "dwrs downloader";

    package = mkOption {
      type = types.package;
      default = self.packages.dwrs;
      description = "dwrs package to use";
    };
    settings.msg_template = mkOption {
      type = type.str;
      default = "{download} {url} → {output}";
    };
    settings.template = mkOption {
      type = types.str;
      default = "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({percent}%) {msg}";
    };
    settings.bar_chars = mkOption {
      type = types.str;
      default = "█▌░";
    };
    settings.workers = mkOption {
      type = types.int;
      default = 1;
    };
  };

  config = mkIf (config.programs.dwrs.enable) {
    home.packages = [ config.programs.dwrs.package ];
    xdg.configFile."dwrs/config.toml".text = ''
      msg_template = "${config.programs.dwrs.settings.msg_template}"
      template = "${config.programs.dwrs.settings.template}"
      bar_chars = "${config.programs.dwrs.settings.bar_chars}"
      workers = ${toString config.programs.dwrs.settings.workers}
    '';
  };
}
