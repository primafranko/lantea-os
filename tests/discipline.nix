# The discipline Strand: os-release, the Steward and Keeper accounts,
# elevation policy and the `lantea` CLI skeleton, on a headless Core node.
{ self }:
{
  name = "discipline";

  nodes.machine = {
    imports = [ self.nixosModules.core ];
    lantea.enable = true;
  };

  testScript =
    { nodes, ... }:
    let
      inherit (nodes.machine.system.nixos) release codeName version;
    in
    ''
      import re

      machine.wait_for_unit("multi-user.target")

      with subtest("os-release names the Lantea edition and keeps the NixOS base visible"):
          os_release = machine.succeed("cat /etc/os-release").splitlines()
          for line in [
              "ID=lantea",
              "ID_LIKE=nixos",
              'NAME="Lantea OS"',
              'VERSION="1.0 (Harbour)"',
              'VERSION_ID="1.0"',
              "VERSION_CODENAME=harbour",
              'PRETTY_NAME="Lantea OS 1.0 (Harbour)"',
              'BUILD_ID="${version}"',
              'LANTEA_BASE="NixOS ${release} (${codeName})"',
              'CPE_NAME="cpe:/o:nixos:nixos:${release}"',
              'HOME_URL="https://github.com/primafranko/lantea-os"',
              'BUG_REPORT_URL="https://github.com/primafranko/lantea-os/issues"',
              'VENDOR_NAME=""',
              'SUPPORT_URL=""',
              'LOGO="nix-snowflake"',
          ]:
              assert line in os_release, (line, os_release)
          for key in [
              "VERSION", "VERSION_ID", "VERSION_CODENAME", "PRETTY_NAME", "BUILD_ID",
              "CPE_NAME", "HOME_URL", "BUG_REPORT_URL", "VENDOR_NAME", "SUPPORT_URL",
              "SUPPORT_END", "LOGO",
          ]:
              assert len([l for l in os_release if l.startswith(key + "=")]) == 1, key
          # SUPPORT_END is the NixOS base's end of security support, a date.
          assert any(
              re.fullmatch(r'SUPPORT_END="?\d{4}-\d{2}-\d{2}"?', l) for l in os_release
          ), os_release

      with subtest("nixos-version still reports the NixOS release"):
          assert machine.succeed("nixos-version") == "${version} (${codeName})\n"

      with subtest("the Steward is unprivileged and the Keeper is in wheel"):
          steward_groups = machine.succeed("id -nG steward").split()
          assert "wheel" not in steward_groups, steward_groups
          keeper_groups = machine.succeed("id -nG keeper").split()
          assert "wheel" in keeper_groups, keeper_groups

      with subtest("sudo is not installed"):
          machine.fail("command -v sudo")
          machine.fail("su - steward -c 'command -v sudo'")
          machine.fail("su - keeper -c 'command -v sudo'")

      with subtest("polkit's admin identity is the wheel group"):
          machine.succeed(
              "grep -F 'return [\"unix-group:wheel\"];' /etc/polkit-1/rules.d/10-nixos.rules"
          )

      with subtest("the Steward cannot elevate with run0"):
          status, out = machine.execute(
              "su - steward -c 'run0 --no-ask-password true' 2>&1"
          )
          assert status != 0, out
          assert status != 127 and "not found" not in out, out
          assert re.search(
              r"(?i)authenticat|access denied|not authori[sz]ed|permission denied", out
          ), out

      with subtest("lantea is on every user's PATH"):
          machine.succeed("command -v lantea")
          machine.succeed("su - steward -c 'command -v lantea'")
          machine.succeed("su - keeper -c 'command -v lantea'")

      def lantea(args):
          status, out = machine.execute(
              f"su - steward -c 'lantea {args}' 2>/tmp/lantea-stderr"
          )
          err = machine.succeed("cat /tmp/lantea-stderr")
          return status, out, err

      def expect(args, code, stdout):
          status, out, err = lantea(args)
          assert status == code, f"lantea {args}: exit {status}, stderr {err!r}"
          assert out == stdout, f"lantea {args}: stdout {out!r}"
          return err

      with subtest("lantea --version names the edition"):
          expect("--version", 0, "lantea 0.1.0 (Harbour)\n")

      with subtest("a bare lantea prints usage to stderr and exits 2"):
          err = expect("", 2, "")
          assert "Usage:" in err, err

      with subtest("lantea condition, plain, --json and --explain"):
          expect("condition", 0, "Condition is not yet known.\n")
          expect("condition --json", 0, '{"state":"unknown"}\n')
          expect("--json condition", 0, '{"state":"unknown"}\n')
          explain = "No commands are run yet: the condition daemon arrives in Phase 3.\n"
          expect("condition --explain", 0, explain)
          expect("--explain condition", 0, explain)
          expect("--yes condition", 0, "Condition is not yet known.\n")
          expect("condition --yes", 0, "Condition is not yet known.\n")

      with subtest("an unknown subcommand is a usage error"):
          expect("no-such-command", 2, "")
    '';
}
