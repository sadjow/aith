{
  description = "Account profile switching for AI coding tools";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        aith = pkgs.rustPlatform.buildRustPackage {
          pname = "aith";
          version = cargoToml.package.version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;

          meta = {
            description = cargoToml.package.description;
            homepage = cargoToml.package.homepage;
            license = pkgs.lib.licenses.asl20;
            mainProgram = "aith";
          };
        };
      in
      {
        packages = {
          default = aith;
          aith = aith;
        };

        apps.default = flake-utils.lib.mkApp { drv = aith; };
      }
    );
}
