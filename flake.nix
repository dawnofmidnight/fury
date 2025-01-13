{
  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      flake-utils,
      nixpkgs,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ rust-overlay.overlays.default ];
        pkgs = import nixpkgs { inherit overlays system; };
        toolchain = pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.default.override {
          extensions = [ "rust-src" ];
        });
        rustPlatform = pkgs.makeRustPlatform {
          rustc = toolchain;
          cargo = toolchain;
        };
        fury = pkgs.callPackage ./nix/fury.nix { inherit rustPlatform; };
        vscode = pkgs.callPackage ./nix/vscode.nix {};
      in
      {
        checks.fury = fury.overrideAttrs { doCheck = true; };

        packages = {
          inherit fury vscode;
          default = fury;
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [ fury vscode ];
          packages = [ pkgs.cargo-insta toolchain ];
        };
      }
    );
}
