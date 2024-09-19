{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, crane, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        craneLib = crane.mkLib pkgs;
        mcmpmgr = craneLib.buildPackage {
          src = craneLib.cleanCargoSource ./.;
          buildInputs = [
            pkgs.pkg-config
            pkgs.openssl
          ];
        };
      in
    {
      packages.default = mcmpmgr;
      devShells.default = craneLib.devShell {
        inputsFrom = [ mcmpmgr ];
        packages = [
          pkgs.rust-analyzer
          pkgs.nil
        ];
      };
    });
}

