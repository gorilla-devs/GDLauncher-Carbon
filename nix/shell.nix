{ pkgs ? import <nixpkgs> {} }: pkgs.mkShell {
  name = "gdl-shell";

  nativeBuildInputs = with pkgs; [
    nodejs-18_x
    nodePackages.pnpm
    rustc
    cargo
    rustfmt

    pkg-config
    gcc
  ];

  buildInputs = with pkgs; [
    openssl
    openssl.dev
  ];
}
