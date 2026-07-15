{
  description = "Rust development environment for beakid-rs";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
    }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      forAllSystems = nixpkgs.lib.genAttrs systems;
    in
    {
      devShells = forAllSystems (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ rust-overlay.overlays.default ];
          };

          rustToolchain = pkgs.rust-bin.stable."1.75.0".default.override {
            extensions = [
              "rust-src"
              "rust-analyzer"
              "clippy"
              "rustfmt"
            ];
          };
        in
        {
          default = pkgs.mkShell {
            packages = [
              rustToolchain
            ];

            RUST_BACKTRACE = "1";
            RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

            shellHook = ''
              echo -e "\033[1;36m╔══════════════════════════════╗\033[0m"
              echo -e "\033[1;36m║   BeakId Development Shell   ║\033[0m"
              echo -e "\033[1;36m╚══════════════════════════════╝\033[0m"
              echo ""
              echo -e "\033[32mRust:\033[0m $(rustc --version)"
              echo -e "\033[33mCargo:\033[0m $(cargo --version)"
              echo ""
            '';
          };
        }
      );
    };
}
