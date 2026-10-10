{
  lib,
  rustPlatform,
  util-linux,
}:
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

  # The tests redirect the Record to their own socket (decision 0017). Only the
  # test phase enables the feature; the installed binary is built without it.
  cargoTestFlags = [
    "--features"
    "test-journal-socket"
  ];

  # `unshare` and `mount`: the mount tests run in a user namespace.
  nativeCheckInputs = [ util-linux ];

  # Read at compile time by `env!("LANTEA_EDITION_NAME")` in the CLI.
  env.LANTEA_EDITION_NAME = edition.name;

  meta = {
    description = "Plain-language access to a Lantea OS system";
    mainProgram = "lantea";
    platforms = lib.platforms.linux;
  };
}
