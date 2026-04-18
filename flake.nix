{
  description = "Premath: kernel doctrine of definability for agent orchestration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    tusk.url = "git+file:///Users/arj/irai/tusk?ref=main";
    devenv.follows = "tusk/devenv";
    llm-agents.follows = "tusk/llm-agents";
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
      tusk,
      devenv,
      llm-agents,
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
                pkgs.apple-sdk
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

          devShells.default = tusk.lib.mkRepoShell {
            inherit system;
            pkgs = pkgs;
            flakeInputs = {
              inherit
                self
                nixpkgs
                flake-utils
                tusk
                devenv
                llm-agents
                rust-overlay
                crane
                jj
                ;
            };
            modules = [
              (
                { pkgs, ... }:
                {
                  codex.skills.tusk.source = tusk + "/.agents/skills/tusk";
                  codex.skills.nix.source = tusk + "/.agents/skills/nix";
                  codex.skills.ops.source = tusk + "/.agents/skills/ops";
                  codex.skills.topology.source = tusk + "/.agents/skills/topology";

                  claude.skills.tusk.source = tusk + "/.agents/skills/tusk";
                  claude.skills.nix.source = tusk + "/.agents/skills/nix";
                  claude.skills.ops.source = tusk + "/.agents/skills/ops";
                  claude.skills.topology.source = tusk + "/.agents/skills/topology";

                  tusk.consumer = {
                    enable = true;
                    # Premath still uses an embedded tracker backend today, so
                    # do not auto-start tuskd/beads-dolt from the shared shell.
                    beadsDolt.enable = false;
                    extraPackages = with pkgs; [
                      rustToolchain
                      cargo-watch
                      cargo-nextest
                      cargo-insta
                      mise
                      opentofu
                      tokei
                      python3
                      pkg-config
                      cmake
                      openssl
                      rocksdb
                    ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                      pkgs.apple-sdk
                      pkgs.libiconv
                      vfkit
                    ];
                    smokeCheck.skillChecks = [
                      ".codex/skills/tusk"
                      ".codex/skills/nix"
                      ".codex/skills/ops"
                      ".codex/skills/topology"
                      ".claude/skills/tusk"
                      ".claude/skills/nix"
                      ".claude/skills/ops"
                      ".claude/skills/topology"
                    ];
                    extraEnterShell = ''
                      export ROCKSDB_LIB_DIR="${pkgs.rocksdb}/lib"
                      export ROCKSDB_INCLUDE_DIR="${pkgs.rocksdb}/include"
                      echo "premath tusk consumer shell"
                      echo "  rust:     $(rustc --version)"
                      echo "  jj:       $(jj --version 2>/dev/null || echo 'not found')"
                      echo "  surreal:  $(surreal version 2>/dev/null || echo 'not found')"
                      echo "  cargo build --workspace"
                      echo "  mise run baseline"
                    '';
                  };
                }
              )
            ];
          };
        }
      );
}
