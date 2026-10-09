{
  description = "Lantea OS: an opinionated layer of NixOS modules, packages and themes";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs =
    { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      forAllSystems = lib.genAttrs [
        "x86_64-linux"
        "aarch64-linux"
      ];
      pkgsFor = system: nixpkgs.legacyPackages.${system};

      vmDev =
        system:
        lib.nixosSystem {
          modules = [
            self.nixosModules.default
            ./hosts/vm-dev
            { nixpkgs.hostPlatform = system; }
          ];
        };
    in
    {
      nixosModules = {
        core = ./modules/core;
        experience = ./modules/experience;
        default = {
          imports = [
            self.nixosModules.core
            self.nixosModules.experience
          ];
        };
      };

      nixosConfigurations = {
        vm-dev = vmDev "x86_64-linux";
        vm-dev-aarch64 = vmDev "aarch64-linux";
      };

      packages = forAllSystems (
        system:
        let
          lantea = (pkgsFor system).callPackage ./pkgs/lantea { };
        in
        {
          inherit lantea;
          default = lantea;
        }
      );

      checks = forAllSystems (
        system:
        import ./tests {
          pkgs = pkgsFor system;
          inherit self;
        }
      );

      formatter = forAllSystems (system: (pkgsFor system).nixfmt);

      devShells = forAllSystems (
        system:
        let
          pkgs = pkgsFor system;
        in
        {
          default = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              rustc
              clippy
              rustfmt
              rust-analyzer
              nixfmt
              statix
              deadnix
              nvd
              jq
            ];
            # cargo builds of the CLI read the edition name with env!().
            LANTEA_EDITION_NAME = (import ./modules/core/discipline/edition.nix).name;
          };
        }
      );
    };
}
