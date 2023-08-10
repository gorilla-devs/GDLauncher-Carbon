{
  inputs.nixpkgs.url = "nixpkgs/nixos-unstable";

  outputs = { nixpkgs, ... }: let
    systems = [ "x86_64-linux" ];

    forEachSystem = system: let
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system} = {
        default = import ./nix/shell.nix { inherit pkgs; };
        fhs-env = import ./nix/fhs-env.nix { inherit pkgs; };
      };
    };
  in builtins.foldl' (a: b: nixpkgs.lib.recursiveUpdate a b) {}
    (builtins.map forEachSystem systems);
}
