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
      # cargo, rustc and the crate's dependencies, for working in studio/,
      # plus the refactor's check tools (docs/refactor/PRINCIPLES.md).
      # clippy and rustfmt come from the same nixpkgs as rustc: a system
      # clippy built for another rustc fails with error E0514.
      devShells.${system}.default = pkgs.mkShell {
        inputsFrom = [ theme-studio.bin ];
        packages = [
          pkgs.clippy
          pkgs.rustfmt
          pkgs.shellcheck
          pkgs.cargo-modules # dependency cycles between modules
          pkgs.cargo-llvm-cov # test coverage
          pkgs.python3Packages.lizard # function length, complexity, duplicates
          pkgs.python3 # scripts/dev/module-cycles.py
        ];
        # cargo-llvm-cov needs the llvm-cov and llvm-profdata of rustc's own LLVM.
        LLVM_COV = "${pkgs.rustc.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.rustc.llvm}/bin/llvm-profdata";
      };
    };
}
