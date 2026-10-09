# The discipline Strand: the `lantea.*` option namespace, os-release,
# the Steward and Keeper accounts, elevation defaults and the `lantea` CLI.
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.lantea;
in
{
  options.lantea = {
    enable = lib.mkOption {
      type = lib.types.bool;
      default = false;
      description = "Whether this system follows the Lantea Discipline: Lantea's os-release, the Steward and Keeper accounts, and the `lantea` command.";
    };

    track = lib.mkOption {
      type = lib.types.enum [
        "standing"
        "rising"
      ];
      default = "standing";
      description = "Which track this system follows: `standing` is the stable track (the NixOS stable channel), `rising` the testing track (nixos-unstable).";
    };

    edition = lib.mkOption {
      type = lib.types.attrsOf lib.types.str;
      default = import ./edition.nix;
      readOnly = true;
      description = "The Lantea edition this module set belongs to: its version number and its name.";
    };

    accounts.steward.name = lib.mkOption {
      type = lib.types.str;
      default = "steward";
      description = "User name of the Steward, the unprivileged daily-use account.";
    };

    accounts.keeper.name = lib.mkOption {
      type = lib.types.str;
      default = "keeper";
      description = "User name of the Keeper, the administrative account. The Keeper is in the `wheel` group and elevates with run0.";
    };
  };

  config = lib.mkIf cfg.enable {
    # os-release: ID=lantea, NAME="Lantea OS"; NixOS adds ID_LIKE=nixos
    # itself whenever the distribution ID is not "nixos". The version keys
    # name the Lantea edition; BUILD_ID stays NixOS's, and LANTEA_BASE names
    # the NixOS version underneath (decision 0016). nixos-version is unaffected.
    # SUPPORT_END is left as NixOS sets it: the base's end of security support.
    # LOGO stays NixOS's until Lantea's visual identity arrives in Phase 7.
    system.nixos = {
      distroId = lib.mkDefault "lantea";
      distroName = lib.mkDefault "Lantea OS";
      extraOSReleaseArgs = lib.mapAttrs (_: lib.mkDefault) {
        VERSION = "${cfg.edition.version} (${cfg.edition.name})";
        VERSION_ID = cfg.edition.version;
        VERSION_CODENAME = lib.toLower cfg.edition.name;
        PRETTY_NAME = "${config.system.nixos.distroName} ${cfg.edition.version} (${cfg.edition.name})";
        LANTEA_BASE = "NixOS ${config.system.nixos.release} (${config.system.nixos.codeName})";
        # Vulnerability scanners match on CPE_NAME, and the packages are NixOS's.
        CPE_NAME = "cpe:/o:nixos:nixos:${config.system.nixos.release}";
        HOME_URL = "https://github.com/primafranko/lantea-os";
        BUG_REPORT_URL = "https://github.com/primafranko/lantea-os/issues";
        # An empty value is how NixOS's os-release marks a key as not set.
        VENDOR_NAME = "";
        SUPPORT_URL = "";
      };
    };

    users.users = {
      ${cfg.accounts.steward.name} = {
        isNormalUser = true;
        description = "Steward";
      };

      ${cfg.accounts.keeper.name} = {
        isNormalUser = true;
        description = "Keeper";
        extraGroups = [ "wheel" ];
      };
    };

    # Elevation is run0 (decision 0008). The real sudo is not installed; the
    # explaining sudo shim arrives in Phase 6.
    security = {
      sudo.enable = lib.mkDefault false;
      polkit = {
        enable = lib.mkDefault true;
        adminIdentities = lib.mkDefault [ "unix-group:wheel" ];
      };
    };

    environment.systemPackages = [ (pkgs.callPackage ../../../pkgs/lantea { }) ];
  };
}
