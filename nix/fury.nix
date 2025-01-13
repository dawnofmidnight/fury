{ lib, rustPlatform }:
let
  manifest = builtins.fromTOML (builtins.readFile ../Cargo.toml);
in rustPlatform.buildRustPackage (finalAttrs: {
  pname = manifest.package.name;
  version = manifest.package.version;

  src = lib.cleanSource ./..;
  cargoLock.lockFile = ../Cargo.lock;

  checkPhase = ''
    cargo fmt --check
    cargo clippy -- --deny warnings
    runHook cargoCheckHook
  '';
})
