{ pkgs ? import <nixpkgs> {} }: let
  shell = import ./shell.nix { inherit pkgs; };
in (pkgs.buildFHSEnv {
  name = "gdl-fhs-env";

  targetPkgs = pkgs: with pkgs; [
    # electron
    alsa-lib
    atk
    at-spi2-atk
    cairo
    cups
    dbus
    expat
    glib
    gtk3
    libdrm
    libappindicator-gtk3
    libdbusmenu
    libxkbcommon
    systemd
    nss
    nspr
    mesa
    pango
    zlib

    # xorg (electron)
    xorg.libX11
    xorg.libXcomposite
    xorg.libXdamage
    xorg.libXext
    xorg.libXfixes
    xorg.libXrandr
    xorg.libxcb
  ]
  # build tools
  ++ shell.nativeBuildInputs
  # dependencies
  ++ shell.buildInputs;

  runScript = "$SHELL";
}).env
