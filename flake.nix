{
  description = "Premath: kernel doctrine of definability for agent orchestration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    jj = {
      url = "github:jj-vcs/jj";
      flake = false;
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      rust-overlay,
      crane,
      jj,
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.stable.latest.default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
            ];
          };

          craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

          # Source filtering: only include Rust/TOML/JSONL files
          src = pkgs.lib.cleanSourceWith {
            src = self;
            filter =
              path: type:
              (craneLib.filterCargoSources path type)
              || (builtins.match ".*\\.jsonl$" path != null);
          };

          # Common build args
          commonArgs = {
            inherit src;
            pname = "premath";
            version = "0.1.0";

            nativeBuildInputs = with pkgs; [
              pkg-config
              cmake # for rocksdb
            ];

            buildInputs =
              with pkgs;
              [
                openssl
                rocksdb
              ]
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.Security
                pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
                pkgs.libiconv
              ];

            # RocksDB build configuration
            ROCKSDB_LIB_DIR = "${pkgs.rocksdb}/lib";
            ROCKSDB_INCLUDE_DIR = "${pkgs.rocksdb}/include";
          };

          # Build only dependencies (for caching)
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          # Full build
          premath = craneLib.buildPackage (
            commonArgs
            // {
              inherit cargoArtifacts;
              # The CLI binary
              cargoExtraArgs = "--package premath-cli";
            }
          );
        in
        {
          packages = {
            default = premath;
          };

          apps.default = {
            type = "app";
            program = "${premath}/bin/premath";
          };

          checks = {
            inherit premath;

            clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            fmt = craneLib.cargoFmt { inherit src; };
          };

          devShells.default = craneLib.devShell {
            checks = self.checks.${system};

            packages = with pkgs; [
              # Rust
              rustToolchain
              cargo-watch
              cargo-nextest
              cargo-insta

              # Version control
              jujutsu

              # Database
              surrealdb

              # Tools
              direnv
              mise
              opentofu
              terraform
              ripgrep
              tokei
            ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              vfkit
            ];

            shellHook = ''
              echo "premath development shell"
              echo "  rust:     $(rustc --version)"
              echo "  jj:      $(jj --version 2>/dev/null || echo 'not found')"
              echo "  surreal: $(surreal version 2>/dev/null || echo 'not found')"
            '';
          };
        }
      );
}
