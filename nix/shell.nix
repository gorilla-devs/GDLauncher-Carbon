{ pkgs ? import <nixpkgs> {}, nodejs ? null, pnpm ? null }:
let
  manifest = builtins.fromJSON (builtins.readFile ../package.json);
  nodePin = builtins.replaceStrings [ "\n" "\r" ] [ "" "" ] (builtins.readFile ../.nvmrc);
  minimumVersion = name: range:
    let matched = builtins.match ">=([0-9]+\\.[0-9]+\\.[0-9]+)" range;
    in if matched == null then
      throw "gdl-shell: review the unsupported ${name} engine range ${range} in package.json."
    else builtins.head matched;
  nodeMinimum = minimumVersion "Node" manifest.engines.node;
  pnpmMinimum = minimumVersion "pnpm" manifest.engines.pnpm;
  pnpmMajor = pkgs.lib.versions.major (pkgs.lib.removePrefix "pnpm@" manifest.packageManager);

  selectedNode = if nodejs != null then nodejs
    else if builtins.hasAttr "nodejs_24" pkgs then pkgs.nodejs_24
    else throw "gdl-shell: nixpkgs has no nodejs_24. Supply a genuine nodejs derivation >= ${nodePin}, or use a suitable nixpkgs package set.";
  selectedPnpm = if pnpm != null then pnpm
    else if builtins.hasAttr "pnpm_${pnpmMajor}" pkgs then builtins.getAttr "pnpm_${pnpmMajor}" pkgs
    else throw "gdl-shell: nixpkgs has no pnpm_${pnpmMajor}. Supply a compatible pnpm derivation >= ${pnpmMinimum} with nodejs and withNode override arguments.";

  # callPackage's override preserves the original function arguments. Refuse
  # packages without this API rather than leave pnpm bound to another Node.
  pnpmArguments = pkgs.lib.functionArgs selectedPnpm.override;
  pnpmForNode = selectedPnpm.override { nodejs = selectedNode; withNode = true; };
in
if !pkgs.lib.versionAtLeast nodePin nodeMinimum then
  throw "gdl-shell: .nvmrc (${nodePin}) is below the enforced Node minimum ${nodeMinimum}; align the contributor pin."
else if !pkgs.lib.isDerivation selectedNode || !(selectedNode ? version) then
  throw "gdl-shell: nodejs must be a genuine Node derivation with its actual version; do not relabel an older runtime."
else if !pkgs.lib.versionAtLeast selectedNode.version nodePin then
  throw "gdl-shell: Node ${selectedNode.version} is below .nvmrc ${nodePin}. Supply a newer nodejs derivation or a suitable nixpkgs package set."
else if !pkgs.lib.isDerivation selectedPnpm || !(selectedPnpm ? version) then
  throw "gdl-shell: pnpm must be a genuine derivation with its actual version."
else if !pkgs.lib.versionAtLeast selectedPnpm.version pnpmMinimum
  || pkgs.lib.versions.major selectedPnpm.version != pnpmMajor then
  throw "gdl-shell: pnpm ${selectedPnpm.version} does not match packageManager ${manifest.packageManager} and minimum ${pnpmMinimum}; supply a compatible pnpm_${pnpmMajor} derivation."
else if !(selectedPnpm ? override) || !pkgs.lib.isFunction selectedPnpm.override
  || !(pnpmArguments ? nodejs) || !(pnpmArguments ? withNode) then
  throw "gdl-shell: pnpm must support the nodejs and withNode package override arguments so it uses the selected Node runtime."
else if !pkgs.lib.isDerivation pnpmForNode then
  throw "gdl-shell: the pnpm Node override must return a genuine derivation."
else pkgs.mkShell {
  name = "gdl-shell";

  nativeBuildInputs = with pkgs; [
    selectedNode
    pnpmForNode
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
