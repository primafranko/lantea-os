# One VM test per Strand, exposed as `checks.<system>.<name>`.
{ pkgs, self }:
{
  discipline = pkgs.testers.runNixOSTest (import ./discipline.nix { inherit self; });
}
