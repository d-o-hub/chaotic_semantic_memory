{
  description = "A Nix-flake-based Rust development environment for chaotic_semantic_memory";

  inputs = {
    nixpkgs.url = "https://flakehub.com/f/NixOS/nixpkgs/0.1";
    fenix = {
      url = "https://flakehub.com/f/nix-community/fenix/0.1";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, ... }@inputs:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];
      forEachSupportedSystem =
        f:
        inputs.nixpkgs.lib.genAttrs supportedSystems (
          system:
          f {
            pkgs = import inputs.nixpkgs {
              inherit system;
              overlays = [ inputs.self.overlays.default ];
            };
          }
        );
    in
    {
      overlays.default = final: prev: {
        # Mirrors rust-toolchain.toml: stable 1.88.0 with clippy, rustfmt, rust-src, wasm32 target.
        rustToolchain =
          with inputs.fenix.packages.${prev.stdenv.hostPlatform.system};
          combine (
            [
              (stable.withComponents [
                "clippy"
                "rustc"
                "cargo"
                "rustfmt"
                "rust-src"
              ])
              targets.wasm32-unknown-unknown.stable.rust-std
            ]
          );
      };

      devShells = forEachSupportedSystem (
        { pkgs }:
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              # Rust toolchain (matches rust-toolchain.toml)
              rustToolchain
              rust-analyzer

              # WASM / npm toolchain
              wasm-pack
              nodejs_20

              # Build essentials
              openssl
              pkg-config
              clang
              mold

              # Cargo tools - mirrors CI pipeline jobs
              cargo-deny       # supply chain / license checks (deny.toml)
              cargo-mutants    # mutation testing (scripts/mutation_test.sh)
              cargo-fuzz       # fuzz testing (fuzz/)
              cargo-audit      # security audit
              cargo-binstall   # fast binary installs
              cargo-llms-txt   # llms.txt generation (scripts/gen-llms-txt.sh)

              # Quality and documentation
              shellcheck       # shell script hygiene
              mdbook           # documentation (book/)
            ];

            env = {
              # Required by rust-analyzer and proc-macro expansion
              RUST_SRC_PATH = "${pkgs.rustToolchain}/lib/rustlib/src/rust/library";
            };
          };
        }
      );
    };
}
