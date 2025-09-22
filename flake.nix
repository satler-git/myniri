{
  description = "A full Rust flake";

  inputs = {
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

    treefmt-nix.url = "github:numtide/treefmt-nix";

    systems.url = "github:nix-systems/default";

    crane.url = "github:ipetkov/crane";
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      flake-parts,
      crane,
      ...
    }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;

      imports = [
        inputs.treefmt-nix.flakeModule
      ];

      perSystem =
        {
          config,
          system,
          pkgs,
          lib,
          ...
        }:
        let
          rust-bin = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
          craneLib = (crane.mkLib pkgs).overrideToolchain rust-bin;

          commonArgs = {
            src = craneLib.cleanCargoSource ./.;
            strictDeps = true;

            buildInputs = with pkgs; [ ];

            nativeBuildInputs = with pkgs; [ ];
          };

          cargoArtifacts = craneLib.buildDepsOnly (
            commonArgs
            // {
              pname = "deps";
            }
          );
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              inputs.rust-overlay.overlays.default
            ];
          };

          treefmt = {
            projectRootFile = "flake.nix";

            programs = {
              nixfmt.enable = true;
              rustfmt = {
                enable = true;
                package = rust-bin;
              };
              actionlint.enable = true;
            };
          };

          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo-expand
              cargo-nextest

              rust-bin
            ];
          };

          packages.default = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              pname = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.name;
              version = "unstable-${self.shortRev or "dirty"}";
              # # For workspace
              # cargoExtraArgs = "-p package_name";
              # version = "unstable-${self.shortRev or "dirty"}";
            }
          );
        };
    };
}
