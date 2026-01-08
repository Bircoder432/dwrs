{
  description = "dwrs – parallel downloader";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    home-manager.url = "github:nix-community/home-manager";
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      naersk,
      home-manager,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (final: prev: {
              dwrs = final.callPackage ./nix/package.nix {
                naersk = naersk.lib.${system};
              };
            })
          ];
        };
      in
      {
        packages.default = pkgs.dwrs;
        packages.dwrs = pkgs.dwrs;

        apps.default = {
          type = "app";
          program = "${pkgs.dwrs}/bin/dwrs";
        };
      }
    )
    // {
      homeManagerModules.dwrs = import ./nix/home-manager.nix;
    };
}
