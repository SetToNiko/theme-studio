{
  # Theme Studio: `nix build` (the package), `nix develop` (cargo and the
  # crate's dependencies, for studio/). Other flakes take its modules:
  # homeManagerModules.default.
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
      # `nix flake check` runs all of these (docs/refactor/GOAL.md, Verification).
      checks.${system} = {
        # The package. Building the studio also runs its tests (cargo test).
        build = theme-studio;

        # Formatting: rustfmt follows the `mod` lines from main.rs.
        fmt = pkgs.runCommand "theme-studio-fmt" { nativeBuildInputs = [ pkgs.rustfmt ]; } ''
          rustfmt --check --edition 2024 ${./studio/src}/main.rs
          touch $out
        '';

        # Clippy gate: any warning (pedantic, style, correctness; see Cargo.toml)
        # fails the check. An allowed lint needs #[expect(..., reason = "...")].
        # It reuses the package's build setup, so the crate's dependencies are
        # already there (no network needed).
        clippy = theme-studio.bin.overrideAttrs (old: {
          pname = "theme-studio-clippy";
          nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
          buildPhase = ''
            runHook preBuild
            cargo clippy --all-targets -- -D warnings
            runHook postBuild
          '';
          doCheck = false;
          installPhase = "touch $out";
        });

        # Rustdoc: the doc comments build without a warning (broken links, HTML-like
        # tags such as <leader> outside backticks). Same build setup as clippy.
        doc = theme-studio.bin.overrideAttrs (old: {
          pname = "theme-studio-doc";
          buildPhase = ''
            runHook preBuild
            RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items
            runHook postBuild
          '';
          doCheck = false;
          installPhase = "touch $out";
        });

        # Shellcheck gate: any finding in a script fails the check. A disabled
        # finding needs a comment with the reason on the line before.
        shellcheck = pkgs.runCommand "theme-studio-shellcheck" { nativeBuildInputs = [ pkgs.shellcheck ]; } ''
          shellcheck ${./scripts}/*.sh ${./scripts}/lib/*.sh ${./scripts}/dev/*.sh ${./tests/scripts}/run.sh ${./tests/install}/run.sh ${./install.sh}
          touch $out
        '';

        # Cycle ratchet: the modules in dependency cycles must not go above the
        # baseline (scripts/dev/module-cycles.py, plan step P4.14).
        cycles = pkgs.runCommand "theme-studio-cycles" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          count=$(python3 ${./scripts/dev/module-cycles.py} ${./studio/src} | sed -n '1s/.*modules in cycles: //p')
          LINT_BASELINE=${./docs/refactor/lint-baseline.txt} \
            bash ${./scripts/dev/lint-counts.sh} compare cycles "$count"
          touch $out
        '';

        # Doc gate: each function that returns Result has a `# Errors` section
        # (scripts/dev/errors-doc.sh).
        errors-doc = pkgs.runCommand "theme-studio-errors-doc" { nativeBuildInputs = [ pkgs.python3 ]; } ''
          bash ${./scripts/dev/errors-doc.sh} ${./studio/src}
          touch $out
        '';

        # File size gate: no .rs file over 500 production lines
        # (scripts/dev/file-limits.sh).
        file-limits = pkgs.runCommand "theme-studio-file-limits" { } ''
          bash ${./scripts/dev/file-limits.sh} ${./studio/src}
          touch $out
        '';

        # Golden-file tests of the switch scripts (tests/scripts/run.sh).
        # The tools are the ones that the tests link into their own PATH.
        scripts =
          pkgs.runCommand "theme-studio-script-tests"
            {
              nativeBuildInputs = [
                pkgs.bash
                pkgs.coreutils
                pkgs.jq
                pkgs.gawk
                pkgs.gnused
                pkgs.gnugrep
              ];
            }
            ''
              cp -r ${./scripts} scripts
              cp -r ${./share} share
              mkdir tests
              cp -r ${./tests/scripts} tests/scripts
              chmod -R u+w .
              bash tests/scripts/run.sh
              touch $out
            '';

        # Install test (tests/install/run.sh): install.sh into a temporary
        # PREFIX with the package's binary (THEME_STUDIO_BIN, so no cargo and no
        # network), golden tests against the installed commands, uninstall.
        install =
          pkgs.runCommand "theme-studio-install-test"
            {
              nativeBuildInputs = [
                pkgs.bash
                pkgs.coreutils
                pkgs.jq
                pkgs.gawk
                pkgs.gnused
                pkgs.gnugrep
                pkgs.findutils
                pkgs.diffutils
              ];
              THEME_STUDIO_BIN = "${theme-studio.bin}/bin/theme-studio";
            }
            ''
              mkdir -p project/docs project/packaging project/tests
              cp -r ${./scripts} project/scripts
              cp -r ${./share} project/share
              cp ${./docs/theme-studio.md} project/docs/theme-studio.md
              cp ${./packaging/theme-studio.desktop} project/packaging/theme-studio.desktop
              cp ${./install.sh} project/install.sh
              cp -r ${./tests/scripts} project/tests/scripts
              cp -r ${./tests/install} project/tests/install
              chmod -R u+w project
              bash project/tests/install/run.sh
              touch $out
            '';
      };

      # programs.theme-studio (home-manager)
      homeManagerModules.default = ./packaging/nix/home-module.nix;
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
          pkgs.python3 # scripts/dev/module-cycles.py, errors-doc.sh
        ];
        # cargo-llvm-cov needs the llvm-cov and llvm-profdata of rustc's own LLVM.
        LLVM_COV = "${pkgs.rustc.llvm}/bin/llvm-cov";
        LLVM_PROFDATA = "${pkgs.rustc.llvm}/bin/llvm-profdata";
      };
    };
}
