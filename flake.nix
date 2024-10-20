{
  description = "mcmpmgr - A minecraft modpack manager";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};

        inherit (pkgs) lib;

        craneLib = crane.mkLib pkgs;
        src = craneLib.cleanCargoSource ./.;

        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          buildInputs = [
            pkgs.openssl
          ] ++ lib.optionals pkgs.stdenv.isDarwin [
            # Additional darwin specific inputs can be set here
            pkgs.libiconv
          ];

          nativeBuildInputs = [
            pkgs.pkg-config
          ];

          # Additional environment variables can be set directly
          # MY_CUSTOM_VAR = "some value";
        };

        craneLibLLvmTools = craneLib.overrideToolchain
          (fenix.packages.${system}.complete.withComponents [
            "cargo"
            "llvm-tools"
            "rustc"
          ]);

        # Build *just* the cargo dependencies (of the entire workspace),
        # so we can reuse all of that work (e.g. via cachix) when running in CI
        # It is *highly* recommended to use something like cargo-hakari to avoid
        # cache misses when building individual top-level-crates
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

        fileSetForCrate = crate: lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [
            ./Cargo.toml
            ./Cargo.lock
            crate
          ];
        };

        # Build the top-level crates of the workspace as individual derivations.
        # This allows consumers to only depend on (and build) only what they need.
        # Though it is possible to build the entire workspace as a single derivation,
        # so this is left up to you on how to organize things
        mcmpmgr = craneLib.buildPackage (individualCrateArgs // {
          pname = "mcmpmgr";
          cargoExtraArgs = "-p mcmpmgr";
          src = craneLib.cleanCargoSource ./.;
        });
        mmm = craneLib.buildPackage (individualCrateArgs // rec {
          pname = "mmm";
          cargoExtraArgs = "-p mmm";
          src = craneLib.cleanCargoSource ./.;
          buildInputs = commonArgs.buildInputs ++ (with pkgs; [
            expat
            fontconfig
            freetype
            freetype.dev
            libGL
            pkg-config
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            wayland
            libxkbcommon
          ]);
          LD_LIBRARY_PATH = builtins.foldl' (a: b: "${a}:${b}/lib") "${pkgs.vulkan-loader}/lib" buildInputs;
        });
      in
      {
        checks = { };

        packages = {
          inherit mcmpmgr;
          inherit mmm;
          default = mcmpmgr;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          mcmpmgr-workspace-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };


        apps = {
          mcmpmgr = flake-utils.lib.mkApp {
            drv = mcmpmgr;
          };
          mmm = flake-utils.lib.mkApp {
            drv = mmm;
          };
        };


        devShells.default = craneLib.devShell rec {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [
            pkgs.cargo-hakari
            pkgs.nil
            pkgs.rust-analyzer
          ];

          buildInputs = with pkgs; [
            expat
            fontconfig
            freetype
            freetype.dev
            libGL
            pkg-config
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            wayland
            libxkbcommon
          ];

          LD_LIBRARY_PATH = builtins.foldl' (a: b: "${a}:${b}/lib") "${pkgs.vulkan-loader}/lib" buildInputs;
        };
      });
}
