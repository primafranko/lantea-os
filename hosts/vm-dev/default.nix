# The development VM, built as `vm-dev` (x86_64-linux) and `vm-dev-aarch64`
# (aarch64-linux). Headless: Core only, on a serial console.
{ config, modulesPath, ... }:
{
  imports = [ (modulesPath + "/virtualisation/qemu-vm.nix") ];

  networking.hostName = "vm-dev";

  virtualisation = {
    memorySize = 2048;
    cores = 2;
    graphics = false;
  };

  lantea.enable = true;

  # Development convenience, vm-dev only: log the Steward in on the console.
  services.getty.autologinUser = config.lantea.accounts.steward.name;

  # Development convenience, vm-dev only: a known password for the Keeper.
  users.users.${config.lantea.accounts.keeper.name}.initialPassword = "keeper";

  system.stateVersion = "26.05";
}
