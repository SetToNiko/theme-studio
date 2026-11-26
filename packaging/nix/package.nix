# Theme Studio's Nix package: the studio (studio/, a Rust crate) and the
# switch scripts as commands (theme-switch, themes-switch, font-switch).
# Used by flake.nix and by the home-manager module (its default `package`).
#
# $out/share/theme-studio mirrors the project (scripts/, share/, docs/), so
# the scripts find share/ beside them and the studio finds the guide; the
# commands and the studio get THEME_STUDIO_SHARE pointing there (unless it's
# already set, e.g. to a checkout for live editing).
{
  lib,
  rustPlatform,
  runCommand,
  makeWrapper,
  bash,
  coreutils,
  gnused,
  gawk,
  gnugrep,
  procps,
  jq,
  fontconfig,
  ghostty,
  i3,
}:
let
  project = ../..;
  studio = project + "/studio";

  bin = rustPlatform.buildRustPackage {
    pname = "theme-studio";
    version = "0.1.0";
    src = lib.fileset.toSource {
      root = studio;
      fileset = lib.fileset.unions [
        (studio + "/Cargo.toml")
        (studio + "/Cargo.lock")
        (studio + "/src")
        (studio + "/tests/fixtures")
      ];
    };
    cargoLock.lockFile = studio + "/Cargo.lock";
    # The data tests use studio/tests/fixtures in the sandbox. Tests that need
    # jq, ImageMagick, fontconfig, or a pty skip themselves there.
    meta.mainProgram = "theme-studio";
  };

  files = lib.fileset.toSource {
    root = project;
    fileset = lib.fileset.unions [
      (project + "/scripts")
      (project + "/share")
      (project + "/docs/theme-studio.md")
    ];
  };

  # What the scripts run. The basics come first; the desktop's own programs
  # (ghostty, i3-msg) only when the system has none, so its versions
  # and configs win.
  basics = [
    coreutils
    gnused
    gawk
    gnugrep
    procps
    jq
    fontconfig.bin
  ];
  desktop = [
    ghostty
    i3
  ];
in
runCommand "theme-studio-0.1.0"
  {
    nativeBuildInputs = [ makeWrapper ];
    meta.mainProgram = "theme-studio";
    passthru = { inherit bin; };
  }
  ''
    share=$out/share/theme-studio
    mkdir -p $out/bin $share
    cp -r ${files}/scripts ${files}/share ${files}/docs $share/

    for name in theme-switch themes-switch font-switch; do
      makeWrapper ${bash}/bin/bash $out/bin/$name \
        --add-flags "\''${THEME_STUDIO_SCRIPTS:-$share/scripts}/$name.sh" \
        --set-default THEME_STUDIO_SHARE $share/share \
        --prefix PATH : ${lib.makeBinPath basics} \
        --suffix PATH : ${lib.makeBinPath desktop}
    done

    makeWrapper ${bin}/bin/theme-studio $out/bin/theme-studio \
      --set-default THEME_STUDIO_SHARE $share/share \
      --prefix PATH : $out/bin
  ''
