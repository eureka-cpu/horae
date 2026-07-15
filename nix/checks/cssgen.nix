{ perSystem, ... }:
# Fail the build if the committed utility stylesheet has drifted from the
# generator — the CSS equivalent of `sqlx prepare --check`. Reuses the package
# build environment (rust toolchain, vendored deps) but only runs the generator.
perSystem.self.default.overrideAttrs (_old: {
  pname = "horae-cssgen-check";

  buildPhase = ''
    runHook preBuild
    export HOME=$(mktemp -d)
    cargo run -p cssgen -- --check
    runHook postBuild
  '';

  installPhase = "touch $out";
  doCheck = false;
  dontFixup = true;
})
