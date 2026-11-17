{
  # Theme Studio: `nix build` (the package), `nix develop` (cargo and the
  # crate's dependencies, for studio/). Other flakes take its modules:
  # homeManagerModules.default, nixosModules.default.
  description = "Theme Studio: colour palettes and fonts with a live preview of the desktop";

  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

  outputs =
    { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      theme-studio = pkgs.callPackage ./packaging/nix/package.nix { };
    in
    {
      packages.${system} = {
        inherit theme-studio;
        default = theme-studio;
      };
      # programs.theme-studio (home-manager) and services.theme-studio.loginFont
      homeManagerModules.default = ./packaging/nix/home-module.nix;
      nixosModules.default = ./packaging/nix/nixos-module.nix;
      # cargo, rustc and the crate's dependencies, for working in studio/
      devShells.${system}.default = pkgs.mkShell { inputsFrom = [ theme-studio.bin ]; };
    };
}
