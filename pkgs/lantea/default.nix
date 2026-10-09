{ lib, rustPlatform }:
let
  edition = import ../../modules/core/discipline/edition.nix;
in
rustPlatform.buildRustPackage {
  pname = "lantea";
  inherit ((lib.importTOML ./Cargo.toml).workspace.package) version;

  src = lib.fileset.toSource {
    root = ./.;
    fileset = lib.fileset.unions [
      ./Cargo.toml
      ./Cargo.lock
      ./cli
    ];
  };

  cargoLock.lockFile = ./Cargo.lock;

  # Read at compile time by `env!("LANTEA_EDITION_NAME")` in the CLI.
  env.LANTEA_EDITION_NAME = edition.name;

  meta = {
    description = "Plain-language access to a Lantea OS system";
    mainProgram = "lantea";
    platforms = lib.platforms.linux;
  };
}
