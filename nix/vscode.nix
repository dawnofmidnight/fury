{
  buildNpmPackage,
  lib,
  libsecret,
  pkg-config,
}:
buildNpmPackage {
  inherit (builtins.fromJSON (builtins.readFile ../editors/vscode/package.json)) version;
  pname = "vscode-fury";

  src = lib.cleanSource ../editors/vscode;
  npmDepsHash = "sha256-SaaUz1ZrtAD4Q2MTGSWt1jFouPoFz9BZDLlXpVCSoDA=";

  installPhase = ''
    mkdir -p $out/bin
    npm run package $out/bin/$pname.vsix
  '';

  nativeBuildInputs = [ pkg-config ];
  buildInputs = [ libsecret ];
}
